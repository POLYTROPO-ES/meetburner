# Draws docs/store/screenshots/04-overlay-schematic.png (1366x768).
# The overlay screenshot is a schematic, not a capture of the app: it shows how the counter
# card sits over a meeting video. Run on Windows (System.Drawing is part of the OS).
Add-Type -AssemblyName System.Drawing

$width = 1366
$height = 768
$bmp = [System.Drawing.Bitmap]::new($width, $height)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::ClearTypeGridFit
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality

function New-RoundPath([int]$x, [int]$y, [int]$w, [int]$h, [int]$r) {
    $path = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $d = 2 * $r
    $path.AddArc($x, $y, $d, $d, 180, 90)
    $path.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $path.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $path.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $path.CloseFigure()
    return $path
}

function New-UiFont([float]$size, [System.Drawing.FontStyle]$style = [System.Drawing.FontStyle]::Regular) {
    return [System.Drawing.Font]::new('Segoe UI', $size, $style, [System.Drawing.GraphicsUnit]::Pixel)
}

# Background gradient and grid.
$bgRect = [System.Drawing.Rectangle]::new(0, 0, $width, $height)
$bgBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
    $bgRect,
    [System.Drawing.Color]::FromArgb(35, 35, 38),
    [System.Drawing.Color]::FromArgb(8, 8, 10),
    65
)
$g.FillRectangle($bgBrush, $bgRect)
$bgBrush.Dispose()

$gridPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(18, 255, 255, 255), 1)
for ($x = 0; $x -lt $width; $x += 64) { $g.DrawLine($gridPen, $x, 0, $x, $height) }
for ($y = 0; $y -lt $height; $y += 64) { $g.DrawLine($gridPen, 0, $y, $width, $y) }
$gridPen.Dispose()

# Person silhouette.
$personBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(58, 58, 63))
$g.FillEllipse($personBrush, 608, 210, 150, 150)
$g.FillEllipse($personBrush, 493, 440, 380, 300)
$personBrush.Dispose()

# Live badge.
$redBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 59, 48))
$g.FillEllipse($redBrush, 42, 38, 12, 12)
$redBrush.Dispose()

$labelFont = New-UiFont 18
$labelBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(207, 199, 188))
$g.DrawString('LIVE MEETING', $labelFont, $labelBrush, 66, 32)
$labelBrush.Dispose()

$meetingFont = New-UiFont 18
$meetingBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(169, 161, 150))
$meetingFormat = [System.Drawing.StringFormat]::new()
$meetingFormat.Alignment = [System.Drawing.StringAlignment]::Far
$g.DrawString(
    '3 participants · Your meeting app',
    $meetingFont,
    $meetingBrush,
    [System.Drawing.RectangleF]::new(600, 32, 728, 30),
    $meetingFormat
)
$meetingBrush.Dispose()

# Camera tag.
$tagPath = New-RoundPath 38 92 220 40 20
$tagPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(115, 255, 181, 46), 1)
$g.DrawPath($tagPen, $tagPath)
$tagFont = New-UiFont 15
$tagBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 181, 46))
$tagFormat = [System.Drawing.StringFormat]::new()
$tagFormat.Alignment = [System.Drawing.StringAlignment]::Center
$tagFormat.LineAlignment = [System.Drawing.StringAlignment]::Center
$g.DrawString('MEETBURNER CAMERA', $tagFont, $tagBrush, [System.Drawing.RectangleF]::new(38, 92, 220, 40), $tagFormat)
$tagPen.Dispose()
$tagBrush.Dispose()
$tagPath.Dispose()

# Schematic note.
$noteFont = New-UiFont 14
$noteBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(127, 120, 111))
$noteFormat = [System.Drawing.StringFormat]::new()
$noteFormat.Alignment = [System.Drawing.StringAlignment]::Far
$g.DrawString(
    'SCHEMATIC · NOT A SCREENSHOT',
    $noteFont,
    $noteBrush,
    [System.Drawing.RectangleF]::new(600, 100, 728, 24),
    $noteFormat
)
$noteBrush.Dispose()

