<#
.SYNOPSIS
  Builds an MSIX package for 花笺 (Floral Notepaper) from a signed executable.

.DESCRIPTION
  Renders AppxManifest.xml from src-tauri/msix/AppxManifest.template.xml,
  assembles the MSIX layout (manifest + signed exe + Store icons), generates
  resources.pri with MakePri.exe from the lang-*/resources.resw files (the
  manifest DisplayName / Description resolve via ms-resource references),
  packs it with MakeAppx.exe and unpacks the result to verify that the
  manifest identity, version, processor architecture, resources.pri and the
  embedded executable hash match the inputs.

.PARAMETER Version
  Release version in MAJOR.MINOR.PATCH form. The manifest version becomes
  MAJOR.MINOR.PATCH.0.

.PARAMETER IdentityName
  MSIX identity Name. For Store submissions this must equal the product name
  reserved in Partner Center.

.PARAMETER PublisherCN
  Certificate subject CN used as the manifest Publisher, e.g. "CN=Example Inc.".
  Must match the Partner Center account publisher for Store submissions.

.PARAMETER PublisherDisplayName
  Publisher display name written to the manifest Properties.

.PARAMETER Arch
  Architecture label used in the output file name: x64 or aarch64.
  The AppxManifest ProcessorArchitecture is derived from it (aarch64 is
  written as arm64, the only value Windows manifests accept).

.PARAMETER BinaryPath
  Path to the signed floral-notepaper.exe to embed.

.PARAMETER IconsDir
  Directory containing the Square*Logo.png and StoreLogo.png assets
  (normally src-tauri/icons).

.PARAMETER OutputDir
  Directory where floral-notepaper_<version>_<arch>.msix is written.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/build-msix.ps1 `
    -Version 1.1.0 -IdentityName FloralNotepaper `
    -PublisherCN "CN=Example Inc." -PublisherDisplayName "Example Inc." `
    -Arch x64 -BinaryPath signed/floral-notepaper.exe `
    -IconsDir src-tauri/icons -OutputDir msix-out
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$Version,
  [Parameter(Mandatory = $true)][string]$IdentityName,
  [Parameter(Mandatory = $true)][string]$PublisherCN,
  [Parameter(Mandatory = $true)][string]$PublisherDisplayName,
  [Parameter(Mandatory = $true)][ValidateSet('x64', 'aarch64')][string]$Arch,
  [Parameter(Mandatory = $true)][string]$BinaryPath,
  [Parameter(Mandatory = $true)][string]$IconsDir,
  [Parameter(Mandatory = $true)][string]$OutputDir
)

$ErrorActionPreference = 'Stop'

