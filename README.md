# stm32-pocket-synth

Carmel-sized synthesizer experiments on a WeAct **STM32F411CEU6 BlackPill**, written in Rust on top of the [Embassy](https://embassy.dev/) async framework. The STM32F411 profile remains the default; an STM32H7 profile is available for **NUCLEO-H743ZI2** with **STM32H743ZIT6**.

## Current state

A **breathing LED** demo that exercises the full PWM + DMA + Flash-LUT pipeline that any DDS-style synthesizer is built on:

- TIM1 generates 1 kHz PWM on **PA8** (TIM1_CH1) on STM32F411,
  or **D6 / PE9** (TIM1_CH1) on NUCLEO-H743ZI2
- DMA2 Stream 5 feeds a gamma-corrected sine LUT into `TIM1->CCR1` on every timer Update event
- The 2048-entry LUT is computed in `build.rs` and lives entirely in Flash (`.rodata`) — zero RAM cost, zero boot-time math
- CPU sleeps on `WFI` between DMA-driven CCR updates
- One full breath cycle = `LUT_LEN / PWM_FREQ_HZ` ≈ 2 s

Same primitives, swapped LUT/rate, scale up to audio: feed an I2S DAC at 48 kHz with a phase-accumulator into the sine LUT, sum a few accumulators for polyphony, multiply samples by an ADSR envelope, modulate the phase increment for FM.

The H7 profile currently mixes up to 8 logical DDS voices into one audio-rate
PWM output on **A0 / PA3**. The older two-pin analog voice mixer is only for
the F411 profile.

## Roadmap

- [x] Breathing LED via TIM1 PWM + DMA (verifies toolchain + DMA-pipeline pattern)
- [x] H7 8-voice software DDS mixer on one PWM audio pin
- [ ] RC-filtered line output polish for the PWM audio path
- [ ] I2S audio out to an external DAC (PCM5102, MAX98357, etc.)
- [ ] Phase-accumulator DDS, variable note frequency
- [x] Polyphony (multiple accumulators mixed on H7)
- [ ] ADSR envelopes
- [ ] MIDI input (UART)
- [ ] Physical piano keys (button matrix → MIDI events)
- [ ] Effect chain (reverb, chorus, simple filters)
- [ ] Multiple synthesized instrument voices

## Build

```powershell
# Default profile: STM32F411CEU6 BlackPill.
cargo build --release

# Same default target, explicit alias.
cargo build-f411

# NUCLEO-H743ZI2 / STM32H743ZI profile.
cargo build-h743

# Equivalent full command.
cargo build --release --no-default-features --features mcu-stm32h7
```

The H7 profile targets NUCLEO-H743ZI2. It currently uses Embassy's default
HSI/HSI48 clock setup for bring-up. After the board-level wiring is settled,
tune `clock_config()` in `src/main.rs` for the desired SYSCLK and audio/USB
clock tree.

Probe-rs uses the chip name `STM32H743ZI` for the STM32H743ZIT6 on this board.

## Flash/run via ST-LINK (NUCLEO-H743ZI2)

Connect the Nucleo STLINK-V3E USB connector to the PC, then run:

```powershell
./run-nucleo-h743zi2.ps1
```

The script builds the `mcu-stm32h7` profile, flashes the ELF with
`probe-rs run --chip STM32H743ZI`, and keeps RTT/defmt output attached.

### USB CDC probe firmware

To isolate board USB wiring/power from the synthesizer MIDI stack, flash the
minimal CDC ACM probe:

```powershell
./run-usb-probe-h743.ps1
```

If CN13 USB is healthy, Windows should enumerate `USB Probe CDC` as a USB serial
device. Bytes sent to the COM port are echoed back, and RTT logs print
`USB CDC connected`.

## Flash via USB DFU (STM32F411)

The STM32F411 has a built-in USB DFU bootloader. No ST-Link required.

1. Connect BlackPill to PC via its own USB cable.
2. Hold **BOOT0**, press and release **NRST**, release **BOOT0** — the board appears as `STM32 BOOTLOADER` (VID:PID `0483:DF11`).
3. First time only: use [Zadig](https://zadig.akeo.ie/) to swap the driver on `STM32 BOOTLOADER` to **WinUSB**.
4. Flash:

```powershell
./flash.ps1
```

The script builds release, calls `cargo objcopy` to get a raw `.bin`, then invokes `dfu-util -s 0x08000000:leave` to flash and auto-reset into the new firmware.

## Hardware

### STM32F411 BlackPill

| BlackPill pin | Connected to |
|---|---|
| PA8 (TIM1_CH1) | LED anode → 330 Ω resistor → GND |
| GND | LED cathode (via the resistor) |
| USB-C / micro-USB | PC, for power and DFU flashing |

### NUCLEO-H743ZI2

The board wiring is centralized in `src/pinmap.rs`. The H7 profile keeps the
external project wiring on Arduino/Zio headers and leaves the usual SPI
display-shield pins free.

| Nucleo signal | MCU pin | Used for |
|---|---|---|
| B1 USER | PC13 | Test-note button, active HIGH |
| LD1 green | PB0 | Button-read indicator, active HIGH |
| D6 (TIM1_CH1) | PE9 | External light-show LED |
| A0 (TIM2_CH4) | PA3 | Mono audio PWM output |
| USB OTG FS | PA12 / PA11 | USB-MIDI D+ / D- |
| STLINK-V3E USB | SWD | Flashing and RTT logs |

The current H7 firmware uses one physical audio pin. See
[`docs/audio-wiring.md`](docs/audio-wiring.md) for the transistor + potentiometer
buzzer-driver wiring notes.

Reserved for a typical SPI display shield:

| Arduino signal | MCU pin | Suggested display use |
|---|---|---|
| D13 | PA5 | SPI SCK |
| D12 | PA6 | SPI MISO |
| D11 | PB5 | SPI MOSI |
| D10 | PD14 | Display CS |
| D9 | PD15 | Display DC |
| D8 | PF3 | Display reset |

## Why Rust + Embassy?

- `async/await` puts the state machine in the compiler, not in your head — DMA-completion glue becomes a single `.await` instead of a global `AtomicBool` + ISR + main-loop polling.
- Zero heap, zero GC, zero hidden allocations. Futures live as stack-allocated state machines sized at compile time.
- The whole sine LUT is a `static [u16; 2048]` baked into Flash by `build.rs` — no boot computation, no `libm` runtime cost.

## License

MIT — see [LICENSE](LICENSE) (TBD).
