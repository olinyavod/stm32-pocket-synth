//! LED light-show — reacts to melody notes published by [`voice::MELODY_NOTE`].
//!
//! On every note attack we flash the LED to a brightness peak that depends on
//! pitch (higher note → brighter), then linearly fade over the note's duration.
//! On a rest the LED is dark.

use embassy_stm32::gpio::OutputType;
use embassy_stm32::peripherals::{PA8, TIM1};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_time::{Duration, Timer};

use black_sitizator::util::peak_brightness;
use crate::voice::MELODY_NOTE;

// Pulled in via build.rs (constants generated from the LED PWM frequency).
include!(concat!(env!("OUT_DIR"), "/lut.rs"));

/// Pitch range used to map note frequency → peak brightness.
/// Notes below LOW_HZ get min brightness; above HIGH_HZ get max.
const LOW_HZ: u32 = 200;
const HIGH_HZ: u32 = 900;

/// Number of linear-fade steps from peak brightness down to off. More steps =
/// smoother fade. 24 steps over a 200 ms note ≈ 8 ms per step (visible smooth).
const FADE_STEPS: u64 = 24;

#[embassy_executor::task]
pub async fn led_task(tim: TIM1, pin: PA8) -> ! {
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
    pwm.ch1().set_duty_cycle(0);
    let max = pwm.max_duty_cycle() as u32;

    loop {
        let (freq, tone_ms) = MELODY_NOTE.wait().await;
        if freq == 0 {
            // Rest — LED stays dark until the next note.
            pwm.ch1().set_duty_cycle(0);
            continue;
        }
        // Map pitch to peak brightness, clamped to [12.5 %, 100 %] of max duty.
        let peak = peak_brightness(freq, LOW_HZ, HIGH_HZ, max);

        // Linear fade from peak → 0 over tone_ms.
        let step_ms = (tone_ms / FADE_STEPS).max(1);
        for i in 0..FADE_STEPS {
            let duty = (peak * (FADE_STEPS - i) as u32 / FADE_STEPS as u32) as u16;
            pwm.ch1().set_duty_cycle(duty);
            Timer::after(Duration::from_millis(step_ms)).await;
        }
        pwm.ch1().set_duty_cycle(0);
    }
}
