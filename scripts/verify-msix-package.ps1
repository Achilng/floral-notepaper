<#
.SYNOPSIS
  Verifies a signed MSIX package: pinned Authenticode signature, then
  AppxManifest identity checks.

.DESCRIPTION
  Runs scripts/verify-authenticode.ps1 with -RequireTimestamp against the
  MSIX package, then reads AppxManifest.xml from the package and verifies the
  identity Name / Publisher / Version against the MSIX_IDENTITY_NAME and
  MSIX_PUBLISHER_CN environment variables and the -Version parameter, and the
  ProcessorArchitecture against -Arch (aarch64 maps to arm64). Verification
  results are appended to $env:GITHUB_STEP_SUMMARY.

.PARAMETER MsixPath
  Literal path of the signed MSIX package.

.PARAMETER Arch
  Architecture label used in the package file name: x64 or aarch64.

.PARAMETER Version
  Release version in MAJOR.MINOR.PATCH form; the manifest version must be
  MAJOR.MINOR.PATCH.0.

.PARAMETER SigntoolVerify
  Also verify the package with signtool.exe verify /pa /all /v.
#>
param(
  [Parameter(Mandatory = $true)][string]$MsixPath,
  [Parameter(Mandatory = $true)][ValidateSet('x64', 'aarch64')][string]$Arch,
  [Parameter(Mandatory = $true)][string]$Version,
  [switch]$SigntoolVerify
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem

if (-not (Test-Path -LiteralPath $MsixPath -PathType Leaf)) {
  throw "Signed MSIX package was not found at $MsixPath."
}

& (Join-Path $PSScriptRoot 'verify-authenticode.ps1') `
  -Path $MsixPath `
  -RequireTimestamp `
  -SigntoolVerify:$SigntoolVerify

$zip = [System.IO.Compression.ZipFile]::OpenRead((Resolve-Path -LiteralPath $MsixPath).Path)
try {
  $manifestEntry = $zip.GetEntry('AppxManifest.xml')
  if (-not $manifestEntry) {
    throw 'AppxManifest.xml was not found in the MSIX.'
  }
  $reader = New-Object System.IO.StreamReader($manifestEntry.Open())
  try {
    [xml]$manifest = $reader.ReadToEnd()
  } finally {
    $reader.Dispose()
  }
} finally {
  $zip.Dispose()
}

$identity = $manifest.Package.Identity
if ($identity.Name -ne $env:MSIX_IDENTITY_NAME) {
  throw "Unexpected identity Name in MSIX: $($identity.Name)"
}
if ($identity.Publisher -ne $env:MSIX_PUBLISHER_CN) {
  throw "Unexpected identity Publisher in MSIX: $($identity.Publisher)"
}
if ($identity.Version -ne "$($Version).0") {
  throw "Unexpected identity Version in MSIX: $($identity.Version)"
}
# File names use "aarch64"; AppxManifest ProcessorArchitecture stays "arm64"
$manifestArch = if ($Arch -eq 'aarch64') { 'arm64' } else { $Arch }
if ($identity.ProcessorArchitecture -ne $manifestArch) {
  throw "Unexpected ProcessorArchitecture in MSIX: $($identity.ProcessorArchitecture)"
}

"### MSIX verification ($Arch)" >> $env:GITHUB_STEP_SUMMARY
"- Package signature: valid, pinned, timestamped" >> $env:GITHUB_STEP_SUMMARY
"- Identity: $($identity.Name) / $($identity.Publisher) / $($identity.Version) / $($identity.ProcessorArchitecture)" >> $env:GITHUB_STEP_SUMMARY