function Get-MakeAppxPath {
  $command = Get-Command makeappx.exe -ErrorAction SilentlyContinue
  if ($command) {
    return $command.Source
  }

  $kitsRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
  $candidate = Get-ChildItem -Path $kitsRoot -Filter makeappx.exe -File -Recurse |
    Where-Object { $_.FullName -match '[\\/]x64[\\/]makeappx\.exe$' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
  if (-not $candidate) {
    throw 'makeappx.exe was not found on the runner.'
  }
  return $candidate.FullName
}

function Get-MakePriPath {
  $command = Get-Command makepri.exe -ErrorAction SilentlyContinue
  if ($command) {
    return $command.Source
  }

  $kitsRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
  $candidate = Get-ChildItem -Path $kitsRoot -Filter makepri.exe -File -Recurse |
    Where-Object { $_.FullName -match '[\\/]x64[\\/]makepri\.exe$' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
  if (-not $candidate) {
    throw 'makepri.exe was not found on the runner.'
  }
  return $candidate.FullName
}

function Assert-ManifestPlaceholdersFilled {
  param([Parameter(Mandatory = $true)][string]$Content)

  $remaining = [regex]::Matches($Content, '__[A-Z_]+__') |
    ForEach-Object { $_.Value } |
    Sort-Object -Unique
  if ($remaining.Count -ne 0) {
    throw "Unresolved AppxManifest placeholders: $($remaining -join ', ')"
  }
}

# --- Validate inputs -------------------------------------------------------

if ($Version -notmatch '^\d+\.\d+\.\d+$') {
  throw "Version must be MAJOR.MINOR.PATCH (stable SemVer), got: $Version"
}
$manifestVersion = "$Version.0"

if (-not (Test-Path -LiteralPath $BinaryPath -PathType Leaf)) {
  throw "Signed executable was not found: $BinaryPath"
}
if (-not (Test-Path -LiteralPath $IconsDir -PathType Container)) {
  throw "Icons directory was not found: $IconsDir"
}
if (-not (Test-Path -LiteralPath (Join-Path $IconsDir 'StoreLogo.png') -PathType Leaf)) {
  throw "StoreLogo.png was not found in $IconsDir"
}

$binarySha256 = (Get-FileHash -LiteralPath $BinaryPath -Algorithm SHA256).Hash
Write-Host "Embedding $BinaryPath (SHA-256 $binarySha256)"

# --- Render the manifest ---------------------------------------------------

$templatePath = Join-Path $PSScriptRoot '..\src-tauri\msix\AppxManifest.template.xml'
if (-not (Test-Path -LiteralPath $templatePath -PathType Leaf)) {
  throw "AppxManifest template was not found: $templatePath"
}

$manifest = Get-Content -LiteralPath $templatePath -Raw -Encoding UTF8
$manifest = $manifest.Replace('__VERSION__', $manifestVersion)
$manifest = $manifest.Replace('__IDENTITY_NAME__', $IdentityName)
$manifest = $manifest.Replace('__PUBLISHER_CN__', $PublisherCN)
$manifest = $manifest.Replace('__PUBLISHER_DISPLAY_NAME__', $PublisherDisplayName)
# The output file name uses "aarch64", but AppxManifest only accepts "arm64" here
$manifestArch = if ($Arch -eq 'aarch64') { 'arm64' } else { $Arch }
$manifest = $manifest.Replace('__ARCH__', $manifestArch)
Assert-ManifestPlaceholdersFilled -Content $manifest

# --- Assemble the layout ----------------------------------------------------

$layoutDir = Join-Path (Join-Path $env:TEMP 'floral-msix-layout') $Arch
if (Test-Path -LiteralPath $layoutDir) {
  Remove-Item -LiteralPath $layoutDir -Recurse -Force
}
New-Item -ItemType Directory -Path $layoutDir -Force | Out-Null

[System.IO.File]::WriteAllText(
  (Join-Path $layoutDir 'AppxManifest.xml'),
  $manifest,
  (New-Object System.Text.UTF8Encoding($false))
)

Copy-Item -LiteralPath $BinaryPath -Destination (Join-Path $layoutDir 'floral-notepaper.exe') -Force

$logoAssets = @(
  'StoreLogo.png',
  'Square30x30Logo.png',
  'Square44x44Logo.png',
  'Square71x71Logo.png',
  'Square89x89Logo.png',
  'Square107x107Logo.png',
  'Square142x142Logo.png',
  'Square150x150Logo.png',
  'Square284x284Logo.png',
  'Square310x310Logo.png'
)
foreach ($asset in $logoAssets) {
  $source = Join-Path $IconsDir $asset
  if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
    throw "Required MSIX logo asset is missing: $source"
  }
  Copy-Item -LiteralPath $source -Destination (Join-Path $layoutDir $asset) -Force
}

# --- Generate resources.pri --------------------------------------------------

$msixDir = Join-Path $PSScriptRoot '..\src-tauri\msix'
$resRoot = Join-Path (Join-Path $env:TEMP 'floral-msix-res') $Arch
if (Test-Path -LiteralPath $resRoot) {
  Remove-Item -LiteralPath $resRoot -Recurse -Force
}
New-Item -ItemType Directory -Path $resRoot -Force | Out-Null

$resLanguages = @('lang-zh-CN', 'lang-en-US', 'lang-zh-HK')
foreach ($langDir in $resLanguages) {
  $source = Join-Path (Join-Path $msixDir $langDir) 'resources.resw'
  if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
    throw "Required MSIX resource file is missing: $source"
  }
  Copy-Item -LiteralPath $source -Destination (Join-Path $resRoot $langDir) -Force
}

