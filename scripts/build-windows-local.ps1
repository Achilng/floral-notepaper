<#
.SYNOPSIS
  Builds unsigned Windows EXE, NSIS, MSIX and MSIXUPLOAD artifacts locally.

.DESCRIPTION
  PowerShell 7 entry point for reproducing the Windows packaging portion of
  the release workflow without SignPath or Microsoft Store credentials.

  By default the script builds both x64 and AArch64. For each architecture it:
  - installs the required Rust target when missing;
  - builds the Tauri executable and NSIS installer without code signing;
  - builds an unsigned MSIX from that executable;
  - combines all selected MSIX packages into an unsigned .msixupload archive.

  Unsigned MSIX/MSIXUPLOAD outputs are intended for package inspection or a
  later signing step. Windows normally will not install them until they are
  signed with a certificate matching the manifest Publisher.

.PARAMETER Architectures
  Architectures to build. Defaults to x64 and aarch64.

.PARAMETER Version
  Stable MAJOR.MINOR.PATCH version. Defaults to package.json and must match
  package.json, src-tauri/tauri.conf.json and src-tauri/Cargo.toml.

.PARAMETER OutputDir
  Artifact directory, relative to the repository root unless absolute.

.PARAMETER SkipNpmInstall
  Skips npm ci. Use only when node_modules already matches package-lock.json.

.PARAMETER SkipNsisToolchainPreparation
  Skips scripts/prepare-nsis-toolchain.ps1 and lets Tauri locate/download NSIS.

.PARAMETER Clean
  Removes OutputDir before building. Generated Cargo/Tauri target directories
  are not removed.

.EXAMPLE
  pwsh -File scripts/build-windows-local.ps1

.EXAMPLE
  pwsh -File scripts/build-windows-local.ps1 `
    -Architectures x64 `
    -SkipNpmInstall `
    -OutputDir local-build/windows-x64

#>
[CmdletBinding()]
param(
  [ValidateSet('x64', 'aarch64')][string[]]$Architectures = @('x64', 'aarch64'),
  [string]$Version = '',
  [string]$OutputDir = 'local-build/windows',
  [switch]$SkipNpmInstall,
  [switch]$SkipNsisToolchainPreparation,
  [switch]$Clean
)

$ErrorActionPreference = 'Stop'

# Keep local packages byte-for-byte aligned with the package identity configured
# as GitHub Actions repository variables for the production release workflow.
$msixIdentityName = 'u202f.FloralNote'
$msixPublisherCN = 'CN=6A63F2B3-04BC-47BA-960C-D62F116042CD'
$msixPublisherDisplayName = 'u202f'

function Invoke-NativeCommand {
  param(
    [Parameter(Mandatory = $true)][string]$Description,
    [Parameter(Mandatory = $true)][scriptblock]$Command
  )

  Write-Host "`n==> $Description" -ForegroundColor Cyan
  & $Command
  if ($LASTEXITCODE -ne 0) {
    throw "$Description failed with exit code $LASTEXITCODE."
  }
}

function Assert-WindowsSdkTool {
  param([Parameter(Mandatory = $true)][string]$Name)

  if (Get-Command $Name -ErrorAction SilentlyContinue) {
    return
  }

  $kitsRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
  $candidate = Get-ChildItem -Path $kitsRoot -Filter $Name -File -Recurse -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match '[\\/]x64[\\/]' } |
    Select-Object -First 1
  if (-not $candidate) {
    throw "$Name was not found. Install the Windows 10/11 SDK before building MSIX packages."
  }
}

if (-not $IsWindows) {
  throw 'This script must be run on Windows with PowerShell 7.'
}
if ($PSVersionTable.PSVersion.Major -lt 7) {
  throw "PowerShell 7 or newer is required; current version is $($PSVersionTable.PSVersion)."
}
if ($Architectures.Count -eq 0) {
  throw 'At least one architecture must be selected.'
}
$Architectures = @($Architectures | Select-Object -Unique)

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$packageJsonPath = Join-Path $repoRoot 'package.json'
$tauriConfigPath = Join-Path $repoRoot 'src-tauri/tauri.conf.json'
$cargoTomlPath = Join-Path $repoRoot 'src-tauri/Cargo.toml'