# Hint.
$hintFont = New-UiFont 18
$hintBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(185, 177, 167))
$g.DrawString(
    'The counter is published as a camera, so your meeting app shows it as your video — or over your real webcam, as the card below.',
    $hintFont,
    $hintBrush,
    [System.Drawing.RectangleF]::new(38, 600, 430, 130)
)
$hintBrush.Dispose()

# Counter card.
$cardPath = New-RoundPath 792 470 520 244 22
$cardBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(235, 8, 8, 9))
$g.FillPath($cardBrush, $cardPath)
$cardPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(50, 255, 255, 255), 1)
$g.DrawPath($cardPen, $cardPath)
$cardBrush.Dispose()
$cardPen.Dispose()
$cardPath.Dispose()

$totalFont = New-UiFont 14
$totalBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(185, 177, 167))
$g.DrawString('TOTAL COST', $totalFont, $totalBrush, 820, 496)
$totalBrush.Dispose()

# Split-flap plates.
$plateFont = New-UiFont 46 ([System.Drawing.FontStyle]::Bold)
$plateBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 154, 46))
$seamPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(8, 8, 10), 2)
$plateFormat = [System.Drawing.StringFormat]::new()
$plateFormat.Alignment = [System.Drawing.StringAlignment]::Center
$plateFormat.LineAlignment = [System.Drawing.StringAlignment]::Center

$x = 820
foreach ($char in @('9', '·', '5', '8', '€')) {
    $w = if ($char -eq '·') { 26 } elseif ($char -eq '€') { 46 } else { 58 }
    $rect = [System.Drawing.RectangleF]::new($x, 526, $w, 82)
    if ($char -ne '·') {
        $path = New-RoundPath ([int]$rect.X) ([int]$rect.Y) ([int]$rect.Width) ([int]$rect.Height) 7
        $plateBg = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
            $rect,
            [System.Drawing.Color]::FromArgb(43, 43, 47),
            [System.Drawing.Color]::FromArgb(32, 32, 36),
            90
        )
        $g.FillPath($plateBg, $path)
        $plateBg.Dispose()
        $plateBorder = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(30, 255, 255, 255), 1)
        $g.DrawPath($plateBorder, $path)
        $plateBorder.Dispose()
        $g.DrawLine($seamPen, $rect.X, $rect.Y + 41, $rect.X + $rect.Width, $rect.Y + 41)
        $path.Dispose()
    }
    $font = if ($char -eq '·') { New-UiFont 36 } elseif ($char -eq '€') { New-UiFont 34 } else { $plateFont }
    $g.DrawString($char, $font, $plateBrush, $rect, $plateFormat)
    if ($char -eq '·' -or $char -eq '€') { $font.Dispose() }
    $x += $w + 8
}
$plateBrush.Dispose()
$seamPen.Dispose()
$plateFont.Dispose()

# Meta line.
$metaFont = New-UiFont 20
$metaBold = New-UiFont 20 ([System.Drawing.FontStyle]::Bold)
$metaBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(216, 208, 197))
$g.DrawString('Elapsed', $metaFont, $metaBrush, 820, 636)
$g.DrawString('00:14:32', $metaBold, [System.Drawing.Brushes]::White, 900, 636)
$metaFormat = [System.Drawing.StringFormat]::new()
$metaFormat.Alignment = [System.Drawing.StringAlignment]::Far
$g.DrawString(
    'Cost per minute  5.13 €',
    $metaFont,
    $metaBrush,
    [System.Drawing.RectangleF]::new(900, 636, 384, 30),
    $metaFormat
)
$metaBrush.Dispose()

$out = Join-Path $PSScriptRoot 'screenshots\04-overlay-schematic.png'
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()
Write-Host "wrote $out"
