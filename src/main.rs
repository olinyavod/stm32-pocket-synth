#![no_std]
#![no_main]

#[cfg(all(feature = "mcu-stm32f411ce", feature = "mcu-stm32h7"))]
compile_error!("Select exactly one MCU feature: mcu-stm32f411ce or mcu-stm32h7.");

#[cfg(not(any(feature = "mcu-stm32f411ce", feature = "mcu-stm32h7")))]
compile_error!("Select one MCU feature: mcu-stm32f411ce or mcu-stm32h7.");

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::Config;

#[cfg(feature = "mcu-stm32f411ce")]
use embassy_stm32::rcc::{
    AHBPrescaler, APBPrescaler, Hse, HseMode, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv, PllSource,
    Sysclk,
};

#[cfg(feature = "mcu-stm32f411ce")]
use embassy_stm32::time::Hertz;
use {defmt_rtt as _, panic_probe as _};

// `songs` and `util` come from the lib half of this same package
// (see Cargo.toml [lib]); only `voice`, `light_show`, `usb` and `button` are bin-only modules.
mod button;
mod light_show;
mod pinmap;
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
#[cfg(feature = "mcu-stm32f411ce")]
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

/// Portable STM32H7 bring-up clock for NUCLEO-H743ZI2.
///
/// The board's default HSE source is the 8 MHz MCO from STLINK-V3E. Keep SYSCLK
/// on Embassy's conservative default HSI, but derive USB's exact 48 MHz clock
/// from PLL3_Q.
#[cfg(feature = "mcu-stm32h7")]
fn clock_config() -> Config {
    use embassy_stm32::rcc::{Hse, HseMode, Pll, PllDiv, PllMul, PllPreDiv, PllSource};
    use embassy_stm32::time::Hertz;

    let mut config = Config::default();
    config.rcc.hse = Some(Hse {
        freq: Hertz(8_000_000),
        mode: HseMode::Bypass,
    });
    config.rcc.hsi48 = None;
    config.rcc.pll3 = Some(Pll {
        source: PllSource::HSE,
        prediv: PllPreDiv::DIV2,
        mul: PllMul::MUL120,
        divp: None,
        divq: Some(PllDiv::DIV10),
        divr: None,
    });
    config.rcc.mux.usbsel = embassy_stm32::rcc::mux::Usbsel::PLL3_Q;
    config
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(clock_config());
    info!("{}", pinmap::BOOT_LOG);
    pinmap::spawn_board_tasks(&spawner, p);
}
