#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::Channel;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use {defmt_rtt as _, panic_probe as _};

// LUT_LEN, PWM_FREQ_HZ, PWM_TOP, DUTY_LUT — all baked in by build.rs and
// placed in Flash (`.rodata`). No RAM cost, no boot-time computation.
include!(concat!(env!("OUT_DIR"), "/lut.rs"));

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = embassy_stm32::init(Default::default());
    info!(
        "Breathing LED: TIM1_CH1 on PA8, {} Hz PWM, {} samples → {} ms/cycle, LUT in Flash",
        PWM_FREQ_HZ,
        LUT_LEN,
        (LUT_LEN as u32 * 1000) / PWM_FREQ_HZ
    );

    let pin = PwmPin::new_ch1(p.PA8, OutputType::PushPull);
    let mut pwm = SimplePwm::new(
        p.TIM1,
        Some(pin),
        None,
        None,
        None,
        Hertz(PWM_FREQ_HZ),
        CountingMode::EdgeAlignedUp,
    );
    pwm.ch1().enable();

    // Sanity check: if the user switched clock config, the LUT will be off.
    // Fail loud at boot rather than silently drive wrong duty.
    let actual_top = pwm.max_duty_cycle();
    defmt::assert_eq!(
        actual_top,
        PWM_TOP,
        "max_duty_cycle mismatch — update TIMER_CLK_HZ / PWM_FREQ_HZ in build.rs"
    );

    let mut dma = p.DMA2_CH5;
    loop {
        // TIM1 Update event → DMA2_CH5 → CCR1. CPU sleeps on WFI between cycles.
        pwm.waveform_up(&mut dma, Channel::Ch1, &DUTY_LUT).await;
    }
}
