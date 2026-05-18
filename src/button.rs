//! On-board `KEY` button (PA0 on WeAct BlackPill) as a test trigger.
//!
//! While the button is held, voice 0 sustains a test note (A4 = MIDI 69).
//! Releasing the button stops it. The 15 ms polling interval doubles as a
//! cheap debounce — switch bounce is typically < 5 ms, so any noisy edges
//! settle by the next poll.

use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::peripherals::{PA0, PC13};
use embassy_time::{Duration, Timer};

use crate::generated::MIDI_NOTE_HZ;
use crate::voice::{dispatch, VoiceCmd};

/// MIDI 69 = A4 = 440 Hz (concert pitch).
const TEST_NOTE: u8 = 69;

#[embassy_executor::task]
pub async fn button_task(pin: PA0, led_pin: PC13) -> ! {
    // PA0 has the WeAct KEY button to GND — pull-up keeps it HIGH idle.
    let button = Input::new(pin, Pull::Up);

    // On-board user LED on PC13 (active LOW: drive LOW to light it up).
    // We use it as a "button is being read" indicator so we can tell
    // firmware-level button detection apart from analog audio problems.
    let mut led = Output::new(led_pin, Level::High, Speed::Low);

    let mut prev_pressed = false;
    loop {
        let pressed = button.is_low();
        if pressed != prev_pressed {
            if pressed {
                let freq = MIDI_NOTE_HZ[TEST_NOTE as usize];
                dispatch(
                    0,
                    VoiceCmd::NoteOn {
                        note: TEST_NOTE,
                        freq_hz: freq,
                    },
                );
                led.set_low(); // LED ON
            } else {
                dispatch(0, VoiceCmd::NoteOff { note: TEST_NOTE });
                led.set_high(); // LED OFF
            }
            prev_pressed = pressed;
        }
        Timer::after(Duration::from_millis(15)).await;
    }
}