$packageVersion = (Get-Content -LiteralPath $packageJsonPath -Raw -Encoding UTF8 | ConvertFrom-Json).version
$tauriVersion = (Get-Content -LiteralPath $tauriConfigPath -Raw -Encoding UTF8 | ConvertFrom-Json).version
$cargoToml = Get-Content -LiteralPath $cargoTomlPath -Raw -Encoding UTF8
if ($cargoToml -notmatch '(?m)^version = "([^"]+)"') {
  throw 'Unable to read the package version from src-tauri/Cargo.toml.'
}
$cargoVersion = $Matches[1]

if ([string]::IsNullOrWhiteSpace($Version)) {
  $Version = $packageVersion
}
$Version = $Version.Trim() -replace '^[vV]', ''
if ($Version -notmatch '^\d+\.\d+\.\d+$') {
  throw "Version must be stable MAJOR.MINOR.PATCH, got: $Version"
}
foreach ($entry in @(
    @{ File = 'package.json'; Value = $packageVersion },
    @{ File = 'src-tauri/tauri.conf.json'; Value = $tauriVersion },
    @{ File = 'src-tauri/Cargo.toml'; Value = $cargoVersion }
  )) {
  if ($entry.Value -ne $Version) {
    throw "Version mismatch in $($entry.File): expected $Version, got $($entry.Value). Run npm run version:sync first."
  }
}

$outputRoot = if ([System.IO.Path]::IsPathRooted($OutputDir)) {
  [System.IO.Path]::GetFullPath($OutputDir)
} else {
  [System.IO.Path]::GetFullPath((Join-Path $repoRoot $OutputDir))
}
if ($Clean -and (Test-Path -LiteralPath $outputRoot)) {
  $normalizedOutput = $outputRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar)
  $normalizedRepo = $repoRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar)
  $volumeRoot = [System.IO.Path]::GetPathRoot($outputRoot).TrimEnd([System.IO.Path]::DirectorySeparatorChar)
  $outputPrefix = "$normalizedOutput$([System.IO.Path]::DirectorySeparatorChar)"
  $repoPrefix = "$normalizedRepo$([System.IO.Path]::DirectorySeparatorChar)"
  $outputContainsRepository = $repoPrefix.StartsWith(
    $outputPrefix,
    [System.StringComparison]::OrdinalIgnoreCase
  )
  if (
    $normalizedOutput -eq $normalizedRepo -or
    $normalizedOutput -eq $volumeRoot -or
    $outputContainsRepository
  ) {
    throw "Refusing to clean unsafe output directory: $outputRoot"
  }
  Remove-Item -LiteralPath $outputRoot -Recurse -Force
}
New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null

$npmCommand = Get-Command npm.cmd -ErrorAction SilentlyContinue
if (-not $npmCommand) {
  $npmCommand = Get-Command npm -ErrorAction Stop
}
$rustupCommand = Get-Command rustup.exe -ErrorAction SilentlyContinue
if (-not $rustupCommand) {
  $rustupCommand = Get-Command rustup -ErrorAction Stop
}
Assert-WindowsSdkTool 'makeappx.exe'
Assert-WindowsSdkTool 'makepri.exe'

