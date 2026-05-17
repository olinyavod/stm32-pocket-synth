//! Songs as note arrays. Each entry is `(freq_hz, duration_ms)`. `freq = 0`
//! means a rest. The voice tasks loop through these forever.

use crate::util::track_total_ms;

pub type Track = &'static [(u32, u32)];

// Tetris A-theme (Korobeiniki). Approximate timings at ~150 BPM:
//   eighth  = 200 ms
//   quarter = 400 ms
//   dotted quarter = 600 ms
pub static TETRIS_MELODY: Track = &[
    (659, 400), (494, 200), (523, 200), (587, 400),  // E5 B4 C5 D5
    (523, 200), (494, 200), (440, 400), (440, 200),  // C5 B4 A4 A4
    (523, 200), (659, 400), (587, 200), (523, 200),  // C5 E5 D5 C5
    (494, 600), (523, 200), (587, 400), (659, 400),  // B4. C5  D5  E5
    (523, 400), (440, 400), (440, 400), (  0, 400),  // C5  A4  A4  rest

    (587, 600), (698, 200), (880, 400), (784, 200), (698, 200),  // D5.  F5  A5  G5  F5
    (659, 600), (523, 200), (659, 400), (587, 200), (523, 200),  // E5.  C5  E5  D5  C5
    (494, 400), (494, 200), (523, 200), (587, 400), (659, 400),  // B4  B4  C5  D5  E5
    (523, 400), (440, 400), (440, 400), (  0, 400),              // C5  A4  A4  rest
];

// Bass: octave-alternating root notes, 16 entries × 800 ms = 12.8 s.
// Stays phase-locked to TETRIS_MELODY (same total duration).
pub static TETRIS_BASS: Track = &[
    (110, 800), (165, 800),  // Bar 1: Am  →  A2 — E3
    (110, 800), (165, 800),  // Bar 2: Am  →  A2 — E3
    ( 82, 800), (123, 800),  // Bar 3: E   →  E2 — B2
    (110, 800), (165, 800),  // Bar 4: Am
    ( 73, 800), (110, 800),  // Bar 5: Dm  →  D2 — A2
    (131, 800), ( 98, 800),  // Bar 6: C/G →  C3 — G2
    (110, 800), (165, 800),  // Bar 7: Am back
    (110, 800), (165, 800),  // Bar 8: Am
];

// Compile-time sanity check: melody and bass must have the same total duration
// so they stay phase-locked when looped forever. Editing one without the other
// will fail the build here with a clear message.
const _: () = assert!(
    track_total_ms(TETRIS_MELODY) == track_total_ms(TETRIS_BASS),
    "TETRIS_MELODY and TETRIS_BASS must have equal total duration",
);
