<#
.SYNOPSIS
    Register mdo with Windows Explorer for .md files (current user only).

.DESCRIPTION
    Adds mdo to the per-user Explorer integration for .md files:
      1. Registers an "Application" entry so mdo shows up in
         "Open with -> Choose another app".
      2. Adds .md to its OpenWithProgids list so Explorer offers it.
      3. Adds a "Render with mdo" right-click verb on .md files.

    All changes are written under HKCU (HKEY_CURRENT_USER), so no admin
    rights are required and nothing system-wide is touched.

    To make mdo the *default* handler for .md, after running this
    script: right-click a .md file -> Open with -> Choose another app ->
    pick mdo -> tick "Always use this app". Windows requires that
    last step to be done interactively.

    Run scripts/uninstall-explorer.ps1 to undo everything this script does.

.PARAMETER ExePath
    Full path to mdo-open.exe. If omitted, the script tries
    `Get-Command mdo-open` and then falls back to
    ..\target\release\mdo-open.exe relative to this script.

    mdo-open.exe is a tiny windows-subsystem wrapper shipped alongside
    mdo.exe specifically so Explorer launches do not flash a console
    window. It must live in the same directory as mdo.exe.

.PARAMETER IconChar
    Single character to render into a .ico used by Explorer for the
    "Open with" entry and the right-click verb. Defaults to Ⓜ (U+24C2,
    "circled latin capital letter M"). Try "📄" for a page-curl emoji.
    The generated icon is written to %LOCALAPPDATA%\mdo\md.ico.

.PARAMETER IconColor
    Hex color (e.g. "#1E66E2" or "1E66E2") for the rendered glyph. The
    default is a mid-tone blue chosen so the glyph stays legible on both
    light and dark Explorer themes. Pure black ("#000000") will vanish
    on Windows dark mode.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\scripts\install-explorer.ps1

.EXAMPLE
    .\scripts\install-explorer.ps1 -ExePath "C:\Tools\mdo-open.exe"

.EXAMPLE
    .\scripts\install-explorer.ps1 -IconChar "📄" -IconColor "#E64A19"
#>
[CmdletBinding()]
param(
    [string]$ExePath,
    [string]$IconChar  = ([char]0x24C2),  # Ⓜ
    [string]$IconColor = '#1E66E2'        # legible on both light and dark
)

$ErrorActionPreference = 'Stop'

function Resolve-ExePath {
    param([string]$Hint)

    if ($Hint) {
        if (-not (Test-Path -LiteralPath $Hint)) {
            throw "mdo-open.exe not found at: $Hint"
        }
        return (Resolve-Path -LiteralPath $Hint).Path
    }

    $cmd = Get-Command mdo-open -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    $local = Join-Path $PSScriptRoot '..\target\release\mdo-open.exe'
    if (Test-Path -LiteralPath $local) {
        return (Resolve-Path -LiteralPath $local).Path
    }

    throw "Could not locate mdo-open.exe. Build it with ``cargo build --release`` or pass -ExePath C:\path\to\mdo-open.exe"
}

