$ErrorActionPreference = 'Stop'

Write-Host "Rewind Installer for Windows" -ForegroundColor Cyan

# Define variables
$Repo = "Chronos778/git-rewind"
$ApiUrl = "https://api.github.com/repos/$Repo/releases/latest"

try {
    $Release = Invoke-RestMethod -Uri $ApiUrl -UseBasicParsing
} catch {
    Write-Host "Failed to fetch latest release: $_" -ForegroundColor Red
    exit 1
}

$Arch = if ([System.Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }
$Asset = $Release.assets | Where-Object { $_.name -match "windows" -and $_.name -match "zip" }

if (-not $Asset) {
    Write-Host "No Windows binary found in the latest release." -ForegroundColor Red
    exit 1
}

$TempZip = "$env:TEMP\rewind.zip"
Write-Host "Downloading $($Asset.name)..."
Invoke-WebRequest -Uri $Asset.browser_download_url -OutFile $TempZip -UseBasicParsing

$InstallDir = "$env:USERPROFILE\.rewind\bin"
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

Write-Host "Extracting to $InstallDir..."
Expand-Archive -Path $TempZip -DestinationPath $InstallDir -Force
Remove-Item $TempZip

$ExePath = "$InstallDir\rewind.exe"
if (-not (Test-Path $ExePath)) {
    Write-Host "Extraction failed: rewind.exe not found." -ForegroundColor Red
    exit 1
}

# Add to PATH
$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if ($UserPath -notmatch [regex]::Escape($InstallDir)) {
    $NewPath = "$UserPath;$InstallDir"
    [Environment]::SetEnvironmentVariable("Path", $NewPath, [EnvironmentVariableTarget]::User)
    Write-Host "Added $InstallDir to your PATH." -ForegroundColor Yellow
}

Write-Host "Installation Complete! ✅" -ForegroundColor Green
Write-Host "You can now run 'rewind' in any new terminal." -ForegroundColor Green
Write-Host "Note: You MUST restart your current terminal to use the command." -ForegroundColor Yellow