$makePri = Get-MakePriPath
Write-Host "Using MakePri at $makePri"

$priConfig = Join-Path $resRoot 'priconfig.xml'
& $makePri createconfig /cf $priConfig /dq zh-CN /o
if ($LASTEXITCODE -ne 0) {
  throw "makepri createconfig failed with exit code $LASTEXITCODE."
}

$layoutPri = Join-Path $layoutDir 'resources.pri'
& $makePri new /pr $resRoot /cf $priConfig /of $layoutPri /o
if ($LASTEXITCODE -ne 0) {
  throw "makepri new failed with exit code $LASTEXITCODE."
}
if (-not (Test-Path -LiteralPath $layoutPri -PathType Leaf)) {
  throw "resources.pri was not produced: $layoutPri"
}

# --- Pack -------------------------------------------------------------------

$makeAppx = Get-MakeAppxPath
Write-Host "Using MakeAppx at $makeAppx"

New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
$outputPath = Join-Path $OutputDir "floral-notepaper_${Version}_${Arch}.msix"
if (Test-Path -LiteralPath $outputPath) {
  Remove-Item -LiteralPath $outputPath -Force
}

& $makeAppx pack /d $layoutDir /p $outputPath /o /l
if ($LASTEXITCODE -ne 0) {
  throw "makeappx pack failed with exit code $LASTEXITCODE."
}
if (-not (Test-Path -LiteralPath $outputPath -PathType Leaf)) {
  throw "MSIX package was not produced: $outputPath"
}

# --- Unpack and verify ------------------------------------------------------

$verifyDir = Join-Path (Join-Path $env:TEMP 'floral-msix-verify') $Arch
if (Test-Path -LiteralPath $verifyDir) {
  Remove-Item -LiteralPath $verifyDir -Recurse -Force
}
New-Item -ItemType Directory -Path $verifyDir -Force | Out-Null

& $makeAppx unpack /p $outputPath /d $verifyDir /o
if ($LASTEXITCODE -ne 0) {
  throw "makeappx unpack verification failed with exit code $LASTEXITCODE."
}

$manifestPath = Join-Path $verifyDir 'AppxManifest.xml'
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
  throw 'AppxManifest.xml was not found in the packed package.'
}

$packedPriPath = Join-Path $verifyDir 'resources.pri'
if (-not (Test-Path -LiteralPath $packedPriPath -PathType Leaf)) {
  throw 'resources.pri was not found in the packed package.'
}

[xml]$packedManifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8
$identity = $packedManifest.Package.Identity
if ($identity.Name -ne $IdentityName) {
  throw "Unexpected identity Name in packed manifest: $($identity.Name)"
}
if ($identity.Publisher -ne $PublisherCN) {
  throw "Unexpected identity Publisher in packed manifest: $($identity.Publisher)"
}
if ($identity.Version -ne $manifestVersion) {
  throw "Unexpected identity Version in packed manifest: $($identity.Version), expected $manifestVersion"
}
if ($identity.ProcessorArchitecture -ne $manifestArch) {
  throw "Unexpected ProcessorArchitecture in packed manifest: $($identity.ProcessorArchitecture), expected $manifestArch"
}

$packedBinaryPath = Join-Path $verifyDir 'floral-notepaper.exe'
if (-not (Test-Path -LiteralPath $packedBinaryPath -PathType Leaf)) {
  throw 'floral-notepaper.exe was not found in the packed package.'
}
$packedSha256 = (Get-FileHash -LiteralPath $packedBinaryPath -Algorithm SHA256).Hash
if ($packedSha256 -ne $binarySha256) {
  throw "The executable changed while packing. Expected $binarySha256, got $packedSha256."
}

Write-Host "MSIX package verified: $outputPath"
Write-Host "  Identity: $($identity.Name) / $($identity.Publisher) / $($identity.Version) / $($identity.ProcessorArchitecture)"
Write-Host "  Embedded executable SHA-256: $packedSha256"
