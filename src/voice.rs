//! Board-independent MIDI voice routing.
//!
//! STM32F411 keeps the original two physical PWM voices. STM32H743 uses one
//! physical PWM audio pin with a small software DDS mixer behind it.

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

/// Light-show feed - signalled on every Note-On.
pub static MELODY_NOTE: Signal<CriticalSectionRawMutex, (u32, u64)> = Signal::new();

/// LED fade hint used by `light_show::led_task` - MIDI notes have no a priori
/// duration so we just pick a perceptually pleasant value.
pub const LED_FADE_HINT_MS: u64 = 200;

#[cfg(feature = "mcu-stm32f411ce")]
pub const VOICE_COUNT: usize = 2;

#[cfg(feature = "mcu-stm32h7")]
pub const VOICE_COUNT: usize = 8;

/// PWM duty cycle for an active F4 square-wave voice.
#[cfg(feature = "mcu-stm32f411ce")]
pub const NOTE_DUTY_PCT: u32 = 50;

#[derive(Clone, Copy)]
pub enum VoiceCmd {
    NoteOn { note: u8, freq_hz: u32 },
    NoteOff { note: u8 },
    AllOff,
}

/// Tracks which MIDI note (if any) each logical voice is currently sounding,
/// and chooses targets for new Note-Ons via round-robin steal.
pub struct Allocator {
    notes: [Option<u8>; VOICE_COUNT],
    next_steal: usize,
}

impl Allocator {
    pub const fn new() -> Self {
        Self {
            notes: [None; VOICE_COUNT],
            next_steal: 0,
        }
    }

    /// Pick a voice to sound `note`. Prefer an idle slot; otherwise steal one
    /// in round-robin order. Returns the logical voice index.
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
        self.notes = [None; VOICE_COUNT];
    }
}

#[cfg(feature = "mcu-stm32f411ce")]
mod f4;

#[cfg(feature = "mcu-stm32h7")]
mod h7;

#[cfg(feature = "mcu-stm32f411ce")]
pub use f4::{dispatch, voice0_task, voice1_task};

#[cfg(feature = "mcu-stm32h7")]
pub use h7::{audio_task, dispatch};

pub fn dispatch_all_off() {
    for idx in 0..VOICE_COUNT {
        dispatch(idx, VoiceCmd::AllOff);
    }
}
