# Ultra STT on macOS

Build on the Mac with the existing Node, Rust and Xcode toolchains:

```sh
sh scripts/build-macos.sh --install
```

Without `--install`, the script only builds and signs the bundle. Installation backs up the previous app under `~/Library/Application Support/com.ultrastt.personal/app-backups`, closes the installed Ultra STT process, installs to `/Applications/Ultra_STT.app`, verifies the signature and launches it. Settings, credentials and history stay in their existing locations.

The personal build uses the stable identifier and designated requirement `com.ultrastt.personal`. Keep this identity and the `/Applications` location across rebuilds so macOS can recognize the application that received permissions. This local ad-hoc signature is for the personal installation, not a notarized public release.

Allow **Ultra STT** in **System Settings → Privacy & Security → Microphone** and **Accessibility**. On a Korean Mac these are **시스템 설정 → 개인정보 보호 및 보안 → 마이크 / 손쉬운 사용**. The microphone prompt appears on the first recording. A denied or restricted grant stops recording with an explanation. The app's Accessibility banner opens the permission settings; the user must enable the switch. Relaunch the installed app if macOS does not immediately recognize an updated grant.

Keyboard output and automatic paste require Accessibility. Automatic paste sends Command+V from the app itself, avoiding an additional System Events Automation permission. With missing Accessibility permission the transcript is retained in history and copied, with a visible warning; granting permission restores the configured cursor insertion without changing the output preference.

For the Windows-hosted recognizer, keep the existing `custom-whisper`/`ultra-remote` configuration, tailnet endpoint and credential. Do not print or copy the token into logs. Test speech capture, recognition and cursor insertion separately before testing the entire flow:

1. Start the installed app with log capture when investigating a problem:
   ```sh
   open -g -a /Applications/Ultra_STT.app --stdout /tmp/ultra-stt-session.log --stderr /tmp/ultra-stt-session.err
   ```
   The startup log records microphone and Accessibility permission status without credentials. Logging options apply only when launching a new process.
   Add `--env ULTRA_STT_DIAGNOSTICS=1` before launching to inspect the capsule's native window and computed canvas backgrounds. This saves a snapshot of the app's own capsule to `/tmp/ultra-stt-capsule.tiff`; it does not capture the desktop or require Screen Recording permission.
2. Use Command+Control+R to record a spoken sentence. Audio `peak` and `rms` must be nonzero when speech is present. An empty transcript in a quiet room is expected and is not proof of a permission failure.
3. Focus a new text document, record again and confirm the recognized sentence appears at its cursor. Inspect insertion status and any warning.
4. Confirm the capsule has a transparent canvas in both light and dark modes, while the main window retains its normal background.

The configured microphone is the system default input. If permission is authorized but speech still produces no signal, verify the selected microphone and input volume in System Settings → Sound → Input.

Keychain operations run without authorization dialogs and restore the previous interaction setting afterward. An inaccessible or locked Keychain fails promptly instead of freezing startup or recording. Automatic migration removes legacy keys only after a verified vault write; if the vault refuses access, the existing fallback retains the configured key and logs a warning. This does not grant additional Keychain access or change macOS permissions.

The `ultra-remote` preset skips a second local LLM call, not text insertion. The server-polished result must still go through the configured insertion strategy and target guard, and history must record the actual insertion outcome.

The floating capsule uses a native borderless window with no shadow or focus ring, and its idle, active and error shells have no CSS border, outline or shadow. The surrounding canvas is transparent; the green capsule itself remains visible.

On macOS, the insertion guard reads the current application identity directly from `NSWorkspace` at recording start and immediately before output. It does not rely on a stale context profile or System Events Automation permission. Actual application changes still trigger a visible clipboard fallback instead of typing into a different application. Optional browser context retains its separate existing permission checks.
