<#
.SYNOPSIS
  Signs one or more MSIX packages with the repository's self-signed certificate.

.DESCRIPTION
  Imports a password-protected PFX supplied by GitHub Secrets, verifies that
  its Subject exactly matches the AppxManifest Publisher, exports the public
  certificate, trusts it for the ephemeral CI machine, and signs every supplied
  MSIX with SHA-256 and an RFC 3161 timestamp. Reusing the same PFX keeps the
  MSIX signing identity stable across Release and Rebuild runs.

.PARAMETER MsixPaths
  One or more unsigned MSIX packages to sign with the same certificate.

.PARAMETER Publisher
  Exact AppxManifest Publisher and certificate Subject, for example
  "CN=01234567-89AB-CDEF-0123-456789ABCDEF".

.PARAMETER PfxPath
  Path to the stable self-signed code-signing PFX reconstructed from a GitHub
  Actions secret.

.PARAMETER PfxPassword
  Password protecting the PFX.

.PARAMETER ExpectedCertificateSha256
  Pinned SHA-256 fingerprint of the public certificate, without separators.

.PARAMETER CertificateOutputPath
  Destination for the exported public DER certificate (.cer).
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string[]]$MsixPaths,
  [Parameter(Mandatory = $true)][string]$Publisher,
  [Parameter(Mandatory = $true)][string]$PfxPath,
  [Parameter(Mandatory = $true)][string]$PfxPassword,
  [Parameter(Mandatory = $true)][string]$ExpectedCertificateSha256,
  [Parameter(Mandatory = $true)][string]$CertificateOutputPath
)

$ErrorActionPreference = 'Stop'

function Get-SignToolPath {
  $command = Get-Command signtool.exe -ErrorAction SilentlyContinue
  if ($command) {
    return $command.Source
  }

  $kitsRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
  $candidate = Get-ChildItem -Path $kitsRoot -Filter signtool.exe -File -Recurse |
    Where-Object { $_.FullName -match '[\\/]x64[\\/]signtool\.exe$' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
  if (-not $candidate) {
    throw 'signtool.exe was not found on the runner.'
  }
  return $candidate.FullName
}

if ([string]::IsNullOrWhiteSpace($Publisher)) {
  throw 'Publisher must not be empty.'
}
if ($MsixPaths.Count -lt 1) {
  throw 'At least one MSIX path is required.'
}
foreach ($path in $MsixPaths) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "Unsigned MSIX package was not found: $path"
  }
  if ([System.IO.Path]::GetExtension($path) -ine '.msix') {
    throw "Expected an .msix file, got: $path"
  }
}
if (-not (Test-Path -LiteralPath $PfxPath -PathType Leaf)) {
  throw "MSIX signing PFX was not found: $PfxPath"
}
if ([string]::IsNullOrWhiteSpace($PfxPassword)) {
  throw 'MSIX signing PFX password must not be empty.'
}

$securePassword = ConvertTo-SecureString -String $PfxPassword -AsPlainText -Force
$importedCertificates = @(
  Import-PfxCertificate `
    -FilePath $PfxPath `
    -CertStoreLocation 'Cert:\CurrentUser\My' `
    -Password $securePassword
)
$certificate = @(
  $importedCertificates |
    Where-Object { $_.HasPrivateKey -and $_.Subject -eq $Publisher }
)
if ($certificate.Count -ne 1) {
  throw "Expected exactly one private-key certificate with Subject '$Publisher' in the PFX, found $($certificate.Count)."
}
$certificate = $certificate[0]

if ($certificate.Issuer -ne $certificate.Subject) {
  throw "MSIX signing certificate is not self-signed: $($certificate.Issuer)"
}
$codeSigningOid = '1.3.6.1.5.5.7.3.3'
$hasCodeSigningEku = @($certificate.Extensions |
  Where-Object { $_.Oid.Value -eq '2.5.29.37' } |
  ForEach-Object { $_.EnhancedKeyUsages } |
  Where-Object { $_.Value -eq $codeSigningOid }).Count -gt 0
if (-not $hasCodeSigningEku) {
  throw 'MSIX signing certificate does not include the Code Signing enhanced key usage.'
}

$certificateDirectory = Split-Path -Parent $CertificateOutputPath
if (-not [string]::IsNullOrWhiteSpace($certificateDirectory)) {
  New-Item -ItemType Directory -Path $certificateDirectory -Force | Out-Null
}
Export-Certificate -Cert $certificate -FilePath $CertificateOutputPath -Force | Out-Null
$expectedSha256 = ($ExpectedCertificateSha256 -replace '\s|:', '').ToUpperInvariant()
if ($expectedSha256 -notmatch '^[0-9A-F]{64}$') {
  throw 'ExpectedCertificateSha256 must contain a 64-character SHA-256 certificate fingerprint.'
}
$actualSha256 = (Get-FileHash -LiteralPath $CertificateOutputPath -Algorithm SHA256).Hash.ToUpperInvariant()
if ($actualSha256 -ne $expectedSha256) {
  throw "Unexpected MSIX signing certificate SHA-256 fingerprint: $actualSha256"
}
Import-Certificate `
  -FilePath $CertificateOutputPath `
  -CertStoreLocation 'Cert:\LocalMachine\TrustedPeople' | Out-Null

$signTool = Get-SignToolPath
foreach ($path in $MsixPaths) {
  & $signTool sign `
    /fd SHA256 `
    /sha1 $certificate.Thumbprint `
    /s My `
    /tr http://timestamp.digicert.com `
    /td SHA256 `
    $path
  if ($LASTEXITCODE -ne 0) {
    throw "signtool signing failed for $path with exit code $LASTEXITCODE."
  }
}

Write-Host 'MSIX packages signed with the repository self-signed certificate.'
Write-Host "  Subject: $($certificate.Subject)"
Write-Host "  Thumbprint: $($certificate.Thumbprint)"
Write-Host "  SHA-256 fingerprint: $actualSha256"
Write-Host "  Public certificate: $CertificateOutputPath"
