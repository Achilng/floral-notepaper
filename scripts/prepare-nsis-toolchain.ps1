<#
.SYNOPSIS
  Preloads the Tauri NSIS toolchain under %LOCALAPPDATA%\tauri\NSIS.

.DESCRIPTION
  Restores NSIS from hash-verified download files under
  %LOCALAPPDATA%\floral-notepaper-ci-cache\nsis. The download directory is safe
  to cache because every file is verified before it is used.
#>
$ErrorActionPreference = 'Stop'

$nsisZipSha1 = 'EF7FF767E5CBD9EDD22ADD3A32C9B8F4500BB10D'
$pluginSha1 = '75197FEE3C6A814FE035788D1C34EAD39349B860'

function Get-VerifiedFileWithRetry {
  param(
    [Parameter(Mandatory = $true)][string]$Uri,
    [Parameter(Mandatory = $true)][string]$OutFile,
    [Parameter(Mandatory = $true)][string]$Sha1
  )

  if (Test-Path -LiteralPath $OutFile -PathType Leaf) {
    $cachedSha1 = (Get-FileHash -LiteralPath $OutFile -Algorithm SHA1).Hash.ToUpperInvariant()
    if ($cachedSha1 -eq $Sha1.ToUpperInvariant()) {
      Write-Host "Using verified cached download $OutFile"
      return
    }
    Write-Warning "Discarding cached file with unexpected SHA1: $OutFile"
    Remove-Item -LiteralPath $OutFile -Force
  }

  for ($attempt = 1; $attempt -le 5; $attempt++) {
    try {
      if (Test-Path -LiteralPath $OutFile) {
        Remove-Item -LiteralPath $OutFile -Force
      }

      Write-Host "Downloading $Uri (attempt $attempt/5)"
      Invoke-WebRequest -Uri $Uri -OutFile $OutFile -TimeoutSec 180

      $actual = (Get-FileHash -LiteralPath $OutFile -Algorithm SHA1).Hash.ToUpperInvariant()
      if ($actual -ne $Sha1.ToUpperInvariant()) {
        throw "SHA1 mismatch for $OutFile. Expected $Sha1, got $actual."
      }
      return
    } catch {
      Write-Warning $_
      if ($attempt -eq 5) {
        throw
      }
      Start-Sleep -Seconds (10 * $attempt)
    }
  }
}

$downloadCache = Join-Path $env:LOCALAPPDATA 'floral-notepaper-ci-cache\nsis'
New-Item -ItemType Directory -Path $downloadCache -Force | Out-Null

$zipPath = Join-Path $downloadCache 'nsis-3.11.zip'
$cachedPluginPath = Join-Path $downloadCache 'nsis_tauri_utils-v0.5.3.dll'
Get-VerifiedFileWithRetry `
  -Uri 'https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip' `
  -OutFile $zipPath `
  -Sha1 $nsisZipSha1
Get-VerifiedFileWithRetry `
  -Uri 'https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll' `
  -OutFile $cachedPluginPath `
  -Sha1 $pluginSha1

$tauriTools = Join-Path $env:LOCALAPPDATA 'tauri'
$nsisPath = Join-Path $tauriTools 'NSIS'
$extractedPath = Join-Path $tauriTools 'nsis-3.11'
New-Item -ItemType Directory -Path $tauriTools -Force | Out-Null
foreach ($path in @($nsisPath, $extractedPath)) {
  if (Test-Path -LiteralPath $path) {
    Remove-Item -LiteralPath $path -Recurse -Force
  }
}

Expand-Archive -LiteralPath $zipPath -DestinationPath $tauriTools -Force
Rename-Item -LiteralPath $extractedPath -NewName 'NSIS'

$pluginDir = Join-Path $nsisPath 'Plugins\x86-unicode\additional'
New-Item -ItemType Directory -Path $pluginDir -Force | Out-Null
Copy-Item -LiteralPath $cachedPluginPath -Destination (Join-Path $pluginDir 'nsis_tauri_utils.dll') -Force

$requiredFiles = @(
  'makensis.exe',
  'Bin\makensis.exe',
  'Stubs\lzma-x86-unicode',
  'Stubs\lzma_solid-x86-unicode',
  'Plugins\x86-unicode\additional\nsis_tauri_utils.dll',
  'Include\MUI2.nsh',
  'Include\FileFunc.nsh',
  'Include\x64.nsh',
  'Include\nsDialogs.nsh',
  'Include\WinMessages.nsh',
  'Include\Win\COM.nsh',
  'Include\Win\Propkey.nsh',
  'Include\Win\RestartManager.nsh'
)
foreach ($file in $requiredFiles) {
  if (-not (Test-Path -LiteralPath (Join-Path $nsisPath $file) -PathType Leaf)) {
    throw "Required NSIS toolchain file is missing after extraction: $file"
  }
}

Write-Host "Tauri NSIS toolchain is ready at $nsisPath"
