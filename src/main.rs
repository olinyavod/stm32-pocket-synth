#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::peripherals::{DMA2_CH5, PA6, PA8, TIM1, TIM3};
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

// --- Audio: chiptune synth on TIM3_CH1 (PA6) ---
//
// Classic 80s sound: the PWM hardware itself is the oscillator. For each note
// we set TIM3 PWM frequency = note pitch and duty = 50% — that's a pure square
// wave coming out of PA6, which is exactly what Game Boy / NES sound channels
// produced. No LUT, no DMA, no phase accumulator. CPU does ~0% work.

const TETRIS: &[(u32, u32)] = &[
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

// Brief silence between notes so two same-pitch notes are heard as separate
// strikes rather than one continuous tone. Same trick Game Boy used.
const NOTE_GAP_MS: u64 = 15;

#[embassy_executor::task]
async fn audio_task(tim: TIM3, pin: PA6) -> ! {
    let p = PwmPin::new_ch1(pin, OutputType::PushPull);
    // Start at an arbitrary frequency; we'll reprogram it for each note.
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
    // 0% duty = silence at boot.
    pwm.ch1().set_duty_cycle(0);

    loop {
        for &(freq, dur_ms) in TETRIS {
            let tone_ms = (dur_ms as u64).saturating_sub(NOTE_GAP_MS);

            if freq == 0 {
                // Rest — keep pin low.
                pwm.ch1().set_duty_cycle(0);
            } else {
                pwm.set_frequency(Hertz(freq));
                // max_duty_cycle changes with frequency (ARR is reprogrammed),
                // so re-query it every note. 50% duty = symmetric square wave.
                let mid = pwm.max_duty_cycle() / 2;
                pwm.ch1().set_duty_cycle(mid);
            }
            Timer::after(Duration::from_millis(tone_ms)).await;

            // Inter-note silence.
            pwm.ch1().set_duty_cycle(0);
            Timer::after(Duration::from_millis(NOTE_GAP_MS)).await;
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("LED on PA8 ({} Hz PWM) | Chiptune synth on PA6 (square wave)", LED_PWM_FREQ_HZ);

    spawner.spawn(led_task(p.TIM1, p.PA8, p.DMA2_CH5)).unwrap();
    spawner.spawn(audio_task(p.TIM3, p.PA6)).unwrap();
}
