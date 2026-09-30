param([switch]$Install)

$ErrorActionPreference = 'Stop'
function Get-ApplicationPayloadHash([string]$Path) {
    $encoding = [Text.Encoding]::GetEncoding(28591)
    $binary = $encoding.GetString([IO.File]::ReadAllBytes($Path)).Replace('__TAURI_BUNDLE_TYPE_VAR_UNK', '__TAURI_BUNDLE_TYPE_VAR_NSS')
    $digest = [Security.Cryptography.SHA256]::Create()
    try {
        return [BitConverter]::ToString($digest.ComputeHash($encoding.GetBytes($binary))).Replace('-', '')
    } finally {
        $digest.Dispose()
    }
}
$app = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$root = (Resolve-Path (Join-Path $app '..')).Path
$release = Join-Path $app 'src-tauri/target/release'
$builtExecutable = Join-Path $release 'Ultra_STT.exe'
$installedDirectory = Join-Path $env:LOCALAPPDATA 'Ultra_STT'
$installedExecutable = Join-Path $installedDirectory 'Ultra_STT.exe'
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$visualStudio = & $vswhere -latest -products '*' -property installationPath
if ($LASTEXITCODE -ne 0 -or !$visualStudio) { throw 'Visual Studio Build Tools were not found.' }
$cmakeDirectory = Join-Path $visualStudio 'Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin'
if (!(Test-Path -LiteralPath (Join-Path $cmakeDirectory 'cmake.exe'))) { throw 'The existing Visual Studio CMake executable was not found.' }
$env:PATH = "$cmakeDirectory;$env:PATH"
$env:CARGO_HOME = Join-Path $root 'cache/cargo'
$env:CARGO_BUILD_JOBS = '4'
Push-Location $app
try {
    npm run build
    if ($LASTEXITCODE -ne 0) { throw 'Frontend build failed.' }
    cargo build --locked --offline --release --lib --manifest-path src-tauri/Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw 'Native library build failed.' }

    $backupDirectory = Join-Path $root ('runtime/app-backups/windows-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
    New-Item -ItemType Directory -Path $backupDirectory | Out-Null
    foreach ($candidate in @($builtExecutable, $installedExecutable)) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            $name = if ($candidate -eq $builtExecutable) { 'project-Ultra_STT.exe' } else { 'installed-Ultra_STT.exe' }
            Copy-Item -LiteralPath $candidate -Destination (Join-Path $backupDirectory $name)
        }
    }
    $running = @(Get-CimInstance Win32_Process -Filter "Name='Ultra_STT.exe'" | Where-Object {
        $_.ExecutablePath -in @($builtExecutable, $installedExecutable)
    })
    foreach ($process in $running) {
        Stop-Process -Id $process.ProcessId
        Wait-Process -Id $process.ProcessId -Timeout 15 -ErrorAction SilentlyContinue
    }
    $buildSucceeded = $false
    try {
        $profileMarker = Join-Path $app 'src-tauri/resources/server-root.txt'
        $profileMarkerTemplate = [IO.File]::ReadAllText($profileMarker)
        $requiredRuntimeFiles = @('.venv/Scripts/python.exe', 'models/large-v3/model.bin', 'opentypeless/local_stt/server.py', 'data/settings.json')
        $personalRuntimeReady = @($requiredRuntimeFiles | Where-Object { !(Test-Path -LiteralPath (Join-Path $root $_) -PathType Leaf) }).Count -eq 0
        try {
            if ($personalRuntimeReady) { [IO.File]::WriteAllText($profileMarker, ($root -replace '\\', '/'), [Text.Encoding]::ASCII) }
            npx tauri build --bundles nsis
            if ($LASTEXITCODE -ne 0) { throw 'Windows application build failed.' }
            $buildSucceeded = $true
        } finally {
            [IO.File]::WriteAllText($profileMarker, $profileMarkerTemplate, [Text.Encoding]::ASCII)
        }
    } finally {
        if (!$buildSucceeded -and $running.Count -gt 0) {
            Copy-Item -LiteralPath (Join-Path $backupDirectory 'project-Ultra_STT.exe') -Destination $builtExecutable
            Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile', '-File', "`"$(Join-Path $PSScriptRoot 'start-ultra-stt.ps1')`"") -WindowStyle Hidden
        }
    }
    $version = (Get-Content -LiteralPath (Join-Path $app 'package.json') -Raw | ConvertFrom-Json).version
    $installer = Join-Path $release "bundle/nsis/Ultra_STT_${version}_x64-setup.exe"
    if (!(Test-Path -LiteralPath $installer -PathType Leaf)) { throw 'The expected Windows installer was not generated.' }
    Write-Output "INSTALLER=$installer"
    Write-Output "BACKUP=$backupDirectory"
    if ($Install) {
        $setup = Start-Process -FilePath $installer -ArgumentList '/S' -WindowStyle Hidden -PassThru -Wait
        if ($setup.ExitCode -ne 0) { throw "Installer failed: $($setup.ExitCode)" }
        $builtHash = Get-ApplicationPayloadHash $builtExecutable
        if ((Get-ApplicationPayloadHash $installedExecutable) -ne $builtHash) { throw 'Installed binary does not match the validated build payload.' }
        $expectedRuntimeMarker = if ($personalRuntimeReady) { $root -replace '\\', '/' } else { '@local-runtime@' }
        if ((Get-Content -LiteralPath (Join-Path $installedDirectory 'server-root.txt') -Raw).Trim() -ne $expectedRuntimeMarker) { throw 'Installed runtime location does not match the build profile.' }
        $menuShortcutPath = Join-Path $env:APPDATA 'Microsoft/Windows/Start Menu/Programs/Ultra_STT.lnk'
        if (Test-Path -LiteralPath $menuShortcutPath -PathType Leaf) {
            $shortcutShell = New-Object -ComObject WScript.Shell
            $menuShortcut = $shortcutShell.CreateShortcut($menuShortcutPath)
            if ([IO.Path]::GetFullPath($menuShortcut.TargetPath) -ine [IO.Path]::GetFullPath($installedExecutable)) { throw 'The Ultra STT Start menu shortcut targets a different application.' }
            Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
[StructLayout(LayoutKind.Sequential)]
public struct UltraSttPropertyKey {
    public Guid FormatId;
    public uint PropertyId;
    public UltraSttPropertyKey(Guid formatId, uint propertyId) { FormatId = formatId; PropertyId = propertyId; }
}
[StructLayout(LayoutKind.Explicit, Size=24)]
public struct UltraSttPropVariant {
    [FieldOffset(0)] public ushort Type;
    [FieldOffset(8)] public IntPtr StringValue;
}
[ComImport, Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
public interface UltraSttPropertyStore {
    [PreserveSig] int GetCount(out uint count);
    [PreserveSig] int GetAt(uint index, out UltraSttPropertyKey key);
    [PreserveSig] int GetValue(ref UltraSttPropertyKey key, out UltraSttPropVariant value);
    [PreserveSig] int SetValue(ref UltraSttPropertyKey key, ref UltraSttPropVariant value);
    [PreserveSig] int Commit();
}
public static class UltraSttTaskbarIconProperties {
    static readonly Guid StoreIid = new Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99");
    static readonly Guid AppUserModelFormatId = new Guid("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3");
    const string TaskbarAppId = "com.ultrastt.voicekeyboard";
    [DllImport("shell32.dll", CharSet=CharSet.Unicode, PreserveSig=true)]
    static extern int SHGetPropertyStoreFromParsingName(string path, IntPtr bindContext, uint flags, ref Guid iid, out UltraSttPropertyStore store);
    [DllImport("ole32.dll")] static extern int PropVariantClear(ref UltraSttPropVariant value);
    [DllImport("shell32.dll", CharSet=CharSet.Unicode)]
    static extern void SHChangeNotify(uint eventId, uint flags, IntPtr item1, IntPtr item2);
    static UltraSttPropertyStore Open(string path, uint flags) {
        UltraSttPropertyStore store;
        Guid iid = StoreIid;
        Marshal.ThrowExceptionForHR(SHGetPropertyStoreFromParsingName(path, IntPtr.Zero, flags, ref iid, out store));
        return store;
    }
    static UltraSttPropertyKey RelaunchIconKey() { return new UltraSttPropertyKey(AppUserModelFormatId, 3); }
    static UltraSttPropertyKey AppIdKey() { return new UltraSttPropertyKey(AppUserModelFormatId, 5); }
    public static string GetRelaunchIcon(string path) {
        var store = Open(path, 0);
        try {
            var key = RelaunchIconKey();
            UltraSttPropVariant value;
            Marshal.ThrowExceptionForHR(store.GetValue(ref key, out value));
            try { return value.Type == 31 && value.StringValue != IntPtr.Zero ? Marshal.PtrToStringUni(value.StringValue) : ""; }
            finally { PropVariantClear(ref value); }
        } finally { Marshal.ReleaseComObject(store); }
    }
    public static string GetAppId(string path) {
        var store = Open(path, 0);
        try {
            var key = AppIdKey();
            UltraSttPropVariant value;
            Marshal.ThrowExceptionForHR(store.GetValue(ref key, out value));
            try { return value.Type == 31 && value.StringValue != IntPtr.Zero ? Marshal.PtrToStringUni(value.StringValue) : ""; }
            finally { PropVariantClear(ref value); }
        } finally { Marshal.ReleaseComObject(store); }
    }
    public static void SetRelaunchIcon(string path, string iconResource) {
        var store = Open(path, 2);
        try {
            var key = RelaunchIconKey();
            var value = new UltraSttPropVariant { Type = 31, StringValue = Marshal.StringToCoTaskMemUni(iconResource) };
            try {
                Marshal.ThrowExceptionForHR(store.SetValue(ref key, ref value));
                Marshal.ThrowExceptionForHR(store.Commit());
            } finally { PropVariantClear(ref value); }
        } finally { Marshal.ReleaseComObject(store); }
    }
    public static void SetAppId(string path) {
        var store = Open(path, 2);
        try {
            var key = AppIdKey();
            var value = new UltraSttPropVariant { Type = 31, StringValue = Marshal.StringToCoTaskMemUni(TaskbarAppId) };
            try {
                Marshal.ThrowExceptionForHR(store.SetValue(ref key, ref value));
                Marshal.ThrowExceptionForHR(store.Commit());
            } finally { PropVariantClear(ref value); }
        } finally { Marshal.ReleaseComObject(store); }
    }
    public static void Refresh(string path) {
        IntPtr value = Marshal.StringToHGlobalUni(path);
        try { SHChangeNotify(0x2000, 0x0005, value, IntPtr.Zero); }
        finally { Marshal.FreeHGlobal(value); }
    }
}
'@
            $desiredIcon = "$installedExecutable,0"
            $existingRelaunchIcon = [UltraSttTaskbarIconProperties]::GetRelaunchIcon($menuShortcutPath)
            $existingAppId = [UltraSttTaskbarIconProperties]::GetAppId($menuShortcutPath)
            if ($menuShortcut.IconLocation -ne $desiredIcon -or $existingRelaunchIcon -ne $desiredIcon -or $existingAppId -ne 'com.ultrastt.voicekeyboard') {
                Copy-Item -LiteralPath $menuShortcutPath -Destination (Join-Path $backupDirectory 'original-start-menu-Ultra_STT.lnk')
                $menuShortcut.IconLocation = $desiredIcon
                $menuShortcut.Save()
                [UltraSttTaskbarIconProperties]::SetRelaunchIcon($menuShortcutPath, $desiredIcon)
                [UltraSttTaskbarIconProperties]::SetAppId($menuShortcutPath)
                [UltraSttTaskbarIconProperties]::Refresh($menuShortcutPath)
                [UltraSttTaskbarIconProperties]::Refresh($installedExecutable)
            }
            Write-Output "START_MENU_ICON=$($menuShortcut.IconLocation)"
            Write-Output "START_MENU_RELAUNCH_ICON=$([UltraSttTaskbarIconProperties]::GetRelaunchIcon($menuShortcutPath))"
            Write-Output "START_MENU_APP_ID=$([UltraSttTaskbarIconProperties]::GetAppId($menuShortcutPath))"
        }
        Start-Process -FilePath $installedExecutable -WindowStyle Hidden -RedirectStandardOutput (Join-Path $root 'runtime/windows-updated.log') -RedirectStandardError (Join-Path $root 'runtime/windows-updated.err')
        Write-Output "INSTALLED=$installedExecutable"
    } elseif ($running.Count -gt 0) {
        Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile', '-File', "`"$(Join-Path $PSScriptRoot 'start-ultra-stt.ps1')`"") -WindowStyle Hidden
    }
} finally {
    Pop-Location
}
