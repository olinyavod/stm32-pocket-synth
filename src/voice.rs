//! Audio synthesis voices — each one drives a TIMx_CH1 pin as a hardware
//! square wave at the note frequency. Two tasks live here:
//!
//! - [`melody_task`]: TIM3_CH1 on PA6, with triangle-LFO vibrato. Signals
//!   [`MELODY_NOTE`] before every note so the light-show stays in sync.
//! - [`bass_task`]: TIM4_CH1 on PB6, plain 50 % duty square waves — no
//!   vibrato, no LED signalling.

use embassy_stm32::gpio::OutputType;
use embassy_stm32::peripherals::{PA6, PB6, TIM3, TIM4};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

// Pure data + helpers live in the lib half (testable on host).
use black_sitizator::songs::{TETRIS_BASS, TETRIS_MELODY};
use black_sitizator::util::vibrato_freq;

/// Brief silence after every note so two same-pitch notes are heard as
/// separate strikes — same articulation trick a Game Boy used.
pub const NOTE_GAP_MS: u64 = 15;

/// Light-show synchronization: melody_task publishes (freq_hz, tone_ms) before
/// each note, led_task waits on this.
pub static MELODY_NOTE: Signal<CriticalSectionRawMutex, (u32, u64)> = Signal::new();

// --- Vibrato LFO ---

/// Triangle wave in cents. ±10 cents = subtle "expressive" wobble.
/// 16 entries × `VIBRATO_STEP_MS` (10 ms) = 160 ms full cycle ≈ 6.25 Hz.
const VIBRATO_CENTS: [i32; 16] = [
    0, 3, 6, 9, 10, 9, 6, 3, 0, -3, -6, -9, -10, -9, -6, -3,
];
const VIBRATO_STEP_MS: u64 = 10;

#[embassy_executor::task]
pub async fn melody_task(tim: TIM3, pin: PA6) -> ! {
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
    let mut lfo_idx: usize = 0;

    loop {
        for &(freq, dur_ms) in TETRIS_MELODY {
            let tone_ms = (dur_ms as u64).saturating_sub(NOTE_GAP_MS);
            // Tell the light-show about this note.
            MELODY_NOTE.signal((freq, tone_ms));
            if freq == 0 {
                pwm.ch1().set_duty_cycle(0);
                Timer::after(Duration::from_millis(tone_ms)).await;
            } else {
                // Step through the LFO entries until tone_ms is consumed.
                let mut elapsed: u64 = 0;
                while elapsed < tone_ms {
                    let cents = VIBRATO_CENTS[lfo_idx % VIBRATO_CENTS.len()];
                    pwm.set_frequency(Hertz(vibrato_freq(freq, cents)));
                    let mid = pwm.max_duty_cycle() / 2;
                    pwm.ch1().set_duty_cycle(mid);
                    let dt = VIBRATO_STEP_MS.min(tone_ms - elapsed);
                    Timer::after(Duration::from_millis(dt)).await;
                    elapsed += dt;
                    lfo_idx = lfo_idx.wrapping_add(1);
                }
            }
            pwm.ch1().set_duty_cycle(0);
            Timer::after(Duration::from_millis(NOTE_GAP_MS)).await;
        }
    }
}

#[embassy_executor::task]
pub async fn bass_task(tim: TIM4, pin: PB6) -> ! {
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

    loop {
        for &(freq, dur_ms) in TETRIS_BASS {
            let tone_ms = (dur_ms as u64).saturating_sub(NOTE_GAP_MS);
            if freq == 0 {
                pwm.ch1().set_duty_cycle(0);
            } else {
                pwm.set_frequency(Hertz(freq));
                let mid = pwm.max_duty_cycle() / 2;
                pwm.ch1().set_duty_cycle(mid);
            }
            Timer::after(Duration::from_millis(tone_ms)).await;
            pwm.ch1().set_duty_cycle(0);
            Timer::after(Duration::from_millis(NOTE_GAP_MS)).await;
        }
    }
}