function New-CharIcon {
    <#
        Render a single Unicode character into a 256x256 PNG and wrap it
        in an ICO container (PNG-in-ICO, supported by Windows Vista+).
        Returns the path written.

        Note: GDI+ (System.Drawing) renders text via GDI, which on most
        installs draws color emoji as their monochrome fallback glyph.
        That's still a recognizable, scalable shape — fine for a 16/32px
        Explorer icon. For a glyph that *always* looks crisp regardless of
        platform, the default Ⓜ (a regular Unicode letter, not an emoji)
        is the safest choice.
    #>
    param(
        [Parameter(Mandatory)] [string]$Char,
        [Parameter(Mandatory)] [string]$OutPath,
        [string]$HexColor = '#1E66E2',
        [int]$Size = 256
    )

    Add-Type -AssemblyName System.Drawing

    # Parse "#RRGGBB" / "RRGGBB" into a Color, falling back to black on error.
    $hex = $HexColor.TrimStart('#')
    try {
        $r = [Convert]::ToInt32($hex.Substring(0, 2), 16)
        $g_ = [Convert]::ToInt32($hex.Substring(2, 2), 16)
        $b = [Convert]::ToInt32($hex.Substring(4, 2), 16)
        $color = [System.Drawing.Color]::FromArgb(255, $r, $g_, $b)
    } catch {
        Write-Warning "Invalid -IconColor '$HexColor', falling back to black"
        $color = [System.Drawing.Color]::Black
    }

    $bmp = New-Object System.Drawing.Bitmap($Size, $Size)
    $g   = [System.Drawing.Graphics]::FromImage($bmp)
    try {
        $g.SmoothingMode     = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
        $g.Clear([System.Drawing.Color]::Transparent)

        # Segoe UI Emoji covers both the circled-M and emoji ranges.
        $font = New-Object System.Drawing.Font(
            'Segoe UI Emoji',
            [single]($Size * 0.78),
            [System.Drawing.FontStyle]::Regular,
            [System.Drawing.GraphicsUnit]::Pixel)

        $sf = New-Object System.Drawing.StringFormat
        $sf.Alignment     = [System.Drawing.StringAlignment]::Center
        $sf.LineAlignment = [System.Drawing.StringAlignment]::Center

        $brush = New-Object System.Drawing.SolidBrush($color)
        $rect = New-Object System.Drawing.RectangleF(0, 0, [single]$Size, [single]$Size)
        $g.DrawString($Char, $font, $brush, $rect, $sf)
        $brush.Dispose()
        $font.Dispose()
        $sf.Dispose()
    } finally {
        $g.Dispose()
    }

    # PNG bytes
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    $pngBytes = $ms.ToArray()

    # ICO container: 6-byte ICONDIR + 16-byte ICONDIRENTRY + PNG payload.
    # Width/height bytes set to 0 mean "256" per the ICO spec.
    $ico = New-Object System.IO.MemoryStream
    $bw  = New-Object System.IO.BinaryWriter($ico)
    $bw.Write([uint16]0)                  # reserved
    $bw.Write([uint16]1)                  # type: 1 = icon
    $bw.Write([uint16]1)                  # image count
    $bw.Write([byte]0)                    # width  (0 = 256)
    $bw.Write([byte]0)                    # height (0 = 256)
    $bw.Write([byte]0)                    # color count (0 for >=8bpp)
    $bw.Write([byte]0)                    # reserved
    $bw.Write([uint16]1)                  # planes
    $bw.Write([uint16]32)                 # bits per pixel
    $bw.Write([uint32]$pngBytes.Length)   # image size
    $bw.Write([uint32]22)                 # offset to image data (6 + 16)
    $bw.Write($pngBytes)
    $bw.Flush()

    $dir = Split-Path -Parent $OutPath
    if (-not (Test-Path -LiteralPath $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    [System.IO.File]::WriteAllBytes($OutPath, $ico.ToArray())
    return $OutPath
}

$exe = Resolve-ExePath -Hint $ExePath

# mdo-open.exe always implies --open, so the registry value is just
# `"<exe>" "%1"` — no extra flags. The wrapper itself spawns mdo.exe
# with CREATE_NO_WINDOW, which is what eliminates the Explorer console flash.
$cmd = '"{0}" "%1"' -f $exe

# Sanity check: mdo.exe must live next to mdo-open.exe so the
# wrapper can find it at runtime.
$sibling = Join-Path (Split-Path -Parent $exe) 'mdo.exe'
if (-not (Test-Path -LiteralPath $sibling)) {
    Write-Warning "mdo.exe not found next to mdo-open.exe at: $sibling"
    Write-Warning "Explorer integration will be registered, but double-clicking will fail until the main binary is in place."
}

# Render the requested glyph into a .ico used by every Explorer surface
# below (Open-with picker, ProgId, right-click verb).
$iconPath = Join-Path $env:LOCALAPPDATA 'mdo\md.ico'
New-CharIcon -Char $IconChar -HexColor $IconColor -OutPath $iconPath | Out-Null
$iconRef = '"{0}",0' -f $iconPath

Write-Host "Using mdo-open: $exe"
Write-Host "Command line       : $cmd"
Write-Host "Icon ($IconChar)            : $iconPath"
Write-Host ""

# 1. Register the application so Explorer's "Open with" picker can find it.
$appRoot = 'HKCU:\Software\Classes\Applications\mdo.exe'
New-Item -Path "$appRoot\shell\open\command" -Force | Out-Null
Set-ItemProperty -Path "$appRoot\shell\open\command" -Name '(Default)' -Value $cmd
Set-ItemProperty -Path $appRoot -Name 'FriendlyAppName' -Value 'mdo (Markdown -> HTML)'
# DefaultIcon under Applications\<exe> is what the Open-with picker shows.
New-Item -Path "$appRoot\DefaultIcon" -Force | Out-Null
Set-ItemProperty -Path "$appRoot\DefaultIcon" -Name '(Default)' -Value $iconRef

# 2. Offer mdo as a choice for .md files in the Open-with list.
$openWith = 'HKCU:\Software\Classes\.md\OpenWithProgids'
New-Item -Path $openWith -Force | Out-Null
New-ItemProperty -Path $openWith -Name 'mdo.md' -Value '' -PropertyType String -Force | Out-Null

$progid = 'HKCU:\Software\Classes\mdo.md'
New-Item -Path "$progid\shell\open\command" -Force | Out-Null
Set-ItemProperty -Path $progid -Name '(Default)' -Value 'Markdown document (mdo)'
Set-ItemProperty -Path "$progid\shell\open\command" -Name '(Default)' -Value $cmd
# DefaultIcon under the ProgId is what Explorer shows for files whose
# default app is mdo.
New-Item -Path "$progid\DefaultIcon" -Force | Out-Null
Set-ItemProperty -Path "$progid\DefaultIcon" -Name '(Default)' -Value $iconRef

# 3. Add a "Render with mdo" right-click verb on every .md file
#    (works alongside whatever the current default handler is).
$verb = 'HKCU:\Software\Classes\SystemFileAssociations\.md\shell\Render with mdo'
New-Item -Path "$verb\command" -Force | Out-Null
Set-ItemProperty -Path "$verb\command" -Name '(Default)' -Value $cmd
Set-ItemProperty -Path $verb -Name 'Icon' -Value $iconRef

Write-Host "Done." -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:"
Write-Host "  - Right-click any .md file -> 'Render with mdo' (Win11: Show more options)."
Write-Host "  - To make it the default: Open with -> Choose another app -> mdo -> Always."
