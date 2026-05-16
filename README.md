# stm32-pocket-synth

Carmel-sized synthesizer experiments on a WeAct **STM32F411CEU6 BlackPill**, written in Rust on top of the [Embassy](https://embassy.dev/) async framework.

## Current state

A **breathing LED** demo that exercises the full PWM + DMA + Flash-LUT pipeline that any DDS-style synthesizer is built on:

- TIM1 generates 1 kHz PWM on **PA8** (TIM1_CH1)
- DMA2 Stream 5 feeds a gamma-corrected sine LUT into `TIM1->CCR1` on every timer Update event
- The 2048-entry LUT is computed in `build.rs` and lives entirely in Flash (`.rodata`) — zero RAM cost, zero boot-time math
- CPU sleeps on `WFI` between DMA-driven CCR updates
- One full breath cycle = `LUT_LEN / PWM_FREQ_HZ` ≈ 2 s

Same primitives, swapped LUT/rate, scale up to audio: feed an I2S DAC at 48 kHz with a phase-accumulator into the sine LUT, sum a few accumulators for polyphony, multiply samples by an ADSR envelope, modulate the phase increment for FM.

## Roadmap

- [x] Breathing LED via TIM1 PWM + DMA (verifies toolchain + DMA-pipeline pattern)
- [ ] PWM audio output through an RC filter (audible sine, swept frequency)
- [ ] I2S audio out to an external DAC (PCM5102, MAX98357, etc.)
- [ ] Phase-accumulator DDS, variable note frequency
- [ ] Polyphony (multiple accumulators mixed)
- [ ] ADSR envelopes
- [ ] MIDI input (UART)
- [ ] Physical piano keys (button matrix → MIDI events)
- [ ] Effect chain (reverb, chorus, simple filters)
- [ ] Multiple synthesized instrument voices

## Build

```powershell
# After installing Rust + the embedded target + cargo-binutils + dfu-util.
cargo build --release
```

## Flash via USB DFU

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

| BlackPill pin | Connected to |
|---|---|
| PA8 (TIM1_CH1) | LED anode → 330 Ω resistor → GND |
| GND | LED cathode (via the resistor) |
| USB-C / micro-USB | PC, for power and DFU flashing |

## Why Rust + Embassy?

- `async/await` puts the state machine in the compiler, not in your head — DMA-completion glue becomes a single `.await` instead of a global `AtomicBool` + ISR + main-loop polling.
- Zero heap, zero GC, zero hidden allocations. Futures live as stack-allocated state machines sized at compile time.
- The whole sine LUT is a `static [u16; 2048]` baked into Flash by `build.rs` — no boot computation, no `libm` runtime cost.

## License

MIT — see [LICENSE](LICENSE) (TBD).
