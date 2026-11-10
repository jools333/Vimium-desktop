Add-Type -AssemblyName System.Drawing

$sizes = @(16, 24, 32, 48, 64, 128, 256)
$bitmaps = @()

foreach ($sz in $sizes) {
    $bmp = New-Object System.Drawing.Bitmap([int]$sz, [int]$sz, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic

    $fSz = [float]$sz
    $pad = [float][Math]::Max(1.0, $fSz * 0.06)
    $w = [float]($fSz - (2.0 * $pad))
    $h = [float]($fSz - (2.0 * $pad))
    $radius = [float]($fSz * 0.22)
    $diameter = [float]($radius * 2.0)

    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $rect = New-Object System.Drawing.RectangleF($pad, $pad, $w, $h)

    $path.AddArc($rect.X, $rect.Y, $diameter, $diameter, 180.0, 90.0)
    $path.AddArc([float]($rect.Right - $diameter), $rect.Y, $diameter, $diameter, 270.0, 90.0)
    $path.AddArc([float]($rect.Right - $diameter), [float]($rect.Bottom - $diameter), $diameter, $diameter, 0.0, 90.0)
    $path.AddArc($rect.X, [float]($rect.Bottom - $diameter), $diameter, $diameter, 90.0, 90.0)
    $path.CloseFigure()

    # Gradient fill: vibrant gold/yellow (Vimium hint color)
    $pt1 = New-Object System.Drawing.PointF(0.0, $pad)
    $pt2 = New-Object System.Drawing.PointF(0.0, [float]($fSz - $pad))
    $col1 = [System.Drawing.Color]::FromArgb(255, 255, 215, 64)
    $col2 = [System.Drawing.Color]::FromArgb(255, 255, 175, 0)
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush($pt1, $pt2, $col1, $col2)
    $g.FillPath($brush, $path)

    # Key border
    $borderCol = [System.Drawing.Color]::FromArgb(230, 180, 100, 0)
    $borderWidth = [float][Math]::Max(1.0, $fSz * 0.05)
    $pen = New-Object System.Drawing.Pen($borderCol, $borderWidth)
    $g.DrawPath($pen, $path)

    # Draw Letter 'V'
    $fontFamily = [System.Drawing.FontFamily]::GenericSansSerif
    $fontSize = [float]($fSz * 0.52)
    $fontStyle = [System.Drawing.FontStyle]::Bold
    $font = New-Object System.Drawing.Font($fontFamily, $fontSize, $fontStyle, [System.Drawing.GraphicsUnit]::Pixel)

    $sf = New-Object System.Drawing.StringFormat
    $sf.Alignment = [System.Drawing.StringAlignment]::Center
    $sf.LineAlignment = [System.Drawing.StringAlignment]::Center

    $textBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 24, 24, 27))
    $textRect = New-Object System.Drawing.RectangleF(0.0, [float]($pad * 0.4), $fSz, $fSz)
    $g.DrawString("V", $font, $textBrush, $textRect, $sf)

    $g.Dispose()
    $bitmaps += $bmp
}

$destDir = "$PSScriptRoot\..\assets"
if (-not (Test-Path $destDir)) {
    New-Item -ItemType Directory -Force -Path $destDir | Out-Null
}

# Save largest as png
$bitmaps[-1].Save("$destDir\icon.png", [System.Drawing.Imaging.ImageFormat]::Png)

# Build .ico file format
$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($ms)

# ICONDIR
$bw.Write([uint16]0) # Reserved
$bw.Write([uint16]1) # Type (1=ICO)
$bw.Write([uint16]$bitmaps.Count) # Image count

$offset = 6 + (16 * $bitmaps.Count)
$pngDataList = @()

foreach ($bmp in $bitmaps) {
    $pngStream = New-Object System.IO.MemoryStream
    $bmp.Save($pngStream, [System.Drawing.Imaging.ImageFormat]::Png)
    $pngBytes = $pngStream.ToArray()
    $pngDataList += ,$pngBytes

    $w = if ($bmp.Width -ge 256) { 0 } else { [byte]$bmp.Width }
    $h = if ($bmp.Height -ge 256) { 0 } else { [byte]$bmp.Height }

    $bw.Write([byte]$w)
    $bw.Write([byte]$h)
    $bw.Write([byte]0)   # Colors
    $bw.Write([byte]0)   # Reserved
    $bw.Write([uint16]1) # Color planes
    $bw.Write([uint16]32) # Bits per pixel
    $bw.Write([uint32]$pngBytes.Length)
    $bw.Write([uint32]$offset)

    $offset += $pngBytes.Length
}

foreach ($pngBytes in $pngDataList) {
    $bw.Write($pngBytes)
}

$bw.Flush()
[System.IO.File]::WriteAllBytes("$destDir\icon.ico", $ms.ToArray())
$ms.Dispose()
Write-Host "Generated assets\icon.ico and assets\icon.png successfully!" -ForegroundColor Green
