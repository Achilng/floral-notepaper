<#
.SYNOPSIS
  Preloads the Tauri NSIS toolchain under %LOCALAPPDATA%\tauri\NSIS.

.DESCRIPTION
  Ensures the NSIS distribution required by tauri-bundler is present under
  $env:LOCALAPPDATA\tauri\NSIS together with the nsis_tauri_utils plugin,
  downloading them from the tauri-apps GitHub releases with retries when
  missing or corrupted:
  - nsis-3.11.zip (SHA1 EF7FF767E5CBD9EDD22ADD3A32C9B8F4500BB10D)
  - nsis_tauri_utils.dll (SHA1 75197FEE3C6A814FE035788D1C34EAD39349B860)
#>
$ErrorActionPreference = 'Stop'

function Get-FileWithRetry {
  param(
    [Parameter(Mandatory = $true)][string]$Uri,
    [Parameter(Mandatory = $true)][string]$OutFile,
    [Parameter(Mandatory = $true)][string]$Sha1
  )

  for ($attempt = 1; $attempt -le 5; $attempt++) {
    try {
      if (Test-Path $OutFile) {
        Remove-Item $OutFile -Force
      }

      Write-Host "Downloading $Uri (attempt $attempt/5)"
      Invoke-WebRequest -Uri $Uri -OutFile $OutFile -TimeoutSec 180

      $actual = (Get-FileHash -Path $OutFile -Algorithm SHA1).Hash.ToUpperInvariant()
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

$tauriTools = Join-Path $env:LOCALAPPDATA 'tauri'
$nsisPath = Join-Path $tauriTools 'NSIS'
New-Item -ItemType Directory -Path $tauriTools -Force | Out-Null

$requiredFiles = @(
  'makensis.exe',
  'Bin/makensis.exe',
  'Stubs/lzma-x86-unicode',
  'Stubs/lzma_solid-x86-unicode',
  'Plugins/x86-unicode/additional/nsis_tauri_utils.dll',
  'Include/MUI2.nsh',
  'Include/FileFunc.nsh',
  'Include/x64.nsh',
  'Include/nsDialogs.nsh',
  'Include/WinMessages.nsh',
  'Include/Win/COM.nsh',
  'Include/Win/Propkey.nsh',
  'Include/Win/RestartManager.nsh'
)

$missingRequiredFile = $false
if (-not (Test-Path $nsisPath)) {
  $missingRequiredFile = $true
} else {
  foreach ($file in $requiredFiles) {
    if (-not (Test-Path (Join-Path $nsisPath $file))) {
      $missingRequiredFile = $true
      break
    }
  }
}

if ($missingRequiredFile) {
  if (Test-Path $nsisPath) {
    Remove-Item $nsisPath -Recurse -Force
  }

  $downloadRoot = if ([string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) {
    [System.IO.Path]::GetTempPath()
  } else {
    $env:RUNNER_TEMP
  }
  $zipPath = Join-Path $downloadRoot 'nsis-3.11.zip'
  Get-FileWithRetry `
    -Uri 'https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip' `
    -OutFile $zipPath `
    -Sha1 'EF7FF767E5CBD9EDD22ADD3A32C9B8F4500BB10D'

  $extractedPath = Join-Path $tauriTools 'nsis-3.11'
  if (Test-Path $extractedPath) {
    Remove-Item $extractedPath -Recurse -Force
  }

  Expand-Archive -Path $zipPath -DestinationPath $tauriTools -Force
  Rename-Item -Path $extractedPath -NewName 'NSIS'
}

$pluginDir = Join-Path $nsisPath 'Plugins/x86-unicode/additional'
$pluginPath = Join-Path $pluginDir 'nsis_tauri_utils.dll'
New-Item -ItemType Directory -Path $pluginDir -Force | Out-Null

$pluginNeedsDownload = -not (Test-Path $pluginPath)
if (-not $pluginNeedsDownload) {
  $actualPluginSha1 = (Get-FileHash -Path $pluginPath -Algorithm SHA1).Hash.ToUpperInvariant()
  $pluginNeedsDownload = $actualPluginSha1 -ne '75197FEE3C6A814FE035788D1C34EAD39349B860'
}

if ($pluginNeedsDownload) {
  Get-FileWithRetry `
    -Uri 'https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll' `
    -OutFile $pluginPath `
    -Sha1 '75197FEE3C6A814FE035788D1C34EAD39349B860'
}

Write-Host "Tauri NSIS toolchain is ready at $nsisPath"
