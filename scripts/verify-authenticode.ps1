<#
.SYNOPSIS
  Verifies an Authenticode signature against an expected certificate.

.DESCRIPTION
  Verifies that the file at -Path carries an Authenticode signature whose
  signer certificate either matches -ExpectedCertificatePath or the
  certificate pinned through EXPECTED_CERTIFICATE_SUBJECT,
  EXPECTED_CERTIFICATE_ISSUER and EXPECTED_CERTIFICATE_SHA1. With
  -RequireTimestamp, also requires a trusted timestamp and a signature status
  of 'Valid'. On success the verified path is appended to the Job Summary.

.PARAMETER Path
  Literal path of the signed file to verify.

.PARAMETER RequireTimestamp
  Also require the signature to carry a trusted timestamp and have status 'Valid'.

.PARAMETER ExpectedCertificatePath
  Optional public certificate file whose Subject, Issuer and thumbprint must
  match the signer. When omitted, the pinned certificate environment variables
  are used for SignPath-signed EXE and NSIS artifacts.

.PARAMETER SigntoolVerify
  Also run signtool.exe verify /pa /all /v against the file.
#>
param(
  [Parameter(Mandatory = $true)][string]$Path,
  [switch]$RequireTimestamp,
  [string]$ExpectedCertificatePath,
  [switch]$SigntoolVerify
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
  throw "Signed file was not found: $Path"
}

if (-not [string]::IsNullOrWhiteSpace($ExpectedCertificatePath)) {
  if (-not (Test-Path -LiteralPath $ExpectedCertificatePath -PathType Leaf)) {
    throw "Expected certificate was not found: $ExpectedCertificatePath"
  }
  $expectedCertificate = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new(
    (Resolve-Path -LiteralPath $ExpectedCertificatePath).Path
  )
  $expectedSubject = $expectedCertificate.Subject
  $expectedIssuer = $expectedCertificate.Issuer
  $expectedThumbprint = ($expectedCertificate.Thumbprint -replace '\s', '').ToUpperInvariant()
} else {
  if ([string]::IsNullOrWhiteSpace($env:EXPECTED_CERTIFICATE_SUBJECT)) {
    throw 'SIGNPATH_CERTIFICATE_SUBJECT is not configured.'
  }
  if ([string]::IsNullOrWhiteSpace($env:EXPECTED_CERTIFICATE_ISSUER)) {
    throw 'SIGNPATH_CERTIFICATE_ISSUER is not configured.'
  }

  $expectedSubject = $env:EXPECTED_CERTIFICATE_SUBJECT
  $expectedIssuer = $env:EXPECTED_CERTIFICATE_ISSUER
  $expectedThumbprint = ($env:EXPECTED_CERTIFICATE_SHA1 -replace '\s', '').ToUpperInvariant()
  if ($expectedThumbprint -notmatch '^[0-9A-F]{40}$') {
    throw 'SIGNPATH_CERTIFICATE_SHA1 must contain a 40-character SHA-1 certificate thumbprint.'
  }
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
if ($certificate.Subject -ne $expectedSubject) {
  throw "Unexpected certificate subject: $($certificate.Subject)"
}
if ($certificate.Issuer -ne $expectedIssuer) {
  throw "Unexpected certificate issuer: $($certificate.Issuer)"
}

if ($RequireTimestamp) {
  if (-not $signature.TimeStamperCertificate -or $signature.Status -ne 'Valid') {
    throw "The signature is not valid and timestamped: $($signature.Status): $($signature.StatusMessage)"
  }
}

if ($SigntoolVerify) {
  $signToolCommand = Get-Command signtool.exe -ErrorAction SilentlyContinue
  if ($signToolCommand) {
    $signToolPath = $signToolCommand.Source
  } else {
    $kitsRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
    $candidate = Get-ChildItem -Path $kitsRoot -Filter signtool.exe -File -Recurse |
      Where-Object { $_.FullName -match '[\\/]x64[\\/]signtool\.exe$' } |
      Sort-Object FullName -Descending |
      Select-Object -First 1
    if (-not $candidate) {
      throw 'signtool.exe was not found on the runner.'
    }
    $signToolPath = $candidate.FullName
  }

  & $signToolPath verify /pa /all /v $Path
  if ($LASTEXITCODE -ne 0) {
    throw "signtool verification failed for $Path with exit code $LASTEXITCODE."
  }
}

"- $Path : signature verified" >> $env:GITHUB_STEP_SUMMARY
