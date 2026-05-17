#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

// `songs` and `util` come from the lib half of this same package
// (see Cargo.toml [lib]); only `voice` and `light_show` are bin-only modules.
mod light_show;
mod voice;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Pocket synth: LED (PA8) | Melody (PA6) | Bass (PB6)");

    spawner.spawn(light_show::led_task(p.TIM1, p.PA8)).unwrap();
    spawner.spawn(voice::melody_task(p.TIM3, p.PA6)).unwrap();
    spawner.spawn(voice::bass_task(p.TIM4, p.PB6)).unwrap();
}
