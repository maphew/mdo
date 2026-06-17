$ErrorActionPreference = "Stop"

$Repo = "maphew/mdo"
$BaseUrl = "https://github.com/$Repo/releases/latest/download"
$Asset = "mdo-x86_64-pc-windows-msvc.zip"
$InstallDir = if ($env:MDO_INSTALL_DIR) {
    $env:MDO_INSTALL_DIR
} else {
    Join-Path $env:LOCALAPPDATA "mdo\bin"
}
$TempDir = Join-Path ([IO.Path]::GetTempPath()) ("mdo-install-" + [Guid]::NewGuid())

New-Item -ItemType Directory -Force -Path $InstallDir, $TempDir | Out-Null

try {
    Push-Location $TempDir

    Invoke-WebRequest -Uri "$BaseUrl/$Asset" -OutFile $Asset
    Invoke-WebRequest -Uri "$BaseUrl/SHA256SUMS" -OutFile SHA256SUMS

    $line = (Select-String -Path SHA256SUMS -Pattern ([regex]::Escape($Asset))).Line
    if (-not $line) {
        throw "Could not find $Asset in SHA256SUMS"
    }

    $expected = ($line -split "\s+")[0].ToLowerInvariant()
    $actual = (Get-FileHash ".\$Asset" -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) {
        throw "SHA256 mismatch for $Asset"
    }

    Expand-Archive -Force ".\$Asset" .
    Copy-Item -Force ".\mdo-x86_64-pc-windows-msvc\mdo*.exe" $InstallDir

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $paths = @($userPath -split ";" | Where-Object { $_ })
    if ($paths -notcontains $InstallDir) {
        [Environment]::SetEnvironmentVariable(
            "Path",
            (@($paths) + $InstallDir) -join ";",
            "User"
        )
        Write-Host "Added $InstallDir to your user PATH. Open a new terminal if mdo is not found."
    }

    Write-Host "mdo installed to $InstallDir"
    & (Join-Path $InstallDir "mdo.exe") --version
} finally {
    Pop-Location
    Remove-Item -LiteralPath $TempDir -Recurse -Force
}
