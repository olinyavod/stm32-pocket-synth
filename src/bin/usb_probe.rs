#![no_std]
#![no_main]

#[cfg(not(feature = "mcu-stm32h7"))]
compile_error!("usb_probe is intended for the NUCLEO-H743ZI2 profile. Build with --no-default-features --features mcu-stm32h7.");

use core::mem::MaybeUninit;

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::bind_interrupts;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::peripherals::{PA11, PA12, PB0, USB_OTG_FS};
use embassy_stm32::rcc::{Hse, HseMode, Pll, PllDiv, PllMul, PllPreDiv, PllSource};
use embassy_stm32::time::Hertz;
use embassy_stm32::usb::{Driver, InterruptHandler};
use embassy_stm32::Config;
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State as CdcState, USB_CLASS_CDC};
use embassy_usb::{Builder, Config as UsbConfig, UsbVersion};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    OTG_FS => InterruptHandler<USB_OTG_FS>;
});

fn clock_config() -> Config {
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
    info!("USB probe CDC boot: STM32H743ZI, USB clock PLL3_Q 48 MHz");

    spawner.spawn(led_task(p.PB0)).unwrap();
    spawner
        .spawn(usb_probe_task(p.USB_OTG_FS, p.PA12, p.PA11))
        .unwrap();
}

#[embassy_executor::task]
async fn led_task(pin: PB0) -> ! {
    let mut led = Output::new(pin, Level::Low, Speed::Low);
    loop {
        led.set_high();
        Timer::after_millis(100).await;
        led.set_low();
        Timer::after_millis(900).await;
    }
}

#[embassy_executor::task]
async fn usb_probe_task(periph: USB_OTG_FS, dp: PA12, dm: PA11) -> ! {
    static mut EP_OUT_BUFFER: [u8; 256] = [0; 256];

    let mut usb_cfg = embassy_stm32::usb::Config::default();
    usb_cfg.vbus_detection = true;

    let driver = Driver::new_fs(
        periph,
        Irqs,
        dp,
        dm,
        unsafe { &mut *core::ptr::addr_of_mut!(EP_OUT_BUFFER) },
        usb_cfg,
    );

    let mut config = UsbConfig::new(0xC0DE, 0x0001);
    config.bcd_usb = UsbVersion::Two;
    config.device_class = USB_CLASS_CDC;
    config.device_sub_class = 0;
    config.device_protocol = 0;
    config.composite_with_iads = false;
    config.manufacturer = Some("olinyavod");
    config.product = Some("USB Probe CDC");
    config.serial_number = Some("NUCLEO-H743ZI2-USB-PROBE");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    static mut CONFIG_DESC: [u8; 256] = [0; 256];
    static mut BOS_DESC: [u8; 64] = [0; 64];
    static mut MSOS_DESC: [u8; 64] = [0; 64];
    static mut CONTROL_BUF: [u8; 64] = [0; 64];
    static mut CDC_STATE: MaybeUninit<CdcState<'static>> = MaybeUninit::uninit();

    let mut builder = Builder::new(
        driver,
        config,
        unsafe { &mut *core::ptr::addr_of_mut!(CONFIG_DESC) },
        unsafe { &mut *core::ptr::addr_of_mut!(BOS_DESC) },
        unsafe { &mut *core::ptr::addr_of_mut!(MSOS_DESC) },
        unsafe { &mut *core::ptr::addr_of_mut!(CONTROL_BUF) },
    );

    let cdc_state = unsafe { (&mut *core::ptr::addr_of_mut!(CDC_STATE)).write(CdcState::new()) };
    let mut class = CdcAcmClass::new(&mut builder, cdc_state, 64);
    let mut usb = builder.build();

    let usb_fut = usb.run();
    let cdc_fut = async {
        let mut buf = [0u8; 64];
        loop {
            class.wait_connection().await;
            info!("USB CDC connected");
            let _ = class.write_packet(b"USB probe ready\r\n").await;

            loop {
                match class.read_packet(&mut buf).await {
                    Ok(n) => {
                        info!("USB CDC RX {} bytes", n);
                        if n != 0 {
                            let _ = class.write_packet(&buf[..n]).await;
                        }
                    }
                    Err(_) => {
                        info!("USB CDC disconnected");
                        break;
                    }
                }
            }
        }
    };

    embassy_futures::join::join(usb_fut, cdc_fut).await;
    core::unreachable!()
}
