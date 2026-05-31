//! STM32F411 voice backend: two independent hardware PWM square waves.

use embassy_stm32::gpio::OutputType;
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

use crate::pinmap::{Voice0Pin, Voice0Timer, Voice1Pin, Voice1Timer};

use super::{LED_FADE_HINT_MS, MELODY_NOTE, NOTE_DUTY_PCT, VoiceCmd};

// One bounded queue per physical voice. 16 slots covers fast key bursts;
// excess events are dropped via `try_send`.
static VOICE_0: Channel<CriticalSectionRawMutex, VoiceCmd, 16> = Channel::new();
static VOICE_1: Channel<CriticalSectionRawMutex, VoiceCmd, 16> = Channel::new();

#[embassy_executor::task]
pub async fn voice0_task(tim: Voice0Timer, pin: Voice0Pin) -> ! {
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
pub async fn voice1_task(tim: Voice1Timer, pin: Voice1Pin) -> ! {
    // PB7 = TIM4_CH2. Switched from PB6/CH1 because PB6 appeared dead in
    // bench testing.
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

/// Send a `VoiceCmd` to physical voice `idx`. Drops the command if the queue
/// is full.
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
