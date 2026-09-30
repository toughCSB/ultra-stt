param(
    [switch]$Login,
    [ValidateSet('shared', 'isolated')][string]$Profile
)

$ErrorActionPreference = 'Stop'
$app = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$root = (Resolve-Path (Join-Path $app '..')).Path
$profileHome = Join-Path $root 'runtime/codex-app-home'
$executable = if ($env:ULTRA_STT_CODEX_EXE) { $env:ULTRA_STT_CODEX_EXE } else {
    Join-Path $env:APPDATA 'npm/node_modules/@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe'
}
if (!(Test-Path -LiteralPath $executable -PathType Leaf)) { throw 'Native Codex executable is unavailable.' }
if (!(Test-Path -LiteralPath $profileHome -PathType Container)) {
    New-Item -ItemType Directory -Path $profileHome | Out-Null
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent().User
    $permissions = [Security.AccessControl.DirectorySecurity]::new()
    $permissions.SetOwner($identity)
    $permissions.SetAccessRuleProtection($true, $false)
    $rule = [Security.AccessControl.FileSystemAccessRule]::new($identity, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
    $permissions.AddAccessRule($rule)
    Set-Acl -LiteralPath $profileHome -AclObject $permissions
}
Copy-Item -LiteralPath (Join-Path $app 'src-tauri/resources/codex-app-AGENTS.md') -Destination (Join-Path $profileHome 'AGENTS.md')
Copy-Item -LiteralPath (Join-Path $app 'src-tauri/resources/codex-app-instructions.md') -Destination (Join-Path $profileHome 'model-instructions.md')
$saved = @{}
$names = @('CODEX_HOME', 'OPENAI_API_KEY', 'CODEX_API_KEY', 'AZURE_OPENAI_API_KEY', 'OPENAI_BASE_URL', 'OPENAI_API_BASE', 'AZURE_OPENAI_ENDPOINT', 'CODEX_BASE_URL', 'CODEX_API_BASE_URL')
foreach ($name in $names) {
    $saved[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
    [Environment]::SetEnvironmentVariable($name, $null, 'Process')
}
try {
    $env:CODEX_HOME = $profileHome
    if ($Login) {
        & $executable login
        if ($LASTEXITCODE -ne 0) { throw 'App-only Codex login did not complete.' }
    }
    if ($Profile -eq 'isolated') {
        $loginStatus = (& $executable login status 2>&1 | Out-String).Trim()
        if ($LASTEXITCODE -ne 0 -or $loginStatus -ne 'Logged in using ChatGPT') {
            throw 'Complete the app-only ChatGPT login before enabling the isolated profile.'
        }
    }
    if ($Profile) {
        Set-Content -LiteralPath (Join-Path $root 'runtime/codex-profile') -Value $Profile -Encoding ascii -NoNewline
        Write-Output "ULTRA_STT_CODEX_PROFILE=$Profile"
    } else { Write-Output 'Prepared the app-only profile; the active profile is unchanged.' }
} finally {
    foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $saved[$name], 'Process') }
}
