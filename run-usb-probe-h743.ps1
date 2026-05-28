$OutputEncoding = [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$env:Path = "$env:Path;D:\Rust\.cargo\bin"
$ErrorActionPreference = "Stop"

Set-Location $PSScriptRoot

Write-Host "Building USB CDC probe for NUCLEO-H743ZI2 (STM32H743ZI)"
cargo build-usb-probe-h743
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$elf = "target\thumbv7em-none-eabihf\release\usb_probe"
if (-not (Test-Path $elf)) {
    Write-Host "ELF not found: $elf"
    exit 1
}

Write-Host ""
Write-Host "Flashing USB CDC probe via on-board STLINK-V3E"
probe-rs run --chip STM32H743ZI $elf
exit $LASTEXITCODE
