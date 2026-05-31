//! STM32H743 voice backend: 8 logical DDS voices mixed onto one PWM audio pin.

use embassy_stm32::gpio::OutputType;
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Ticker};

use crate::pinmap::{AudioPin, AudioTimer};

use super::{LED_FADE_HINT_MS, MELODY_NOTE, VOICE_COUNT, VoiceCmd};

const AUDIO_SAMPLE_HZ: u32 = 8_192;
const AUDIO_PWM_HZ: u32 = 65_536;
const WAVE_PEAK: i32 = 127;
const OUTPUT_DUTY_PCT: i32 = 20;

#[derive(Clone, Copy)]
struct RoutedVoiceCmd {
    idx: usize,
    cmd: VoiceCmd,
}

#[derive(Clone, Copy)]
struct Oscillator {
    note: Option<u8>,
    phase: u32,
    phase_step: u32,
}

impl Oscillator {
    const fn idle() -> Self {
        Self {
            note: None,
            phase: 0,
            phase_step: 0,
        }
    }
}

static AUDIO: Channel<CriticalSectionRawMutex, RoutedVoiceCmd, 64> = Channel::new();

#[embassy_executor::task]
pub async fn audio_task(tim: AudioTimer, pin: AudioPin) -> ! {
    // PA3 = Arduino A0 = TIM2_CH4 on NUCLEO-H743ZI2.
    let p = PwmPin::new_ch4(pin, OutputType::PushPull);
    let mut pwm = SimplePwm::new(
        tim,
        None,
        None,
        None,
        Some(p),
        Hertz(AUDIO_PWM_HZ),
        CountingMode::EdgeAlignedUp,
    );
    pwm.ch4().enable();
    pwm.ch4().set_duty_cycle(0);

    let mut voices = [Oscillator::idle(); VOICE_COUNT];
    let mut ticker = Ticker::every(Duration::from_hz(AUDIO_SAMPLE_HZ as u64));
    let max_duty = pwm.max_duty_cycle();

    loop {
        drain_commands(&mut voices);
        pwm.ch4().set_duty_cycle(next_duty(&mut voices, max_duty));
        ticker.next().await;
    }
}

fn drain_commands(voices: &mut [Oscillator; VOICE_COUNT]) {
    while let Ok(routed) = AUDIO.try_receive() {
        match routed.cmd {
            VoiceCmd::NoteOn { note, freq_hz } => {
                if let Some(voice) = voices.get_mut(routed.idx) {
                    voice.note = Some(note);
                    voice.phase = 0;
                    voice.phase_step = phase_step(freq_hz);
                    MELODY_NOTE.signal((freq_hz, LED_FADE_HINT_MS));
                }
            }
            VoiceCmd::NoteOff { note } => {
                if let Some(voice) = voices.get_mut(routed.idx) {
                    if voice.note == Some(note) {
                        *voice = Oscillator::idle();
                    }
                }
            }
            VoiceCmd::AllOff => {
                for voice in voices.iter_mut() {
                    *voice = Oscillator::idle();
                }
            }
        }
    }
}

fn next_duty(voices: &mut [Oscillator; VOICE_COUNT], max_duty: u16) -> u16 {
    let mut mix = 0;
    let mut active = 0;

    for voice in voices.iter_mut() {
        if voice.note.is_some() {
            voice.phase = voice.phase.wrapping_add(voice.phase_step);
            mix += triangle_sample(voice.phase);
            active += 1;
        }
    }

    if active == 0 {
        return 0;
    }

    let averaged = mix / active;
    let unsigned = averaged + WAVE_PEAK;
    let max_output = max_duty as i32 * OUTPUT_DUTY_PCT / 100;
    (unsigned * max_output / (WAVE_PEAK * 2)).clamp(0, max_duty as i32) as u16
}

fn phase_step(freq_hz: u32) -> u32 {
    (((freq_hz as u64) << 32) / AUDIO_SAMPLE_HZ as u64) as u32
}

fn triangle_sample(phase: u32) -> i32 {
    let x = (phase >> 24) as i32;
    if x < 128 {
        x * 2 - WAVE_PEAK
    } else {
        383 - x * 2
    }
}

/// Send a `VoiceCmd` to one logical H7 voice. Drops the command if the queue is
/// full; `AllOff` clears stale queued note events first so panic wins.
pub fn dispatch(idx: usize, cmd: VoiceCmd) {
    if let VoiceCmd::AllOff = cmd {
        AUDIO.clear();
        let _ = AUDIO.try_send(RoutedVoiceCmd { idx: 0, cmd });
        return;
    }

    if idx < VOICE_COUNT {
        let _ = AUDIO.try_send(RoutedVoiceCmd { idx, cmd });
    }
}
