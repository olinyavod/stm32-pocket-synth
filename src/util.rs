//! Pure-logic helpers used by the voice and light-show tasks. Living here
//! (not inside the embedded modules) means they can be unit-tested on the
//! host with `cargo test`.

/// Apply a vibrato cents offset to a base frequency.
///
/// 1 cent ≈ 0.0578 %, so for small offsets (`|cents|` ≤ ~100) the multiplier
/// `1 + cents · 6 / 10000` is a tight linear approximation.
#[inline]
pub fn vibrato_freq(base_hz: u32, cents: i32) -> u32 {
    let scaled = 10_000_i64 + cents as i64 * 6;
    (base_hz as i64 * scaled / 10_000) as u32
}

/// Map a note frequency to peak LED brightness via linear interpolation over
/// `[low_hz, high_hz]`, clamped to `[max_duty / 8, max_duty]` so even the
/// quietest note stays visible.
///
/// Computed in `u64` internally to survive large `max_duty` without overflow
/// (the previous `max_duty * 7 / 8 * offset` could wrap a `u32` for
/// `max_duty` above ~6·10⁸).
#[inline]
pub fn peak_brightness(freq_hz: u32, low_hz: u32, high_hz: u32, max_duty: u32) -> u32 {
    let f = freq_hz.clamp(low_hz, high_hz);
    let max = max_duty as u64;
    let span = (high_hz - low_hz) as u64;
    let offset = (f - low_hz) as u64;
    let min_b = max / 8;
    let scaled = max * 7 / 8;
    (min_b + scaled * offset / span) as u32
}

/// Total duration of a track in milliseconds. `const fn` so we can use it for
/// compile-time assertions like "melody and bass have matching lengths".
pub const fn track_total_ms(track: &[(u32, u32)]) -> u64 {
    let mut total: u64 = 0;
    let mut i = 0;
    while i < track.len() {
        total += track[i].1 as u64;
        i += 1;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // --- vibrato_freq ---

    #[test]
    fn vibrato_zero_cents_is_identity() {
        for freq in [110u32, 220, 440, 880] {
            assert_eq!(vibrato_freq(freq, 0), freq);
        }
    }

    #[test]
    fn vibrato_positive_cents_raises_freq() {
        assert!(vibrato_freq(440, 10) > 440);
        assert!(vibrato_freq(880, 50) > 880);
    }

    #[test]
    fn vibrato_negative_cents_lowers_freq() {
        assert!(vibrato_freq(440, -10) < 440);
        assert!(vibrato_freq(880, -50) < 880);
    }

    // --- peak_brightness ---

    #[test]
    fn peak_brightness_clamps_low_to_minimum() {
        // freq < low_hz → min brightness = max/8
        let max = 16_000u32;
        assert_eq!(peak_brightness(50, 200, 900, max), max / 8);
    }

    #[test]
    fn peak_brightness_clamps_high_to_maximum() {
        // freq > high_hz → full brightness = max
        let max = 16_000u32;
        assert_eq!(peak_brightness(2000, 200, 900, max), max);
    }

    #[test]
    fn peak_brightness_at_low_endpoint_is_min() {
        let max = 16_000u32;
        assert_eq!(peak_brightness(200, 200, 900, max), max / 8);
    }

    #[test]
    fn peak_brightness_at_high_endpoint_is_max() {
        let max = 16_000u32;
        assert_eq!(peak_brightness(900, 200, 900, max), max);
    }

    // --- track_total_ms ---

    #[test]
    fn track_total_ms_empty_is_zero() {
        let empty: &[(u32, u32)] = &[];
        assert_eq!(track_total_ms(empty), 0);
    }

    #[test]
    fn track_total_ms_sums_durations() {
        let t: &[(u32, u32)] = &[(440, 100), (494, 200), (523, 300)];
        assert_eq!(track_total_ms(t), 600);
    }

    #[test]
    fn track_total_ms_ignores_frequencies() {
        // Same total whether the frequencies are A4 or A5 — only durations matter.
        let t1: &[(u32, u32)] = &[(440, 200), (440, 200)];
        let t2: &[(u32, u32)] = &[(880, 200), (110, 200)];
        assert_eq!(track_total_ms(t1), track_total_ms(t2));
    }

    // --- Property-based tests ---

    proptest! {
        /// vibrato_freq should always stay within ±7 % of base for cents in [-100, 100].
        #[test]
        fn prop_vibrato_within_seven_percent(
            freq in 80u32..2_000,
            cents in -100i32..100,
        ) {
            let result = vibrato_freq(freq, cents);
            let lower = freq * 93 / 100;
            let upper = freq * 107 / 100 + 1;
            prop_assert!(result >= lower, "freq={} cents={} result={} lower={}", freq, cents, result, lower);
            prop_assert!(result <= upper, "freq={} cents={} result={} upper={}", freq, cents, result, upper);
        }

        /// Sign of vibrato matches sign of cents — but only when the product
        /// `freq · cents · 6` is large enough that integer division does not
        /// round back to the base frequency. For our music range (freq ≥ 200,
        /// |cents| ≥ 20) the change is always at least a couple of Hz.
        #[test]
        fn prop_vibrato_sign_follows_cents(
            freq in 200u32..2_000,
            cents in 20i32..100,
        ) {
            prop_assert!(vibrato_freq(freq, cents) > freq);
            prop_assert!(vibrato_freq(freq, -cents) < freq);
        }

        /// peak_brightness output is always within [max/8, max].
        #[test]
        fn prop_peak_brightness_bounded(
            freq in 0u32..10_000,
            low in 50u32..500,
            high in 600u32..2_000,
            max in 100u32..u32::MAX / 16,
        ) {
            prop_assume!(low < high);
            let b = peak_brightness(freq, low, high, max);
            prop_assert!(b >= max / 8, "below min: b={} max/8={}", b, max / 8);
            prop_assert!(b <= max, "above max: b={} max={}", b, max);
        }

        /// peak_brightness is monotonically non-decreasing in frequency.
        #[test]
        fn prop_peak_brightness_monotonic_in_freq(
            f1 in 100u32..2_000,
            f2 in 100u32..2_000,
        ) {
            let max = 16_000u32;
            let (low, high) = (200u32, 900u32);
            let b1 = peak_brightness(f1, low, high, max);
            let b2 = peak_brightness(f2, low, high, max);
            if f1 <= f2 {
                prop_assert!(b1 <= b2);
            } else {
                prop_assert!(b1 >= b2);
            }
        }

        /// track_total_ms equals the literal sum of durations.
        #[test]
        fn prop_track_total_ms_equals_sum(durs in proptest::collection::vec(0u32..10_000, 0..50)) {
            let track: std::vec::Vec<(u32, u32)> = durs.iter().map(|&d| (440u32, d)).collect();
            let expected: u64 = durs.iter().map(|&d| d as u64).sum();
            prop_assert_eq!(track_total_ms(&track), expected);
        }
    }
}
