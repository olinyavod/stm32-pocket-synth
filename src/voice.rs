//! Two-voice polyphonic chiptune driven by MIDI events from `usb_task`.
//!
//! Each voice owns a hardware timer (TIM3 → PA6, TIM4 → PB6) running as a
//! square-wave oscillator. The MIDI parser picks which voice should sound each
//! Note-On using a tiny round-robin allocator, then drops the Note-Off back
//! to whichever voice was holding that note. Both voices output to the same
//! piezo through the resistor summer, so we get a real 2-channel chiptune mix.

use embassy_stm32::gpio::OutputType;
use embassy_stm32::peripherals::{PA6, PB7, TIM3, TIM4};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

/// Light-show feed — signalled on every Note-On (any voice).
pub static MELODY_NOTE: Signal<CriticalSectionRawMutex, (u32, u64)> = Signal::new();

/// LED fade hint used by `light_show::led_task` — MIDI notes have no a priori
/// duration so we just pick a perceptually pleasant value.
pub const LED_FADE_HINT_MS: u64 = 200;

/// PWM duty cycle for an active note. 50 % is the standard symmetric square
/// wave — maximum fundamental power, classic chiptune sound. Two voices summed
/// through the resistor mixer can briefly hit the supply rail; if that turns
/// out to crackle in the amplifier, lower this value (e.g. 30 %).
pub const NOTE_DUTY_PCT: u32 = 50;

#[derive(Clone, Copy)]
pub enum VoiceCmd {
    NoteOn { note: u8, freq_hz: u32 },
    NoteOff { note: u8 },
    AllOff,
}

// One bounded queue per voice. 16 slots covers the burstiest realistic key-press
// rate (VMPK / fast trills); excess events are dropped via `try_send`.
pub static VOICE_0: Channel<CriticalSectionRawMutex, VoiceCmd, 16> = Channel::new();
pub static VOICE_1: Channel<CriticalSectionRawMutex, VoiceCmd, 16> = Channel::new();

// Voice tasks are near-duplicates because each is generic over a different
// concrete TIM/pin pair — embassy's task macro doesn't accept generics, so we
// duplicate the small inner loop rather than fight the borrow checker.

#[embassy_executor::task]
pub async fn voice0_task(tim: TIM3, pin: PA6) -> ! {
    let p = PwmPin::new_ch1(pin, OutputType::PushPull);
    let mut pwm = SimplePwm::new(
        tim,
        Some(p),
        None,
        None,
        None,
        Hertz(1_000),
        CountingMode::EdgeAlignedUp,
    );
    pwm.ch1().enable();
    pwm.ch1().set_duty_cycle(0);

    let mut current: Option<u8> = None;
    loop {
        match VOICE_0.receive().await {
            VoiceCmd::NoteOn { note, freq_hz } => {
                pwm.set_frequency(Hertz(freq_hz));
                // 30 % duty (not 50 %) so when both voices happen to be HIGH at
                // the same instant the summed voltage on node A stays in the
                // amplifier's linear range. Avoids clipping during 2-key
                // polyphony at the cost of mono-volume; the user compensates
                // with the volume pot.
                let duty = (pwm.max_duty_cycle() as u32 * NOTE_DUTY_PCT / 100) as u16;
                pwm.ch1().set_duty_cycle(duty);
                current = Some(note);
                MELODY_NOTE.signal((freq_hz, LED_FADE_HINT_MS));
            }
            VoiceCmd::NoteOff { note } => {
                if current == Some(note) {
                    pwm.ch1().set_duty_cycle(0);
                    current = None;
                }
            }
            VoiceCmd::AllOff => {
                pwm.ch1().set_duty_cycle(0);
                current = None;
            }
        }
    }
}

#[embassy_executor::task]
pub async fn voice1_task(tim: TIM4, pin: PB7) -> ! {
    // PB7 = TIM4_CH2. Switched from PB6/CH1 because PB6 appeared dead in
    // bench-testing (possibly burnt or never properly soldered).
    let p = PwmPin::new_ch2(pin, OutputType::PushPull);
    let mut pwm = SimplePwm::new(
        tim,
        None,
        Some(p),
        None,
        None,
        Hertz(1_000),
        CountingMode::EdgeAlignedUp,
    );
    pwm.ch2().enable();
    pwm.ch2().set_duty_cycle(0);

    let mut current: Option<u8> = None;
    loop {
        match VOICE_1.receive().await {
            VoiceCmd::NoteOn { note, freq_hz } => {
                pwm.set_frequency(Hertz(freq_hz));
                let duty = (pwm.max_duty_cycle() as u32 * NOTE_DUTY_PCT / 100) as u16;
                pwm.ch2().set_duty_cycle(duty);
                current = Some(note);
                MELODY_NOTE.signal((freq_hz, LED_FADE_HINT_MS));
            }
            VoiceCmd::NoteOff { note } => {
                if current == Some(note) {
                    pwm.ch2().set_duty_cycle(0);
                    current = None;
                }
            }
            VoiceCmd::AllOff => {
                pwm.ch2().set_duty_cycle(0);
                current = None;
            }
        }
    }
}

// --- Voice allocator (consumed by usb_task) ---

/// Tracks which MIDI note (if any) each voice is currently sounding, and
/// chooses targets for new Note-Ons via round-robin steal.
pub struct Allocator {
    notes: [Option<u8>; 2],
    next_steal: usize,
}

impl Allocator {
    pub const fn new() -> Self {
        Self {
            notes: [None; 2],
            next_steal: 0,
        }
    }

    /// Pick a voice to sound `note`. Prefer an idle slot; otherwise steal one
    /// in round-robin order. Returns the voice index (0 or 1).
    pub fn note_on(&mut self, note: u8) -> usize {
        for i in 0..self.notes.len() {
            if self.notes[i].is_none() {
                self.notes[i] = Some(note);
                return i;
            }
        }
        let idx = self.next_steal;
        self.next_steal = (idx + 1) % self.notes.len();
        self.notes[idx] = Some(note);
        idx
    }

    /// Return the voice (if any) that was holding this note; clears it.
    pub fn note_off(&mut self, note: u8) -> Option<usize> {
        for i in 0..self.notes.len() {
            if self.notes[i] == Some(note) {
                self.notes[i] = None;
                return Some(i);
            }
        }
        None
    }

    pub fn all_off(&mut self) {
        self.notes = [None; 2];
    }
}

/// Send a `VoiceCmd` to voice `idx`. Drops the command if the queue is full.
pub fn dispatch(idx: usize, cmd: VoiceCmd) {
    match idx {
        0 => {
            let _ = VOICE_0.try_send(cmd);
        }
        1 => {
            let _ = VOICE_1.try_send(cmd);
        }
        _ => {}
    }
}
