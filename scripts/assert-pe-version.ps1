<#
.SYNOPSIS
  Verifies PE FileVersion and ProductVersion metadata.
#>
param(
  [Parameter(Mandatory = $true)][string]$Path,
  [Parameter(Mandatory = $true)][string]$Version,
  [Parameter(Mandatory = $true)][ValidateSet('PE', 'installer', 'signed')][string]$Kind,
  [string]$ContextText = ''
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
  throw "$Kind file was not found: $Path"
}

$versionInfo = [System.Diagnostics.FileVersionInfo]::GetVersionInfo(
  (Resolve-Path -LiteralPath $Path).Path
)
$expectedVersionPattern = "^$([Regex]::Escape($Version))(?:\.0)?(?:[+ -]|$)"
$contextSuffix = if ([string]::IsNullOrWhiteSpace($ContextText)) {
  ''
} else {
  " for $ContextText"
}

foreach ($entry in @(
    @{ Name = 'FileVersion'; Value = $versionInfo.FileVersion },
    @{ Name = 'ProductVersion'; Value = $versionInfo.ProductVersion }
  )) {
  if ([string]::IsNullOrWhiteSpace($entry.Value) -or $entry.Value -notmatch $expectedVersionPattern) {
    if ($Kind -eq 'signed') {
      throw "Unexpected $($entry.Name) after signing: expected $Version, got $($entry.Value)."
    }
    throw "Unexpected $Kind $($entry.Name)$($contextSuffix): expected $Version, got $($entry.Value)."
  }
}
