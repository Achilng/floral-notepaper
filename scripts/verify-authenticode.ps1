<#
.SYNOPSIS
  Verifies an Authenticode signature against the pinned SignPath certificate.

.DESCRIPTION
  Verifies that the file at -Path carries an Authenticode signature whose
  signer certificate matches the SignPath certificate pinned through the
  EXPECTED_CERTIFICATE_SUBJECT, EXPECTED_CERTIFICATE_ISSUER and
  EXPECTED_CERTIFICATE_SHA1 environment variables. With -RequireTimestamp,
  also requires a trusted timestamp and a signature status of 'Valid'. On
  success the verified path is appended to $env:GITHUB_STEP_SUMMARY.

.PARAMETER Path
  Literal path of the signed file to verify.

.PARAMETER RequireTimestamp
  Also require the signature to carry a trusted timestamp and have status 'Valid'.
#>
param(
  [Parameter(Mandatory = $true)][string]$Path,
  [switch]$RequireTimestamp
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($env:EXPECTED_CERTIFICATE_SUBJECT)) {
  throw 'SIGNPATH_CERTIFICATE_SUBJECT is not configured.'
}
if ([string]::IsNullOrWhiteSpace($env:EXPECTED_CERTIFICATE_ISSUER)) {
  throw 'SIGNPATH_CERTIFICATE_ISSUER is not configured.'
}

$expectedThumbprint = ($env:EXPECTED_CERTIFICATE_SHA1 -replace '\s', '').ToUpperInvariant()
if ($expectedThumbprint -notmatch '^[0-9A-F]{40}$') {
  throw 'SIGNPATH_CERTIFICATE_SHA1 must contain a 40-character SHA-1 certificate thumbprint.'
}

$signature = Get-AuthenticodeSignature -LiteralPath $Path
if (-not $signature.SignerCertificate) {
  throw "The Authenticode signature does not contain a signer certificate: $Path"
}

$certificate = $signature.SignerCertificate
$actualThumbprint = ($certificate.Thumbprint -replace '\s', '').ToUpperInvariant()
if ($actualThumbprint -ne $expectedThumbprint) {
  throw "Unexpected certificate thumbprint: $actualThumbprint"
}
if ($certificate.Subject -ne $env:EXPECTED_CERTIFICATE_SUBJECT) {
  throw "Unexpected certificate subject: $($certificate.Subject)"
}
if ($certificate.Issuer -ne $env:EXPECTED_CERTIFICATE_ISSUER) {
  throw "Unexpected certificate issuer: $($certificate.Issuer)"
}

if ($RequireTimestamp) {
  if (-not $signature.TimeStamperCertificate -or $signature.Status -ne 'Valid') {
    throw "The signature is not valid and timestamped: $($signature.Status): $($signature.StatusMessage)"
  }
}

"- $Path : signature verified" >> $env:GITHUB_STEP_SUMMARY
