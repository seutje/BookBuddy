# Deterministic placeholder artwork; no downloaded assets.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$bitmap = New-Object System.Drawing.Bitmap 256, 256
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$background = New-Object System.Drawing.SolidBrush ([System.Drawing.ColorTranslator]::FromHtml('#183f32'))
$paper = New-Object System.Drawing.SolidBrush ([System.Drawing.ColorTranslator]::FromHtml('#faf9f5'))
$spine = New-Object System.Drawing.Pen ([System.Drawing.ColorTranslator]::FromHtml('#ae7825')), 8
try {
    $graphics.FillRectangle($background, 0, 0, 256, 256)
    $graphics.FillRectangle($paper, 48, 52, 72, 152)
    $graphics.FillRectangle($paper, 136, 52, 72, 152)
    $graphics.DrawLine($spine, 128, 52, 128, 204)
    $destination = Join-Path $PSScriptRoot '../src-tauri/icons'
    New-Item -ItemType Directory -Force -Path $destination | Out-Null
    $bitmap.Save((Join-Path $destination 'source.png'), [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
    $spine.Dispose()
    $paper.Dispose()
    $background.Dispose()
    $graphics.Dispose()
    $bitmap.Dispose()
}
