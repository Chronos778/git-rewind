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
$Asset = $Release.assets | Where-Object { $_.name -match "windows" -and $_.name -match '\.zip$' -and $_.name -match $Arch } | Select-Object -First 1

if (-not $Asset) {
    Write-Host "No Windows binary found in the latest release." -ForegroundColor Red
    exit 1
}

$TempZip = "$env:TEMP\rewind.zip"
Write-Host "Downloading $($Asset.name)..."
Invoke-WebRequest -Uri $Asset.browser_download_url -OutFile $TempZip -UseBasicParsing

$ChecksumUrl = $Asset.browser_download_url -replace '\.zip$', '.sha256'
$TempChecksum = "$env:TEMP\rewind.zip.sha256"
Write-Host "Downloading checksum..."
try {
    Invoke-WebRequest -Uri $ChecksumUrl -OutFile $TempChecksum -UseBasicParsing
    $ExpectedChecksum = (Get-Content $TempChecksum -TotalCount 1).Split(' ')[0].Trim().ToUpper()
    $ActualChecksum = (Get-FileHash -Path $TempZip -Algorithm SHA256).Hash.ToUpper()

    if ($ExpectedChecksum -ne $ActualChecksum) {
        Write-Host "Error: Checksum verification failed!" -ForegroundColor Red
        Write-Host "Expected: $ExpectedChecksum" -ForegroundColor Red
        Write-Host "Actual:   $ActualChecksum" -ForegroundColor Red
        Remove-Item $TempZip, $TempChecksum -ErrorAction SilentlyContinue
        exit 1
    }
    Write-Host "Checksum verified successfully." -ForegroundColor Green
    Remove-Item $TempChecksum -ErrorAction SilentlyContinue
} catch {
    Write-Host "Warning: Could not download checksum file. Skipping verification." -ForegroundColor Yellow
}

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

Write-Host "[SUCCESS] Installation Complete!" -ForegroundColor Green
Write-Host "You can now run 'rewind' in any new terminal." -ForegroundColor Green
Write-Host "Note: You MUST restart your current terminal to use the command." -ForegroundColor Yellow
