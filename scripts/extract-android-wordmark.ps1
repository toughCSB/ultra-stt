$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$source = [System.Drawing.Bitmap]::new((Join-Path $root 'src/assets/brand/title-lockup.png'))
$wordmark = $null
try {
    if ($source.Width -ne 2216 -or $source.Height -ne 709) {
        throw 'The original title-lockup dimensions changed; inspect the crop before extracting.'
    }
    $left = $source.Width
    $top = $source.Height
    $right = 0
    $bottom = 0
    for ($row = 142; $row -lt 425; $row++) {
        for ($column = 735; $column -lt $source.Width; $column++) {
            $color = $source.GetPixel($column, $row)
            if ([Math]::Max($color.R, [Math]::Max($color.G, $color.B)) -gt 150) {
                $left = [Math]::Min($left, $column)
                $top = [Math]::Min($top, $row)
                $right = [Math]::Max($right, $column)
                $bottom = [Math]::Max($bottom, $row)
            }
        }
    }
    $left -= 4
    $top -= 4
    $wordmark = [System.Drawing.Bitmap]::new($right - $left + 5, $bottom - $top + 5, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    for ($row = 0; $row -lt $wordmark.Height; $row++) {
        for ($column = 0; $column -lt $wordmark.Width; $column++) {
            $color = $source.GetPixel($column + $left, $row + $top)
            $brightness = [Math]::Max($color.R, [Math]::Max($color.G, $color.B))
            $alpha = [int][Math]::Clamp(($brightness - 50) * 2.55, 0, 255)
            $wordmark.SetPixel($column, $row, [System.Drawing.Color]::FromArgb($alpha, $color.R, $color.G, $color.B))
        }
    }
    $wordmark.Save((Join-Path $root 'android/app/src/main/res/drawable-nodpi/brand_wordmark.png'), [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
    if ($wordmark) { $wordmark.Dispose() }
    $source.Dispose()
}
