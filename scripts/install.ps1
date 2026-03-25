$ErrorActionPreference = "Stop"

$Repo = if ($env:MPA_REPO) { $env:MPA_REPO } else { "shijianzhong/multi-platform-articles" }
$Version = $env:MPA_VERSION
$InstallDir = if ($env:MPA_INSTALL_DIR) { $env:MPA_INSTALL_DIR } else { Join-Path $env:USERPROFILE ".local\\bin" }

if (-not $Version) {
  Write-Host "MPA_VERSION not set, fetching latest release version..."
  $ReleaseUrl = "https://api.github.com/repos/$Repo/releases/latest"
  try {
    $ReleaseData = Invoke-RestMethod -Uri $ReleaseUrl -UseBasicParsing
    $Version = $ReleaseData.tag_name
  } catch {
    Write-Error "Failed to fetch latest version"
    exit 2
  }
  Write-Host "Latest version: $Version"
}

$Target = "x86_64-pc-windows-msvc"
$Name = "mpa"
$Asset = "$Name-$Version-$Target.zip"
$Url = "https://github.com/$Repo/releases/download/$Version/$Asset"

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$Tmp = New-Item -ItemType Directory -Force -Path (Join-Path $env:TEMP ("mpa-install-" + [guid]::NewGuid().ToString()))

try {
  Write-Host "Downloading $Url"
  $ZipPath = Join-Path $Tmp.FullName $Asset
  Invoke-WebRequest -Uri $Url -OutFile $ZipPath

  Expand-Archive -Path $ZipPath -DestinationPath $Tmp.FullName -Force

  $Bin = Get-ChildItem -Path $Tmp.FullName -Recurse -Filter "mpa.exe" | Select-Object -First 1
  if (-not $Bin) {
    Write-Error "mpa.exe not found in archive"
    exit 2
  }

  $BinDir = Split-Path $Bin.FullName
  Set-Location -Path $BinDir
  Write-Host "Running mpa install command..."
  & $Bin.FullName install

  Write-Host "Installation complete!"
  Write-Host "Run: mpa themes list"
  Write-Host "Config: run 'mpa' to open TUI and set WECHAT_APPID/WECHAT_SECRET"
} finally {
  Remove-Item -Recurse -Force $Tmp.FullName | Out-Null
}
