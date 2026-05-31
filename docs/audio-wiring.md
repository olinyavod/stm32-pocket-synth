# Audio wiring

## Current H7 mono buzzer wiring

The STM32H743 profile currently mixes up to 8 logical voices and drives audio
from one PWM pin:

| Board signal | MCU pin | Timer channel | Firmware role |
|---|---|---|---|
| A0 | PA3 | TIM2_CH4 | Mono note-frequency PWM output |

The older two-output analog mixer is no longer needed on H7. If the breadboard
still has a branch from D0/PB7 into the audio node, remove that branch; D0 is
not part of the current H7 audio path.

Our normal breadboard audio stage is the transistor driver with a volume pot:

```text
                    +3V3 or +5V
                         |
                    buzzer/speaker +
                    buzzer/speaker -
                         |
                     collector
NUCLEO A0 / PA3 -- 1k -- pot top
NUCLEO GND ------------- pot bottom
pot wiper -------- 2.2k..10k -------- base      NPN transistor
NUCLEO GND -------------------------- emitter
base ------------- 100k ------------- GND       optional pulldown
```

Keep the MCU and buzzer supply grounds common. The fixed base resistor matters:
if the potentiometer is turned all the way up, it still limits current from the
STM32 pin into the transistor base. A 10k..50k potentiometer is a good range.

If the buzzer is a magnetic/coil type, add a flyback diode across the buzzer:

```text
diode cathode ---- +3V3/+5V side of buzzer
diode anode ------ transistor collector side
```

For a MOSFET version, use the same load wiring, but drive the gate from the pot
wiper through about 100R and keep a 100k gate pulldown to GND.

## Direct piezo smoke test

This is only a quick test path for a passive piezo element, not the normal
breadboard amplifier:

```text
NUCLEO A0 / PA3 ---- 100R..330R ---- piezo +
NUCLEO GND ------------------------- piezo -
```

Do not connect a low-impedance speaker directly to the STM32 GPIO. Use the
transistor stage above, or a real audio amplifier module for an 8 ohm speaker.
An active buzzer has its own oscillator, so it will mostly turn the firmware
notes into on/off beeps instead of controlled pitch.

## Filtered line output

The current firmware generates note-frequency square waves, not high-rate DAC
PWM. A small RC filter can soften the square wave before an amplifier input,
but it is not required for a piezo buzzer and it should not drive headphones or
a speaker directly.

Example line-level-ish filter into a high-impedance amp input:

```text
PA3 / A0 ---- 1k ----+---- 1uF..10uF ---- amp input
                     |
                   10nF..47nF
                     |
                    GND
```
