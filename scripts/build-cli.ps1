param([string]$Destination)
$ErrorActionPreference='Stop'
$projectRoot=Split-Path -Parent $PSScriptRoot
$manifest=Join-Path $projectRoot 'engine/Cargo.toml'
& cargo build --release --locked --manifest-path $manifest
if($LASTEXITCODE -ne 0){throw 'CLI build failed'}
$metadata=(& cargo metadata --no-deps --format-version 1 --manifest-path $manifest | ConvertFrom-Json)
if($LASTEXITCODE -ne 0){throw 'Cannot resolve Cargo target directory'}
$binary=Join-Path $metadata.target_directory 'release/fitig-engine.exe'
if(-not(Test-Path -LiteralPath $binary -PathType Leaf)){throw 'Windows native CLI executable was not produced'}
if(-not $Destination){$Destination=Join-Path $projectRoot 'artifacts/cli'}
[void](New-Item -ItemType Directory -Path $Destination -Force)
$output=Join-Path $Destination 'fitig-engine.exe'
Copy-Item -LiteralPath $binary -Destination $output -Force
Write-Output "CLI built: $output"
