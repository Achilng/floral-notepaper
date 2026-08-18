<#
.SYNOPSIS
  Builds an .msixupload container from the signed per-architecture MSIX
  packages (x64 + AArch64) for a single Microsoft Store submission.

.DESCRIPTION
  The .msixupload file is a plain ZIP container holding one or more signed
  .msix packages; Partner Center expands it into the packages of the
  submission. This is the same shape Visual Studio produces when publishing
  multi-architecture MSIX apps.

  The container itself is unsigned: only the inner .msix packages carry
  signatures.

.PARAMETER Version
  Release version in MAJOR.MINOR.PATCH form, used for the output file name.

.PARAMETER MsixPaths
  One or more paths to signed .msix files. Exactly the file names listed here
  must end up at the root of the upload container.

.PARAMETER OutputDir
  Directory where floral-notepaper_<version>.msixupload is written.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/build-msixupload.ps1 `
    -Version 1.1.0 `
    -MsixPaths signed-x64/floral-notepaper_1.1.0_x64.msix, signed-arm64/floral-notepaper_1.1.0_arm64.msix `
    -OutputDir upload-out
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$Version,
  [Parameter(Mandatory = $true)][string[]]$MsixPaths,
  [Parameter(Mandatory = $true)][string]$OutputDir
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

if ($Version -notmatch '^\d+\.\d+\.\d+$') {
  throw "Version must be MAJOR.MINOR.PATCH (stable SemVer), got: $Version"
}
if ($MsixPaths.Count -lt 1) {
  throw 'At least one .msix path is required.'
}

# --- Validate inputs --------------------------------------------------------

$sourceFiles = @()
foreach ($path in $MsixPaths) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "MSIX package was not found: $path"
  }
  if ([System.IO.Path]::GetExtension($path) -ine '.msix') {
    throw "Expected a .msix file, got: $path"
  }

  $signature = Get-AuthenticodeSignature -LiteralPath $path
  if ($signature.Status -ne 'Valid') {
    throw "MSIX package is not validly signed: $path ($($signature.Status): $($signature.StatusMessage))"
  }

  $zip = [System.IO.Compression.ZipFile]::OpenRead((Resolve-Path -LiteralPath $path).Path)
  try {
    $hasManifest = $zip.Entries |
      Where-Object { $_.FullName -eq 'AppxManifest.xml' } |
      Select-Object -First 1
    if (-not $hasManifest) {
      throw "MSIX package has no root AppxManifest.xml: $path"
    }
  } finally {
    $zip.Dispose()
  }

  $sourceFiles += Get-Item -LiteralPath $path
}

# --- Stage and compress -----------------------------------------------------

$stagingDir = Join-Path (Join-Path $env:TEMP 'floral-msixupload-stage') ([guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stagingDir -Force | Out-Null
try {
  foreach ($file in $sourceFiles) {
    Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $stagingDir $file.Name) -Force
  }

  New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
  $uploadPath = Join-Path $OutputDir "floral-notepaper_${Version}.msixupload"
  if (Test-Path -LiteralPath $uploadPath) {
    Remove-Item -LiteralPath $uploadPath -Force
  }

  $stagePattern = Join-Path $stagingDir '*.msix'
  Compress-Archive -Path $stagePattern -DestinationPath $uploadPath -CompressionLevel Optimal

  # --- Verify the container ---------------------------------------------------

  $expectedNames = @($sourceFiles | ForEach-Object Name | Sort-Object)
  $zip = [System.IO.Compression.ZipFile]::OpenRead((Resolve-Path -LiteralPath $uploadPath).Path)
  try {
    $entries = @($zip.Entries | Where-Object { -not $_.FullName.EndsWith('/') })
    $actualNames = @($entries | ForEach-Object { $_.FullName } | Sort-Object)
    if ($actualNames.Count -ne $expectedNames.Count) {
      throw "Unexpected entry count in $uploadPath : $($actualNames -join ', ')"
    }
    for ($i = 0; $i -lt $expectedNames.Count; $i++) {
      if ($actualNames[$i] -ne $expectedNames[$i]) {
        throw "Unexpected entry in $uploadPath : $($actualNames[$i])"
      }
    }
  } finally {
    $zip.Dispose()
  }

  Write-Host "MSIX upload container verified: $uploadPath"
  Write-Host "  Entries: $($expectedNames -join ', ')"
} finally {
  if (Test-Path -LiteralPath $stagingDir) {
    Remove-Item -LiteralPath $stagingDir -Recurse -Force
  }
}
