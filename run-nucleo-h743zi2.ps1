$OutputEncoding = [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$env:Path = "$env:Path;D:\Rust\.cargo\bin"
$ErrorActionPreference = "Stop"

Set-Location $PSScriptRoot

Write-Host "Building release for NUCLEO-H743ZI2 (STM32H743ZI)"
cargo build-h743
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$elf = "target\thumbv7em-none-eabihf\release\black_sitizator"
if (-not (Test-Path $elf)) {
    Write-Host "ELF not found: $elf"
    exit 1
}

Write-Host ""
Write-Host "Flashing and running via on-board STLINK-V3E"
probe-rs run --chip STM32H743ZI $elf
exit $LASTEXITCODE
