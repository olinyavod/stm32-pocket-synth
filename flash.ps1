$OutputEncoding = [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$env:Path = "$env:Path;D:\Rust\.cargo\bin"
$ErrorActionPreference = "Stop"

Set-Location $PSScriptRoot

Write-Host "Checking for STM32 in DFU mode"
$dfu = Get-PnpDevice -PresentOnly | Where-Object { $_.InstanceId -match "VID_0483.*PID_DF11" }
if (-not $dfu) {
    Write-Host "NOT FOUND. Enter DFU mode first (hold BOOT0, press NRST, release BOOT0)."
    exit 1
}
Write-Host "Found:" $dfu.FriendlyName
Write-Host ""

Write-Host "Building release"
cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ""
Write-Host "Generating raw binary"
$elf = "target\thumbv7em-none-eabihf\release\black_sitizator"
$bin = "target\thumbv7em-none-eabihf\release\firmware.bin"
cargo objcopy --release -- -O binary $bin
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$sz = (Get-Item $bin).Length
Write-Host "Binary: $bin ($sz bytes)"
Write-Host ""

Write-Host "Flashing via dfu-util"
& dfu-util -a 0 -s 0x08000000:leave -d 0483:DF11 -D $bin
exit $LASTEXITCODE
