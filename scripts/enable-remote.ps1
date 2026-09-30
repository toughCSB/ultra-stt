# Run after launching Ultra_STT on the always-on PC. This modifies only a
# previously unused Tailnet HTTPS port; it never enables public Funnel access.
$ErrorActionPreference = 'Stop'
$tailscale = 'C:/Program Files/Tailscale/tailscale.exe'
$remote = (Resolve-Path (Join-Path $PSScriptRoot '../remote')).Path
$port = 10443

if (!(Test-Path -LiteralPath $tailscale -PathType Leaf)) {
    throw 'Tailscale is not installed on this PC.'
}
if (!(Get-NetTCPConnection -LocalAddress '127.0.0.1' -LocalPort 17846 -State Listen -ErrorAction SilentlyContinue)) {
    throw 'Start Ultra_STT first; its local remote-input listener is not ready.'
}
$status = (& $tailscale status --json | ConvertFrom-Json -AsHashtable)
if ($LASTEXITCODE -ne 0 -or $status.BackendState -ne 'Running') {
    throw 'Connect Tailscale before enabling remote input.'
}
$owner = $status.User[[string]$status.Self.UserID].LoginName
if (!$owner -or !$status.Self.DNSName) {
    throw 'The current Tailscale owner could not be identified.'
}
$existing = (& $tailscale serve status --json | ConvertFrom-Json -AsHashtable)
if ($LASTEXITCODE -ne 0 -or $existing.TCP.ContainsKey([string]$port)) {
    throw "Tailnet HTTPS port $port is unavailable; existing Serve settings were not changed."
}

& $tailscale serve --bg --https=$port --set-path=/ $remote
if ($LASTEXITCODE -ne 0) { throw 'Could not serve the Ultra_STT remote page.' }
& $tailscale serve --bg --https=$port --set-path=/ws 'http://127.0.0.1:17846'
& $tailscale serve --bg --https=$port --set-path=/v1 'http://127.0.0.1:17846'
if ($LASTEXITCODE -ne 0) {
    & $tailscale serve --https=$port --set-path=/ off | Out-Null
    throw 'Could not connect the remote page to Ultra_STT; its new file share was removed.'
}
$dnsName = $status.Self.DNSName.TrimEnd('.')
Write-Output "Ultra_STT remote input: https://${dnsName}:$port/"
