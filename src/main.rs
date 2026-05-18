#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::Config;
use embassy_stm32::rcc::{
    AHBPrescaler, APBPrescaler, Hse, HseMode, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv, PllSource,
    Sysclk,
};
use embassy_stm32::time::Hertz;
use {defmt_rtt as _, panic_probe as _};

// `songs` and `util` come from the lib half of this same package
// (see Cargo.toml [lib]); only `voice`, `light_show`, `usb` and `button` are bin-only modules.
mod button;
mod light_show;
mod usb;
mod voice;

/// Compile-time-generated constants and tables (see build.rs).
/// Single source of truth — included once, used from light_show and usb.
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/lut.rs"));
}

/// Clock tree for HSE+PLL: 25 MHz crystal → 96 MHz SYSCLK and 48 MHz USB.
///
///   HSE 25 MHz  /25 (prediv)  → 1 MHz
///               ×192 (mul)    → 192 MHz VCO
///               /2  (divp)    → 96 MHz   ← SYSCLK
///               /4  (divq)    → 48 MHz   ← USB
///   AHB /1  → HCLK = 96 MHz
///   APB1 /2 → PCLK1 = 48 MHz  (TIM2..7 run at 96 MHz: PCLK1 × 2)
///   APB2 /1 → PCLK2 = 96 MHz  (TIM1, 9, 10, 11 run at 96 MHz)
fn clock_config() -> Config {
    let mut config = Config::default();
    config.rcc.hsi = false;
    config.rcc.hse = Some(Hse {
        freq: Hertz(25_000_000),
        mode: HseMode::Oscillator,
    });
    config.rcc.pll_src = PllSource::HSE;
    config.rcc.pll = Some(Pll {
        prediv: PllPreDiv::DIV25,
        mul: PllMul::MUL192,
        divp: Some(PllPDiv::DIV2),
        divq: Some(PllQDiv::DIV4),
        divr: None,
    });
    config.rcc.sys = Sysclk::PLL1_P;
    config.rcc.ahb_pre = AHBPrescaler::DIV1;
    config.rcc.apb1_pre = APBPrescaler::DIV2;
    config.rcc.apb2_pre = APBPrescaler::DIV1;
    config
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(clock_config());
    info!("Pocket synth: SYSCLK 96 MHz, USB clk 48 MHz | LED PA8 | Melody PA6 | Bass PB6");

    spawner.spawn(light_show::led_task(p.TIM1, p.PA8)).unwrap();
    spawner.spawn(voice::voice0_task(p.TIM3, p.PA6)).unwrap();
    spawner.spawn(voice::voice1_task(p.TIM4, p.PB6)).unwrap();
    spawner.spawn(button::button_task(p.PA0, p.PC13)).unwrap();
    spawner
        .spawn(usb::usb_task(p.USB_OTG_FS, p.PA12, p.PA11))
        .unwrap();
}
