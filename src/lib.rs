//! Pure-logic crate half: data + functions that don't touch hardware.
//! Tested on the host via `cargo test`. The `bin` half (main + voice + light_show)
//! re-uses these modules but adds the embedded-only glue around them.

#![no_std]

// Tests opt into std (assertions, proptest, formatting).
#[cfg(test)]
extern crate std;

pub mod songs;
pub mod util;
