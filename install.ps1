#!/usr/bin/env pwsh
# Copyright 2024 the SchemaJS authors. All rights reserved. MIT license.
# Adapted from https://deno.land/x/install@v0.3.1/install.sh by the Deno authors.
# TODO(everyone): Keep this script simple and easily auditable.

$ErrorActionPreference = 'Stop'

if ($v) {
  $Version = "v${v}"
}
if ($Args.Length -eq 1) {
  $Version = $Args.Get(0)
}

$SjsInstall = $env:SJS_INSTALL
$BinDir = if ($SjsInstall) {
  "${SjsInstall}\bin"
} else {
  "${Home}\.schemajs\bin"
}

$SjsZip = "$BinDir\schemajs.zip"
$SjsExe = "$BinDir\schemajs.exe"
$Target = 'x86_64-pc-windows-msvc'

$Version = if (!$Version) {
  "v0.1.0"
} else {
  $Version
}

$DownloadUrl = "https://github.com/Schema-JS/schema-js/releases/download/${Version}/schemajs-${Target}.zip"

if (!(Test-Path $BinDir)) {
  New-Item $BinDir -ItemType Directory | Out-Null
}

curl.exe -Lo $SjsZip $DownloadUrl

tar.exe xf $SjsZip -C $BinDir

Remove-Item $SjsZip

$User = [System.EnvironmentVariableTarget]::User
$Path = [System.Environment]::GetEnvironmentVariable('Path', $User)
if (!(";${Path};".ToLower() -like "*;${BinDir};*".ToLower())) {
  [System.Environment]::SetEnvironmentVariable('Path', "${Path};${BinDir}", $User)
  $Env:Path += ";${BinDir}"
}

Write-Output "SchemaJS was installed successfully to ${SjsExe}"
Write-Output "Run 'schemajs --help' to get started"
Write-Output "Stuck? Join our Discord https://discord.gg/nRzTHygKn5"