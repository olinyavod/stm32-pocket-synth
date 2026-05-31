//! Compile-time board pin map.
//!
//! Embassy pins and timers are owned, strongly typed peripherals, so this map
//! uses type aliases and one board-specific spawn function instead of plain
//! runtime constants.

use embassy_executor::Spawner;
use embassy_stm32::Peripherals;

#[cfg(feature = "mcu-stm32f411ce")]
pub type LedPin = embassy_stm32::peripherals::PA8;
pub type LedTimer = embassy_stm32::peripherals::TIM1;

#[cfg(feature = "mcu-stm32f411ce")]
pub type Voice0Pin = embassy_stm32::peripherals::PA6;
#[cfg(feature = "mcu-stm32f411ce")]
pub type Voice0Timer = embassy_stm32::peripherals::TIM3;

#[cfg(feature = "mcu-stm32h7")]
pub type LedPin = embassy_stm32::peripherals::PE9;
#[cfg(feature = "mcu-stm32h7")]
pub type AudioPin = embassy_stm32::peripherals::PA3;
#[cfg(feature = "mcu-stm32h7")]
pub type AudioTimer = embassy_stm32::peripherals::TIM2;

#[cfg(feature = "mcu-stm32f411ce")]
pub type Voice1Pin = embassy_stm32::peripherals::PB7;
#[cfg(feature = "mcu-stm32f411ce")]
pub type Voice1Timer = embassy_stm32::peripherals::TIM4;

#[cfg(feature = "mcu-stm32f411ce")]
pub type ButtonPin = embassy_stm32::peripherals::PA0;
#[cfg(feature = "mcu-stm32f411ce")]
pub type ButtonLedPin = embassy_stm32::peripherals::PC13;

#[cfg(feature = "mcu-stm32h7")]
pub type ButtonPin = embassy_stm32::peripherals::PC13;
#[cfg(feature = "mcu-stm32h7")]
pub type ButtonLedPin = embassy_stm32::peripherals::PB0;

pub type UsbPeripheral = embassy_stm32::peripherals::USB_OTG_FS;
pub type UsbDpPin = embassy_stm32::peripherals::PA12;
pub type UsbDmPin = embassy_stm32::peripherals::PA11;

#[cfg(feature = "mcu-stm32f411ce")]
pub const BOOT_LOG: &str =
    "Pocket synth STM32F411CE: SYSCLK 96 MHz, USB clk 48 MHz | LED PA8 | Melody PA6 | Bass PB7";

#[cfg(feature = "mcu-stm32h7")]
pub const BOOT_LOG: &str = "Pocket synth STM32H743ZI: HSI SYSCLK, USB clock PLL3_Q 48 MHz from STLINK MCO | LED D6/PE9 | Mono audio A0/PA3";

#[cfg(feature = "mcu-stm32f411ce")]
pub fn spawn_board_tasks(spawner: &Spawner, p: Peripherals) {
    spawner
        .spawn(crate::light_show::led_task(p.TIM1, p.PA8))
        .unwrap();
    spawner
        .spawn(crate::voice::voice0_task(p.TIM3, p.PA6))
        .unwrap();
    spawn_shared_tasks(
        spawner,
        p.TIM4,
        p.PB7,
        p.PA0,
        p.PC13,
        p.USB_OTG_FS,
        p.PA12,
        p.PA11,
    );
}

#[cfg(feature = "mcu-stm32h7")]
pub fn spawn_board_tasks(spawner: &Spawner, p: Peripherals) {
    spawner
        .spawn(crate::light_show::led_task(p.TIM1, p.PE9))
        .unwrap();
    spawner
        .spawn(crate::voice::audio_task(p.TIM2, p.PA3))
        .unwrap();
    spawn_control_tasks(spawner, p.PC13, p.PB0, p.USB_OTG_FS, p.PA12, p.PA11);
}

#[cfg(feature = "mcu-stm32f411ce")]
fn spawn_shared_tasks(
    spawner: &Spawner,
    voice1_timer: Voice1Timer,
    voice1_pin: Voice1Pin,
    button_pin: ButtonPin,
    button_led_pin: ButtonLedPin,
    usb: UsbPeripheral,
    usb_dp: UsbDpPin,
    usb_dm: UsbDmPin,
) {
    spawner
        .spawn(crate::voice::voice1_task(voice1_timer, voice1_pin))
        .unwrap();
    spawn_control_tasks(spawner, button_pin, button_led_pin, usb, usb_dp, usb_dm);
}

fn spawn_control_tasks(
    spawner: &Spawner,
    button_pin: ButtonPin,
    button_led_pin: ButtonLedPin,
    usb: UsbPeripheral,
    usb_dp: UsbDpPin,
    usb_dm: UsbDmPin,
) {
    spawner
        .spawn(crate::button::button_task(button_pin, button_led_pin))
        .unwrap();
    spawner
        .spawn(crate::usb::usb_task(usb, usb_dp, usb_dm))
        .unwrap();
}
