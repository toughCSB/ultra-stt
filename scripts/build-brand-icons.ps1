param([string]$InputImage)

$ErrorActionPreference = 'Stop'
$project = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$icons = Join-Path $project 'src-tauri/icons'
if (!$InputImage) { $InputImage = Join-Path $icons 'ultra-stt-microphone.png' }
$source = (Resolve-Path -LiteralPath $InputImage).Path
Add-Type -AssemblyName System.Drawing
$original = [Drawing.Bitmap]::new($source)
try {
    if ($original.Width -ne $original.Height) { throw 'Brand icon must be square.' }
} finally { $original.Dispose() }
Push-Location $project
try {
    npx --no-install tauri icon $source --output $icons
    if ($LASTEXITCODE -ne 0) { throw 'Platform icon conversion failed.' }
    npx --no-install tauri icon $source --output $icons --png 64
    if ($LASTEXITCODE -ne 0) { throw 'Tray icon conversion failed.' }
    Copy-Item -LiteralPath (Join-Path $icons '64x64.png') -Destination (Join-Path $icons 'tray-icon.png')
    Copy-Item -LiteralPath $source -Destination (Join-Path $icons 'icon.png')
    $bitmap = [Drawing.Bitmap]::new((Join-Path $icons 'tray-icon.png'))
    try {
        $pixels = [byte[]]::new(64 * 64 * 4)
        for ($row = 0; $row -lt 64; $row++) {
            for ($column = 0; $column -lt 64; $column++) {
                $pixel = $bitmap.GetPixel($column, $row)
                $offset = ($row * 64 + $column) * 4
                $pixels[$offset] = $pixel.R
                $pixels[$offset + 1] = $pixel.G
                $pixels[$offset + 2] = $pixel.B
                $pixels[$offset + 3] = $pixel.A
            }
        }
        [IO.File]::WriteAllBytes((Join-Path $icons 'tray-icon.rgba'), $pixels)
    } finally { $bitmap.Dispose() }
} finally { Pop-Location }
