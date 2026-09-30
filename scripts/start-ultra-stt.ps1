# Launch only the project-owned personal profile; never modify account-wide settings.
$ErrorActionPreference = 'Stop'
$app = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$root = (Resolve-Path (Join-Path $app '..')).Path
$exe = Join-Path $app 'src-tauri/target/release/Ultra_STT.exe'
$codexPackage = Join-Path $env:APPDATA 'npm/node_modules/@openai/codex'
$codexExe = Join-Path $codexPackage 'node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe'
if (!(Test-Path -LiteralPath $exe -PathType Leaf)) { throw "Build the native app first: $exe" }
if (!(Test-Path -LiteralPath $codexExe -PathType Leaf)) { throw "Native Codex unavailable: $codexExe" }
$env:ULTRA_STT_ROOT = $root
$env:ULTRA_STT_DATA_DIR = Join-Path $root 'data'
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $root 'webview'
$env:TEMP = Join-Path $root 'temp'
$env:TMP = $env:TEMP
$env:CUDA_CACHE_PATH = Join-Path $root 'cache/cuda'
$env:HF_HUB_OFFLINE = '1'
$env:ULTRA_STT_CODEX_EXE = $codexExe
$env:CODEX_MANAGED_PACKAGE_ROOT = $codexPackage
foreach ($dir in @($env:ULTRA_STT_DATA_DIR, $env:WEBVIEW2_USER_DATA_FOLDER, $env:TEMP, $env:CUDA_CACHE_PATH)) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
}
& $exe @args
exit $LASTEXITCODE
