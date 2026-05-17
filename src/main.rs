#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::peripherals::{DMA2_CH5, PA6, PA8, PB6, TIM1, TIM3, TIM4};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::Channel;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

include!(concat!(env!("OUT_DIR"), "/lut.rs"));

// --- LED breathing on TIM1_CH1 (PA8) ---

#[embassy_executor::task]
async fn led_task(tim: TIM1, pin: PA8, mut dma: DMA2_CH5) -> ! {
    let p = PwmPin::new_ch1(pin, OutputType::PushPull);
    let mut pwm = SimplePwm::new(
        tim,
        Some(p),
        None,
        None,
        None,
        Hertz(LED_PWM_FREQ_HZ),
        CountingMode::EdgeAlignedUp,
    );
    pwm.ch1().enable();
    defmt::assert_eq!(pwm.max_duty_cycle(), LED_PWM_TOP);
    loop {
        pwm.waveform_up(&mut dma, Channel::Ch1, &LED_LUT).await;
    }
}

// --- Shared synth machinery: chiptune square wave on TIMx_CH1 ---

// Brief silence between notes so adjacent same-pitch notes are heard separately.
const NOTE_GAP_MS: u64 = 15;

// One channel = one melody track. Each (freq_hz, duration_ms). freq=0 → rest.
type Track = &'static [(u32, u32)];

// --- Channel 1: Melody (Tetris main theme) on TIM3_CH1 / PA6 ---

const MELODY: Track = &[
    (659, 400), (494, 200), (523, 200), (587, 400),  // E5 B4 C5 D5
    (523, 200), (494, 200), (440, 400), (440, 200),  // C5 B4 A4 A4
    (523, 200), (659, 400), (587, 200), (523, 200),  // C5 E5 D5 C5
    (494, 600), (523, 200), (587, 400), (659, 400),  // B4. C5  D5  E5
    (523, 400), (440, 400), (440, 400), (  0, 400),  // C5  A4  A4  rest

    (587, 600), (698, 200), (880, 400), (784, 200), (698, 200),  // D5.  F5  A5  G5  F5
    (659, 600), (523, 200), (659, 400), (587, 200), (523, 200),  // E5.  C5  E5  D5  C5
    (494, 400), (494, 200), (523, 200), (587, 400), (659, 400),  // B4  B4  C5  D5  E5
    (523, 400), (440, 400), (440, 400), (  0, 400),              // C5  A4  A4  rest
];

// --- Channel 2: Bass (octave-alternating root notes) on TIM4_CH1 / PB6 ---
//
// 16 entries × 800 ms = 12.8 s — matches MELODY length exactly so the two
// loops stay phase-locked. Per bar (1600 ms), bass plays two long notes
// roughly outlining the Am tonality of Tetris.
const BASS: Track = &[
    // Bar 1: Am  →  A2 - E3
    (110, 800), (165, 800),
    // Bar 2: Am  →  A2 - E3
    (110, 800), (165, 800),
    // Bar 3: passing E  →  E2 - B2
    ( 82, 800), (123, 800),
    // Bar 4: Am  →  A2 - E3
    (110, 800), (165, 800),
    // Bar 5: Dm  →  D2 - A2
    ( 73, 800), (110, 800),
    // Bar 6: C/G  →  C3 - G2
    (131, 800), ( 98, 800),
    // Bar 7: Am back  →  A2 - E3
    (110, 800), (165, 800),
    // Bar 8: Am  →  A2 - E3
    (110, 800), (165, 800),
];

// --- Helper: shared task body that drives any TIMx_CH1 as a square-wave voice ---
//
// We can't easily share this across generic timer types in async fns because
// embassy uses different concrete types per TIM, so we duplicate the loop in
// the two tasks below. The duplicated body is small.

// Vibrato: small periodic frequency wobble on top of the note. Triangle LFO
// in cents (1 semitone = 100 cents, so ±10 cents = subtle "expressive" wobble).
// 16 entries × 10 ms = 160 ms full LFO cycle ≈ 6.25 Hz, classic vibrato rate.
const VIBRATO_CENTS: [i32; 16] = [
    0, 3, 6, 9, 10, 9, 6, 3,
    0, -3, -6, -9, -10, -9, -6, -3,
];
const VIBRATO_STEP_MS: u64 = 10;

// Convert "cents offset" to a frequency multiplier as fixed-point ratio over 10000.
// 1 cent ≈ 0.0578% — close enough to (1 + cents * 6 / 10000) for tiny offsets.
#[inline]
fn vibrato_freq(base_hz: u32, cents: i32) -> u32 {
    let scaled = 10_000_i64 + cents as i64 * 6;
    (base_hz as i64 * scaled / 10_000) as u32
}

#[embassy_executor::task]
async fn melody_task(tim: TIM3, pin: PA6) -> ! {
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
        for &(freq, dur_ms) in MELODY {
            let tone_ms = (dur_ms as u64).saturating_sub(NOTE_GAP_MS);
            if freq == 0 {
                pwm.ch1().set_duty_cycle(0);
                Timer::after(Duration::from_millis(tone_ms)).await;
            } else {
                // Step through LFO entries until tone_ms is consumed.
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
async fn bass_task(tim: TIM4, pin: PB6) -> ! {
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
        for &(freq, dur_ms) in BASS {
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

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("3-channel pocket synth: LED (PA8) | Melody (PA6) | Bass (PB6)");

    spawner.spawn(led_task(p.TIM1, p.PA8, p.DMA2_CH5)).unwrap();
    spawner.spawn(melody_task(p.TIM3, p.PA6)).unwrap();
    spawner.spawn(bass_task(p.TIM4, p.PB6)).unwrap();
}
