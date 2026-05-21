<#
.SYNOPSIS
    Remove the mdo Explorer integration created by install-explorer.ps1.

.DESCRIPTION
    Deletes every HKCU registry key the install script creates:
      - HKCU\Software\Classes\Applications\mdo.exe
      - HKCU\Software\Classes\mdo.md
      - HKCU\Software\Classes\.md\OpenWithProgids\mdo.md value
      - HKCU\Software\Classes\SystemFileAssociations\.md\shell\Render with mdo

    Leaves the .md OpenWithProgids key itself in place (other apps may
    have entries there).

    Does not touch HKLM and does not require admin.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\scripts\uninstall-explorer.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Continue'

function Remove-KeyIfPresent {
    param([string]$Path)
    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
        Write-Host "Removed: $Path"
    }
    else {
        Write-Host "Skip   : $Path (not present)"
    }
}

Remove-KeyIfPresent 'HKCU:\Software\Classes\Applications\mdo.exe'
Remove-KeyIfPresent 'HKCU:\Software\Classes\mdo.md'
Remove-KeyIfPresent 'HKCU:\Software\Classes\SystemFileAssociations\.md\shell\Render with mdo'

# Also remove legacy keys from the pre-rename integration. These are harmless
# if absent and keep upgrades from leaving stale Explorer entries behind.
$legacyName = 'md2' + 'htmlx'
Remove-KeyIfPresent "HKCU:\Software\Classes\Applications\$legacyName.exe"
Remove-KeyIfPresent "HKCU:\Software\Classes\$legacyName.md"
Remove-KeyIfPresent "HKCU:\Software\Classes\SystemFileAssociations\.md\shell\Render with $legacyName"

# Remove the generated .ico (and its folder, if it ends up empty).
$iconDir = Join-Path $env:LOCALAPPDATA 'mdo'
$iconPath = Join-Path $iconDir 'md.ico'
if (Test-Path -LiteralPath $iconPath) {
    Remove-Item -LiteralPath $iconPath -Force
    Write-Host "Removed: $iconPath"
}
if ((Test-Path -LiteralPath $iconDir) -and -not (Get-ChildItem -LiteralPath $iconDir -Force)) {
    Remove-Item -LiteralPath $iconDir -Force
    Write-Host "Removed: $iconDir (empty)"
}

# Just remove the single value we added under OpenWithProgids; leave the key.
$openWith = 'HKCU:\Software\Classes\.md\OpenWithProgids'
if (Test-Path -LiteralPath $openWith) {
    $prop = Get-ItemProperty -LiteralPath $openWith -Name 'mdo.md' -ErrorAction SilentlyContinue
    if ($prop) {
        Remove-ItemProperty -LiteralPath $openWith -Name 'mdo.md' -Force
        Write-Host "Removed value: $openWith\mdo.md"
    }
    else {
        Write-Host "Skip   : $openWith\mdo.md (not present)"
    }

    $legacyProgId = "$legacyName.md"
    $legacyProp = Get-ItemProperty -LiteralPath $openWith -Name $legacyProgId -ErrorAction SilentlyContinue
    if ($legacyProp) {
        Remove-ItemProperty -LiteralPath $openWith -Name $legacyProgId -Force
        Write-Host "Removed legacy value: $openWith\$legacyProgId"
    }
}

Write-Host ""
Write-Host "Done." -ForegroundColor Green
Write-Host "If mdo was set as the default handler for .md, Windows will"
Write-Host "prompt you to pick a new default the next time you open a .md file."
