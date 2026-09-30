#!/bin/sh
set -eu

export PATH="/opt/homebrew/bin:$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:$PATH"
project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_dir"

if [ "$(uname -s)" != Darwin ]; then
    printf '%s\n' 'This script must run on macOS.' >&2
    exit 1
fi
if [ "$#" -gt 1 ] || { [ "$#" -eq 1 ] && [ "$1" != --install ]; }; then
    printf '%s\n' 'Usage: sh scripts/build-macos.sh [--install]' >&2
    exit 1
fi

npx tauri build --bundles app
bundle="$project_dir/src-tauri/target/release/bundle/macos/Ultra_STT.app"
identifier=$(/usr/libexec/PlistBuddy -c 'Print CFBundleIdentifier' "$bundle/Contents/Info.plist")
if [ "$identifier" != com.ultrastt.personal ]; then
    printf '%s\n' "Unexpected application identifier: $identifier" >&2
    exit 1
fi
codesign --force --deep --sign - --identifier "$identifier" \
    --requirements '=designated => identifier "com.ultrastt.personal"' "$bundle"
codesign --verify --deep --strict "$bundle"
codesign -d -r- "$bundle" 2>&1

if [ "${1:-}" != --install ]; then
    printf '%s\n' "Built and signed: $bundle"
    exit 0
fi

installed=/Applications/Ultra_STT.app
if [ -d "$installed" ]; then
    backup_dir="$HOME/Library/Application Support/com.ultrastt.personal/app-backups"
    mkdir -p "$backup_dir"
    backup="$backup_dir/Ultra_STT-$(date +%Y%m%d-%H%M%S).app"
    if [ -e "$backup" ]; then
        printf '%s\n' "Backup already exists: $backup" >&2
        exit 1
    fi
    ditto "$installed" "$backup"
    printf '%s\n' "Previous app backed up: $backup"
fi

running_pids=$(pgrep -f '^/Applications/Ultra_STT[.]app/Contents/MacOS/Ultra_STT($| )' || true)
if [ -n "$running_pids" ]; then
    for process_id in $running_pids; do
        kill -TERM "$process_id"
    done
    attempts=0
    while pgrep -f '^/Applications/Ultra_STT[.]app/Contents/MacOS/Ultra_STT($| )' >/dev/null; do
        attempts=$((attempts + 1))
        if [ "$attempts" -ge 10 ]; then
            printf '%s\n' 'Ultra STT did not exit. Installation stopped.' >&2
            exit 1
        fi
        sleep 1
    done
fi

ditto "$bundle" "$installed"
codesign --verify --deep --strict "$installed"
if [ "${ULTRA_STT_DIAGNOSTICS:-0}" = 1 ]; then
    open -n -g "$installed" --env ULTRA_STT_DIAGNOSTICS=1 \
        --stdout /tmp/ultra-stt-session.log --stderr /tmp/ultra-stt-session.err
else
    open -n -g "$installed"
fi
attempts=0
while ! pgrep -f '^/Applications/Ultra_STT[.]app/Contents/MacOS/Ultra_STT($| )' >/dev/null; do
    attempts=$((attempts + 1))
    if [ "$attempts" -ge 10 ]; then
        printf '%s\n' 'Installed Ultra STT did not start.' >&2
        exit 1
    fi
    sleep 1
done
printf '%s\n' "Installed and launched: $installed"