Push-Location $repoRoot
try {
  if (-not $SkipNpmInstall) {
    Invoke-NativeCommand 'Install npm dependencies' {
      & $npmCommand.Source ci
    }
  } elseif (-not (Test-Path -LiteralPath (Join-Path $repoRoot 'node_modules'))) {
    throw 'node_modules does not exist; remove -SkipNpmInstall or run npm ci first.'
  }

  if (-not $SkipNsisToolchainPreparation) {
    Write-Host "`n==> Prepare Tauri NSIS toolchain" -ForegroundColor Cyan
    & (Join-Path $PSScriptRoot 'prepare-nsis-toolchain.ps1')
  }

  $targetByArchitecture = @{
    x64 = 'x86_64-pc-windows-msvc'
    aarch64 = 'aarch64-pc-windows-msvc'
  }
  $installedTargets = @(& $rustupCommand.Source target list --installed)
  if ($LASTEXITCODE -ne 0) {
    throw "Unable to list installed Rust targets (exit code $LASTEXITCODE)."
  }

  $artifacts = [System.Collections.Generic.List[string]]::new()
  $msixPaths = [System.Collections.Generic.List[string]]::new()

  foreach ($architecture in $Architectures) {
    $rustTarget = $targetByArchitecture[$architecture]
    if ($installedTargets -notcontains $rustTarget) {
      Invoke-NativeCommand "Install Rust target $rustTarget" {
        & $rustupCommand.Source target add $rustTarget
      }
    }

    Invoke-NativeCommand "Build unsigned Tauri EXE and NSIS ($architecture)" {
      & $npmCommand.Source run tauri -- build `
        --target $rustTarget `
        --bundles nsis `
        --no-sign `
        --ci
    }

    $targetReleaseDir = Join-Path $repoRoot "src-tauri/target/$rustTarget/release"
    $binaryPath = Join-Path $targetReleaseDir 'floral-notepaper.exe'
    if (-not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
      throw "Built executable was not found: $binaryPath"
    }

    $installer = Get-ChildItem -Path (Join-Path $targetReleaseDir 'bundle/nsis') -File -Recurse -Filter '*.exe' |
      Sort-Object LastWriteTimeUtc -Descending |
      Select-Object -First 1
    if (-not $installer) {
      throw "No NSIS installer was found for $architecture."
    }

    $portableName = if ($architecture -eq 'x64') {
      "floral-notepaper_${Version}.exe"
    } else {
      "floral-notepaper_${Version}_aarch64.exe"
    }
    $portableOutput = Join-Path $outputRoot $portableName
    $installerOutput = Join-Path $outputRoot "floral-notepaper_${Version}_${architecture}-setup.exe"
    Copy-Item -LiteralPath $binaryPath -Destination $portableOutput -Force
    Copy-Item -LiteralPath $installer.FullName -Destination $installerOutput -Force
    [void]$artifacts.Add($portableOutput)
    [void]$artifacts.Add($installerOutput)

    Write-Host "`n==> Build unsigned MSIX ($architecture)" -ForegroundColor Cyan
    & (Join-Path $PSScriptRoot 'build-msix.ps1') `
      -Version $Version `
      -IdentityName $msixIdentityName `
      -PublisherCN $msixPublisherCN `
      -PublisherDisplayName $msixPublisherDisplayName `
      -Arch $architecture `
      -BinaryPath $binaryPath `
      -IconsDir (Join-Path $repoRoot 'src-tauri/icons') `
      -OutputDir $outputRoot

    $msixPath = Join-Path $outputRoot "floral-notepaper_${Version}_${architecture}.msix"
    if (-not (Test-Path -LiteralPath $msixPath -PathType Leaf)) {
      throw "Built MSIX package was not found: $msixPath"
    }
    [void]$msixPaths.Add($msixPath)
    [void]$artifacts.Add($msixPath)
  }

  Write-Host "`n==> Build unsigned MSIX upload container" -ForegroundColor Cyan
  & (Join-Path $PSScriptRoot 'build-msixupload.ps1') `
    -Version $Version `
    -MsixPaths ($msixPaths.ToArray()) `
    -OutputDir $outputRoot `
    -AllowUnsignedPackages

  $msixUploadPath = Join-Path $outputRoot "floral-notepaper_${Version}.msixupload"
  if (-not (Test-Path -LiteralPath $msixUploadPath -PathType Leaf)) {
    throw "Built MSIX upload container was not found: $msixUploadPath"
  }
  [void]$artifacts.Add($msixUploadPath)

  Write-Host "`nUnsigned Windows artifacts built successfully:" -ForegroundColor Green
  foreach ($artifactPath in $artifacts) {
    $file = Get-Item -LiteralPath $artifactPath
    $sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
    Write-Host "  $($file.Name)  $($file.Length) bytes  SHA256=$sha256"
  }
  Write-Host "`nOutput directory: $outputRoot" -ForegroundColor Green
} finally {
  Pop-Location
}
