//! USB-CDC echo. The board appears to the PC as a virtual COM port; anything
//! you type into PuTTY / `screen` / any terminal is echoed back.
//!
//! Stage 5B in the synth roadmap — confirms that USB OTG_FS is alive at the
//! new clock config before we layer a MIDI class on top of it.

use defmt::*;
use embassy_stm32::bind_interrupts;
use embassy_stm32::peripherals::{PA11, PA12, USB_OTG_FS};
use embassy_stm32::usb::{Driver, InterruptHandler};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, Config as UsbConfig};

bind_interrupts!(pub struct Irqs {
    OTG_FS => InterruptHandler<USB_OTG_FS>;
});

#[embassy_executor::task]
pub async fn usb_task(periph: USB_OTG_FS, dp: PA12, dm: PA11) -> ! {
    // 256-byte staging area for endpoint OUT (host → device) transfers. Sized
    // to comfortably hold our max packet size of 64.
    static mut EP_OUT_BUFFER: [u8; 256] = [0; 256];

    let mut usb_cfg = embassy_stm32::usb::Config::default();
    // BlackPill doesn't route VBUS to a dedicated sense pin, so disable
    // session-valid check or USB enumeration won't start.
    usb_cfg.vbus_detection = false;

    let driver = Driver::new_fs(
        periph,
        Irqs,
        dp,
        dm,
        // SAFETY: only this task accesses EP_OUT_BUFFER, and it lives for 'static.
        unsafe { &mut *core::ptr::addr_of_mut!(EP_OUT_BUFFER) },
        usb_cfg,
    );

    // USB device descriptor — what the host sees in Device Manager / `lsusb`.
    let mut config = UsbConfig::new(0xC0DE, 0xCAFE);
    config.manufacturer = Some("olinyavod");
    config.product = Some("Pocket Synth");
    config.serial_number = Some("STM32-POCKET-1");
    config.max_power = 100;
    config.max_packet_size_0 = 64;
    // Composite-class workaround for Windows enumeration (some hosts are picky
    // about single-interface CDC; this is the recommended embassy boilerplate).
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    // Static buffers for the USB descriptor builder.
    static mut CONFIG_DESC: [u8; 256] = [0; 256];
    static mut BOS_DESC: [u8; 256] = [0; 256];
    static mut MSOS_DESC: [u8; 128] = [0; 128];
    static mut CONTROL_BUF: [u8; 64] = [0; 64];

    let mut state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        unsafe { &mut *core::ptr::addr_of_mut!(CONFIG_DESC) },
        unsafe { &mut *core::ptr::addr_of_mut!(BOS_DESC) },
        unsafe { &mut *core::ptr::addr_of_mut!(MSOS_DESC) },
        unsafe { &mut *core::ptr::addr_of_mut!(CONTROL_BUF) },
    );

    let mut class = CdcAcmClass::new(&mut builder, &mut state, 64);
    let mut usb = builder.build();

    // Run the USB stack + echo loop concurrently.
    let usb_fut = usb.run();
    let echo_fut = async {
        loop {
            class.wait_connection().await;
            info!("USB-CDC connected");
            let _ = echo(&mut class).await;
            info!("USB-CDC disconnected");
        }
    };
    embassy_futures::join::join(usb_fut, echo_fut).await;
    core::unreachable!()
}

async fn echo<'d, T: embassy_usb::driver::Driver<'d>>(
    class: &mut CdcAcmClass<'d, T>,
) -> Result<(), Disconnected> {
    let mut buf = [0u8; 64];
    loop {
        let n = class.read_packet(&mut buf).await?;
        class.write_packet(&buf[..n]).await?;
    }
}

struct Disconnected;
impl From<EndpointError> for Disconnected {
    fn from(e: EndpointError) -> Self {
        match e {
            EndpointError::BufferOverflow => core::panic!("buffer overflow"),
            EndpointError::Disabled => Disconnected,
        }
    }
}
