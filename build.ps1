$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
foreach ($command in @('cargo','rustc','npm.cmd')) { if (-not (Get-Command $command -ErrorAction SilentlyContinue)) { throw "Missing $command. Install Rust and Node prerequisites first." } }
& cargo test --manifest-path engine/Cargo.toml
if ($LASTEXITCODE) { throw 'Engine tests failed' }
& npm.cmd ci
if ($LASTEXITCODE) { throw 'Desktop dependency installation failed' }
& npm.cmd --prefix ui ci
if ($LASTEXITCODE) { throw 'UI dependency installation failed' }
& npm.cmd --prefix ui test
if ($LASTEXITCODE) { throw 'UI tests failed' }
& npm.cmd --prefix ui run test:core
if ($LASTEXITCODE) { throw 'UI core tests failed' }
& npm.cmd run build
if ($LASTEXITCODE) { throw 'Desktop packaging failed' }
Write-Host 'Installer: src-tauri/target/release/bundle/nsis'
