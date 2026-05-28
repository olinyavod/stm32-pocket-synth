//! USB-MIDI device. The BlackPill appears on the PC as a class-compliant
//! USB MIDI device. Incoming Note-On / Note-Off messages are routed through
//! a small voice allocator to whichever of the two physical voices is free.

use defmt::*;
use embassy_stm32::bind_interrupts;
use embassy_stm32::peripherals::{PA11, PA12, USB_OTG_FS};
use embassy_stm32::usb::{Driver, InterruptHandler};
use embassy_usb::class::midi::MidiClass;
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, Config as UsbConfig};

use crate::generated::MIDI_NOTE_HZ;
use crate::voice::{dispatch, Allocator, VoiceCmd};

bind_interrupts!(pub struct Irqs {
    OTG_FS => InterruptHandler<USB_OTG_FS>;
});

#[embassy_executor::task]
pub async fn usb_task(periph: USB_OTG_FS, dp: PA12, dm: PA11) -> ! {
    static mut EP_OUT_BUFFER: [u8; 256] = [0; 256];

    let usb_cfg = {
        let config = embassy_stm32::usb::Config::default();
        #[cfg(feature = "mcu-stm32h7")]
        {
            let mut config = config;
            config.vbus_detection = true;
            config
        }
        #[cfg(not(feature = "mcu-stm32h7"))]
        {
            config
        }
    };

    let driver = Driver::new_fs(
        periph,
        Irqs,
        dp,
        dm,
        unsafe { &mut *core::ptr::addr_of_mut!(EP_OUT_BUFFER) },
        usb_cfg,
    );

    let mut config = UsbConfig::new(0xC0DE, 0xCAFE);
    config.manufacturer = Some("olinyavod");
    config.product = Some("Pocket Synth");
    config.serial_number = Some("STM32-POCKET-1");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    static mut CONFIG_DESC: [u8; 256] = [0; 256];
    static mut BOS_DESC: [u8; 256] = [0; 256];
    static mut MSOS_DESC: [u8; 128] = [0; 128];
    static mut CONTROL_BUF: [u8; 64] = [0; 64];

    let mut builder = Builder::new(
        driver,
        config,
        unsafe { &mut *core::ptr::addr_of_mut!(CONFIG_DESC) },
        unsafe { &mut *core::ptr::addr_of_mut!(BOS_DESC) },
        unsafe { &mut *core::ptr::addr_of_mut!(MSOS_DESC) },
        unsafe { &mut *core::ptr::addr_of_mut!(CONTROL_BUF) },
    );

    let mut class = MidiClass::new(&mut builder, 1, 1, 64);
    let mut usb = builder.build();

    let usb_fut = usb.run();
    let midi_fut = async {
        loop {
            class.wait_connection().await;
            info!("USB-MIDI connected");
            let _ = pump_midi(&mut class).await;
            info!("USB-MIDI disconnected");
            // Silence both voices in case a note was held when the host went away.
            dispatch(0, VoiceCmd::AllOff);
            dispatch(1, VoiceCmd::AllOff);
        }
    };
    embassy_futures::join::join(usb_fut, midi_fut).await;
    core::unreachable!()
}

async fn pump_midi<'d, D: embassy_usb::driver::Driver<'d>>(
    class: &mut MidiClass<'d, D>,
) -> Result<(), EndpointError> {
    let mut buf = [0u8; 64];
    let mut alloc = Allocator::new();
    loop {
        let n = class.read_packet(&mut buf).await?;
        // Each USB-MIDI event is 4 bytes; iterate any number that fit in the packet.
        let mut i = 0;
        while i + 4 <= n {
            let status = buf[i + 1];
            let note = buf[i + 2];
            let velocity = buf[i + 3];
            handle_event(&mut alloc, status, note, velocity);
            i += 4;
        }
    }
}

#[inline]
fn handle_event(alloc: &mut Allocator, status: u8, note: u8, velocity: u8) {
    let event = status & 0xF0;
    if note >= 128 {
        return;
    }
    match event {
        0x90 if velocity > 0 => {
            // Note-On
            let freq = MIDI_NOTE_HZ[note as usize];
            let idx = alloc.note_on(note);
            dispatch(idx, VoiceCmd::NoteOn { note, freq_hz: freq });
        }
        0x80 | 0x90 => {
            // Note-Off (or velocity-0 Note-On)
            if let Some(idx) = alloc.note_off(note) {
                dispatch(idx, VoiceCmd::NoteOff { note });
            }
        }
        0xB0 if note == 123 => {
            // CC #123 = All Notes Off (panic button)
            alloc.all_off();
            dispatch(0, VoiceCmd::AllOff);
            dispatch(1, VoiceCmd::AllOff);
        }
        _ => {}
    }
}
