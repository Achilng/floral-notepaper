<#
.SYNOPSIS
  Verifies a self-signed MSIX package and its AppxManifest identity.

.DESCRIPTION
  Verifies the package signature against the supplied public certificate,
  requires a trusted timestamp, then reads AppxManifest.xml and verifies its
  identity Name / Publisher / Version and ProcessorArchitecture. The signer
  certificate Subject must exactly equal the manifest Publisher.

.PARAMETER MsixPath
  Literal path of the signed MSIX package.

.PARAMETER Arch
  Architecture label used in the package file name: x64 or aarch64.

.PARAMETER Version
  Release version in MAJOR.MINOR.PATCH form; the manifest version must be
  MAJOR.MINOR.PATCH.0.

.PARAMETER CertificatePath
  Public certificate exported by scripts/sign-msix-self-signed.ps1.

.PARAMETER ExpectedCertificateSha256
  Pinned SHA-256 fingerprint of the public certificate, without separators.

.PARAMETER SigntoolVerify
  Also verify the package with signtool.exe verify /pa /all /v.
#>
param(
  [Parameter(Mandatory = $true)][string]$MsixPath,
  [Parameter(Mandatory = $true)][ValidateSet('x64', 'aarch64')][string]$Arch,
  [Parameter(Mandatory = $true)][string]$Version,
  [Parameter(Mandatory = $true)][string]$CertificatePath,
  [Parameter(Mandatory = $true)][string]$ExpectedCertificateSha256,
  [switch]$SigntoolVerify
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem

if (-not (Test-Path -LiteralPath $MsixPath -PathType Leaf)) {
  throw "Signed MSIX package was not found at $MsixPath."
}
if (-not (Test-Path -LiteralPath $CertificatePath -PathType Leaf)) {
  throw "MSIX signing certificate was not found at $CertificatePath."
}
$expectedCertificateSha256 = ($ExpectedCertificateSha256 -replace '\s|:', '').ToUpperInvariant()
if ($expectedCertificateSha256 -notmatch '^[0-9A-F]{64}$') {
  throw 'ExpectedCertificateSha256 must contain a 64-character SHA-256 certificate fingerprint.'
}
$actualCertificateSha256 = (Get-FileHash -LiteralPath $CertificatePath -Algorithm SHA256).Hash.ToUpperInvariant()
if ($actualCertificateSha256 -ne $expectedCertificateSha256) {
  throw "Unexpected MSIX signing certificate SHA-256 fingerprint: $actualCertificateSha256"
}

& (Join-Path $PSScriptRoot 'verify-authenticode.ps1') `
  -Path $MsixPath `
  -RequireTimestamp `
  -ExpectedCertificatePath $CertificatePath `
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
$signingCertificate = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new(
  (Resolve-Path -LiteralPath $CertificatePath).Path
)
if ($identity.Publisher -ne $signingCertificate.Subject) {
  throw "MSIX Publisher does not match the signing certificate Subject: $($identity.Publisher) != $($signingCertificate.Subject)"
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
"- Package signature: valid, self-signed certificate matched, timestamped" >> $env:GITHUB_STEP_SUMMARY
"- Certificate thumbprint: $($signingCertificate.Thumbprint)" >> $env:GITHUB_STEP_SUMMARY
"- Certificate SHA-256: $actualCertificateSha256" >> $env:GITHUB_STEP_SUMMARY
"- Identity: $($identity.Name) / $($identity.Publisher) / $($identity.Version) / $($identity.ProcessorArchitecture)" >> $env:GITHUB_STEP_SUMMARY
