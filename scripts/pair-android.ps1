param(
    [Parameter(Mandatory = $true)][string]$Serial,
    [string]$ToolRoot,
    [string]$Endpoint = 'https://sb.tail337f88.ts.net:10443/v1/audio/transcriptions'
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $ToolRoot) {
    $projectsRoot = Split-Path -Parent (Split-Path -Parent $projectRoot)
    $ToolRoot = Join-Path $projectsRoot 'my-family-app\.sites-runtime\android-tools'
}
$adb = Join-Path $ToolRoot 'sdk\platform-tools\adb.exe'
if (-not (Test-Path -LiteralPath $adb)) { throw 'Existing Android adb tool not found.' }
$model = (& $adb -s $Serial shell getprop ro.product.model).Trim()
if ($LASTEXITCODE -ne 0 -or $model -ne 'SM-F966N') { throw 'The approved connected Galaxy SM-F966N was not found.' }
$tokenPath = Join-Path (Split-Path -Parent $projectRoot) 'data\remote-client-token'
$token = [IO.File]::ReadAllText($tokenPath).Trim()
if ($token -notmatch '^[0-9a-fA-F]{96}$') { throw 'The existing server token is invalid.' }
$self = (& tailscale status --json | ConvertFrom-Json).Self
if ($LASTEXITCODE -ne 0 -or $self.DNSName.TrimEnd('.') -ne ([Uri]$Endpoint).DnsSafeHost) {
    throw 'The endpoint must match this approved PC Tailscale hostname.'
}
$serverAddress = @($self.TailscaleIPs | Where-Object { $_ -match '^100\.(\d+)\.(\d+)\.(\d+)$' -and [int]$Matches[1] -ge 64 -and [int]$Matches[1] -le 127 }) | Select-Object -First 1
if (-not $serverAddress) { throw 'The approved PC Tailscale IPv4 address was not found.' }
$configuration = @{
    endpoint = $Endpoint
    serverAddress = $serverAddress
    token = $token
    expiresAt = [DateTimeOffset]::UtcNow.AddMinutes(10).ToUnixTimeMilliseconds()
} | ConvertTo-Json -Compress

$start = [Diagnostics.ProcessStartInfo]::new()
$start.FileName = $adb
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
$start.RedirectStandardInput = $true
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
foreach ($argument in @('-s', $Serial, 'shell', '-T', "run-as com.ultrastt.androidvoice sh -c 'umask 077; mkdir -p files; cat > files/pairing.json'")) {
    $start.ArgumentList.Add($argument)
}
$process = [Diagnostics.Process]::Start($start)
try {
    $process.StandardInput.WriteLine($configuration)
    $process.StandardInput.Close()
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { throw 'Private device pairing transfer failed. No token is printed.' }
} finally {
    $process.Dispose()
    $token = $null
    $configuration = $null
}
& $adb -s $Serial shell am start -W -f 0x14000000 -n com.ultrastt.androidvoice/.SettingsActivity
if ($LASTEXITCODE -ne 0) { throw 'Could not open the app to import its private pairing file.' }
Start-Sleep -Seconds 1
& $adb -s $Serial shell run-as com.ultrastt.androidvoice test '!' -f files/pairing.json
if ($LASTEXITCODE -ne 0) { throw 'The private pairing file was not consumed. Open Ultra STT and try again.' }
& $adb -s $Serial shell run-as com.ultrastt.androidvoice grep -q token-encrypted shared_prefs/voice-settings.xml
if ($LASTEXITCODE -ne 0) { throw 'The app did not store the encrypted server token. Reopen Ultra STT and retry.' }
Write-Output 'Approved Galaxy pairing imported; temporary device file removed. No token value was printed.'
