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

function Assert-MsixRuntimeManifest {
  param(
    [Parameter(Mandatory = $true)][xml]$Document,
    [Parameter(Mandatory = $true)][string]$RawContent
  )

  foreach ($forbidden in @(
    'unvirtualizedResources',
    'RegistryWriteVirtualization',
    'FileSystemWriteVirtualization',
    'xmlns:desktop6='
  )) {
    if ($RawContent.Contains($forbidden, [System.StringComparison]::OrdinalIgnoreCase)) {
      throw "MSIX manifest contains forbidden virtualization declaration: $forbidden"
    }
  }

  $namespaces = New-Object System.Xml.XmlNamespaceManager($Document.NameTable)
  $namespaces.AddNamespace('f', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10')
  $namespaces.AddNamespace('desktop', 'http://schemas.microsoft.com/appx/manifest/desktop/windows10')
  $namespaces.AddNamespace('rescap', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities')

  $targetFamily = $Document.SelectSingleNode(
    '/f:Package/f:Dependencies/f:TargetDeviceFamily[@Name="Windows.Desktop"]',
    $namespaces
  )
  if (-not $targetFamily -or $targetFamily.MinVersion -ne '10.0.19041.0') {
    throw "MSIX Windows.Desktop MinVersion must be 10.0.19041.0, got: $($targetFamily.MinVersion)"
  }
  if (-not $Document.SelectSingleNode(
      '/f:Package/f:Capabilities/rescap:Capability[@Name="runFullTrust"]',
      $namespaces
    )) {
    throw 'MSIX manifest must retain the runFullTrust capability.'
  }

  $fullTrustProcess = $Document.SelectSingleNode(
    '/f:Package/f:Applications/f:Application/f:Extensions/desktop:Extension[@Category="windows.fullTrustProcess"]',
    $namespaces
  )
  if (-not $fullTrustProcess -or $fullTrustProcess.Executable -ne 'floral-notepaper.exe') {
    throw 'MSIX manifest must retain the floral-notepaper.exe fullTrustProcess extension.'
  }

  $startupExtension = $Document.SelectSingleNode(
    '/f:Package/f:Applications/f:Application/f:Extensions/desktop:Extension[@Category="windows.startupTask"]',
    $namespaces
  )
  if (-not $startupExtension -or
      $startupExtension.Executable -ne 'floral-notepaper.exe' -or
      $startupExtension.EntryPoint -ne 'Windows.FullTrustApplication') {
    throw 'MSIX manifest StartupTask extension is missing or invalid.'
  }
  $parameters = $startupExtension.GetAttribute(
    'Parameters',
    'http://schemas.microsoft.com/appx/manifest/uap/windows10/10'
  )
  if ($parameters -ne '--silent') {
    throw "MSIX StartupTask parameters must be --silent, got: $parameters"
  }
  $startupTask = $startupExtension.SelectSingleNode('desktop:StartupTask', $namespaces)
  if (-not $startupTask -or
      $startupTask.TaskId -ne 'FloralNotepaperStartup' -or
      $startupTask.Enabled -ne 'false' -or
      $startupTask.DisplayName -ne 'ms-resource:AppName') {
    throw 'MSIX StartupTask TaskId, default state, or DisplayName is incorrect.'
  }
}

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
    $manifestContent = $reader.ReadToEnd()
    [xml]$manifest = $manifestContent
  } finally {
    $reader.Dispose()
  }
} finally {
  $zip.Dispose()
}

Assert-MsixRuntimeManifest -Document $manifest -RawContent $manifestContent

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
"- Runtime manifest: no unvirtualizedResources; LocalState/StartupTask contract verified" >> $env:GITHUB_STEP_SUMMARY
