/// Multi-level drowsiness alert system.
///
/// Escalation (consecutive drowsy frames, speed-scaled):
///   At reference speed V_REF = 60 km/h:
///     Alert1 after ~2 frames (200 ms)
///     Alert2 after ~3 frames (~300 ms)  -> ~5 m distance
///
/// Thresholds scale as V_REF / speed, clamped to a 2-frame floor
/// (the minimum that rejects single-sample sensor noise).
///
/// De-escalation (consecutive normal frames):
///   5  -> drop one level:  Alert2 -> Alert1,  Alert1 -> None

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AlertLevel {
    None,
    Alert1,
    Alert2,
}

pub struct AlertState {
    drowsy_count: u32,
    normal_count: u32,
    level: AlertLevel,
}

// ---- Speed-scaled threshold tuning ------------------------------------------
//
// Targets ~5 m distance-to-Alert2 across the speed range. Below 90 km/h the
// two alert levels are distinct; above 90 km/h both collapse to the 2-frame
// floor (can't go lower without triggering on single-sample noise).

/// Alert1 base frames at reference speed (effectively floored to 2).
pub const ALERT1_BASE: f32 = 1.5;
/// Alert2 base frames at reference speed (~5 m distance at 60 km/h).
pub const ALERT2_BASE: f32 = 3.0;
/// Reference speed (km/h) at which BASE frames apply.
pub const V_REF: f32 = 60.0;
/// Minimum frames (noise floor). 1 frame = single-sample, rejects nothing.
pub const MIN_FRAMES: u32 = 2;
/// Minimum speed used in scaling (avoid div-by-zero / infinite window at rest).
pub const MIN_SPEED_KMH: f32 = 10.0;

/// Derive (Alert1, Alert2) thresholds from current speed in km/h.
///
/// Returns consecutive-drowsy-frame counts. Always satisfies:
/// `MIN_FRAMES <= t1 <= t2`.
pub fn thresholds_for_speed(speed_kmh: f32) -> (u32, u32) {
    let v = if speed_kmh.is_finite() && speed_kmh > MIN_SPEED_KMH {
        speed_kmh
    } else {
        MIN_SPEED_KMH
    };
    let scale = V_REF / v;
    let t1_raw = (ALERT1_BASE * scale) as u32;
    let t2_raw = (ALERT2_BASE * scale) as u32;
    let t1 = if t1_raw < MIN_FRAMES { MIN_FRAMES } else { t1_raw };
    let t2 = if t2_raw < MIN_FRAMES { MIN_FRAMES } else { t2_raw };
    // Preserve t1 <= t2 invariant (both floored, Alert2 must not be stricter
    // than Alert1 after clamping).
    let t2 = if t2 < t1 { t1 } else { t2 };
    (t1, t2)
}

impl AlertState {
    pub fn new() -> Self {
        Self {
            drowsy_count: 0,
            normal_count: 0,
            level: AlertLevel::None,
        }
    }

    /// Feed one processed EEG frame using speed-independent defaults
    /// (Alert1 = 5, Alert2 = 10). Kept for backward compatibility with
    /// tests and callers that don't know the current vehicle speed.
    pub fn update(&mut self, is_drowsy: bool) -> AlertLevel {
        self.update_with_thresholds(is_drowsy, 5, 10)
    }

    /// Feed one processed EEG frame with explicit thresholds (typically
    /// derived from the current vehicle speed via `thresholds_for_speed`).
    ///
    /// Re-derives the alert level from the running counts every call, so
    /// changing `t1`/`t2` mid-escalation is safe and consistent.
    pub fn update_with_thresholds(
        &mut self,
        is_drowsy: bool,
        t1: u32,
        t2: u32,
    ) -> AlertLevel {
        // Enforce t1 <= t2 defensively.
        let t2 = if t2 < t1 { t1 } else { t2 };

        if is_drowsy {
            self.drowsy_count = self.drowsy_count.saturating_add(1);
            self.normal_count = 0;

            // Escalate: only move up, never demote here (de-escalation is
            // handled exclusively by the normal-frame branch below).
            if self.drowsy_count >= t2 {
                self.level = AlertLevel::Alert2;
            } else if self.drowsy_count >= t1 && self.level == AlertLevel::None {
                self.level = AlertLevel::Alert1;
            }
        } else {
            self.normal_count = self.normal_count.saturating_add(1);
            self.drowsy_count = 0;

            // De-escalate after 5 consecutive normal frames
            if self.normal_count >= 5 {
                self.normal_count = 0; // reset for next de-escalation window
                match self.level {
                    AlertLevel::Alert2 => self.level = AlertLevel::Alert1,
                    AlertLevel::Alert1 => self.level = AlertLevel::None,
                    AlertLevel::None => {}
                }
            }
        }

        self.level
    }

    pub fn level(&self) -> AlertLevel {
        self.level
    }

    pub fn drowsy_count(&self) -> u32 {
        self.drowsy_count
    }

    pub fn normal_count(&self) -> u32 {
        self.normal_count
    }

    pub fn level_label(&self) -> &'static str {
        match self.level {
            AlertLevel::None => "NONE",
            AlertLevel::Alert1 => "ALERT1",
            AlertLevel::Alert2 => "ALERT1+ALERT2",
        }
    }
}

// =============================================================================
//  Tests — run with `cargo test` on the host (x86/x64)
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    // ---- drowsiness ratio threshold (mirrors main.rs constant) ----
    const DROWSY_RATIO_THRESHOLD: f32 = 1.2;

    // =====================================================================
    //  1. AlertState unit tests
    // =====================================================================

    #[test]
    fn alert_initial_state() {
        let a = AlertState::new();
        assert_eq!(a.level(), AlertLevel::None);
        assert_eq!(a.drowsy_count(), 0);
        assert_eq!(a.normal_count(), 0);
        assert_eq!(a.level_label(), "NONE");
    }

    #[test]
    fn alert_single_drowsy_no_escalation() {
        let mut a = AlertState::new();
        a.update(true);
        assert_eq!(a.level(), AlertLevel::None);
        assert_eq!(a.drowsy_count(), 1);
    }

    #[test]
    fn alert_four_drowsy_still_none() {
        let mut a = AlertState::new();
        for _ in 0..4 {
            a.update(true);
        }
        assert_eq!(a.level(), AlertLevel::None);
        assert_eq!(a.drowsy_count(), 4);
    }

    #[test]
    fn alert_five_drowsy_escalates_to_alert1() {
        let mut a = AlertState::new();
        for _ in 0..5 {
            a.update(true);
        }
        assert_eq!(a.level(), AlertLevel::Alert1);
        assert_eq!(a.drowsy_count(), 5);
        assert_eq!(a.level_label(), "ALERT1");
    }

    #[test]
    fn alert_nine_drowsy_stays_alert1() {
        let mut a = AlertState::new();
        for _ in 0..9 {
            a.update(true);
        }
        assert_eq!(a.level(), AlertLevel::Alert1);
    }

    #[test]
    fn alert_ten_drowsy_escalates_to_alert2() {
        let mut a = AlertState::new();
        for _ in 0..10 {
            a.update(true);
        }
        assert_eq!(a.level(), AlertLevel::Alert2);
        assert_eq!(a.drowsy_count(), 10);
        assert_eq!(a.level_label(), "ALERT1+ALERT2");
    }

    #[test]
    fn alert_twenty_drowsy_stays_alert2() {
        let mut a = AlertState::new();
        for _ in 0..20 {
            a.update(true);
        }
        assert_eq!(a.level(), AlertLevel::Alert2);
        assert_eq!(a.drowsy_count(), 20);
    }

    #[test]
    fn alert_deescalation_from_alert1_to_none() {
        let mut a = AlertState::new();
        for _ in 0..5 {
            a.update(true);
        }
        assert_eq!(a.level(), AlertLevel::Alert1);
        // 5 consecutive normal → drops to None
        for _ in 0..5 {
            a.update(false);
        }
        assert_eq!(a.level(), AlertLevel::None);
        assert_eq!(a.normal_count(), 0); // reset after de-escalation
    }

    #[test]
    fn alert_deescalation_from_alert2_to_alert1() {
        let mut a = AlertState::new();
        for _ in 0..10 {
            a.update(true);
        }
        assert_eq!(a.level(), AlertLevel::Alert2);
        // 5 consecutive normal → drops ONE level to Alert1
        for _ in 0..5 {
            a.update(false);
        }
        assert_eq!(a.level(), AlertLevel::Alert1);
    }

    #[test]
    fn alert_full_deescalation_alert2_to_none() {
        let mut a = AlertState::new();
        for _ in 0..10 {
            a.update(true);
        }
        assert_eq!(a.level(), AlertLevel::Alert2);
        // 5 normal → Alert1
        for _ in 0..5 {
            a.update(false);
        }
        assert_eq!(a.level(), AlertLevel::Alert1);
        // 5 more normal → None
        for _ in 0..5 {
            a.update(false);
        }
        assert_eq!(a.level(), AlertLevel::None);
    }

    #[test]
    fn alert_interrupted_drowsy_streak_resets_count() {
        let mut a = AlertState::new();
        // 4 drowsy (not enough)
        for _ in 0..4 {
            a.update(true);
        }
        assert_eq!(a.level(), AlertLevel::None);
        assert_eq!(a.drowsy_count(), 4);
        // 1 normal breaks the streak
        a.update(false);
        assert_eq!(a.drowsy_count(), 0);
        assert_eq!(a.normal_count(), 1);
        // need full 5 again from scratch
        for _ in 0..4 {
            a.update(true);
        }
        assert_eq!(a.level(), AlertLevel::None);
        assert_eq!(a.drowsy_count(), 4);
        // 5th in a row → Alert1
        a.update(true);
        assert_eq!(a.level(), AlertLevel::Alert1);
    }

    #[test]
    fn alert_interrupted_normal_streak_no_deescalation() {
        let mut a = AlertState::new();
        // Reach Alert1
        for _ in 0..5 {
            a.update(true);
        }
        assert_eq!(a.level(), AlertLevel::Alert1);
        // 4 normal (not enough to de-escalate)
        for _ in 0..4 {
            a.update(false);
        }
        assert_eq!(a.level(), AlertLevel::Alert1);
        assert_eq!(a.normal_count(), 4);
        // 1 drowsy breaks the normal streak
        a.update(true);
        assert_eq!(a.level(), AlertLevel::Alert1);
        assert_eq!(a.normal_count(), 0);
        assert_eq!(a.drowsy_count(), 1);
    }

    #[test]
    fn alert_reescalation_after_deescalation() {
        let mut a = AlertState::new();
        // Alert1
        for _ in 0..5 {
            a.update(true);
        }
        assert_eq!(a.level(), AlertLevel::Alert1);
        // De-escalate to None
        for _ in 0..5 {
            a.update(false);
        }
        assert_eq!(a.level(), AlertLevel::None);
        // Re-escalate to Alert1 again
        for _ in 0..5 {
            a.update(true);
        }
        assert_eq!(a.level(), AlertLevel::Alert1);
    }

    #[test]
    fn alert_none_stays_none_on_normal() {
        let mut a = AlertState::new();
        for _ in 0..20 {
            a.update(false);
        }
        assert_eq!(a.level(), AlertLevel::None);
    }

    #[test]
    fn alert_update_returns_current_level() {
        let mut a = AlertState::new();
        assert_eq!(a.update(true), AlertLevel::None);   // 1st
        assert_eq!(a.update(true), AlertLevel::None);   // 2nd
        assert_eq!(a.update(true), AlertLevel::None);   // 3rd
        assert_eq!(a.update(true), AlertLevel::None);   // 4th
        assert_eq!(a.update(true), AlertLevel::Alert1); // 5th ← transition
        assert_eq!(a.update(true), AlertLevel::Alert1); // 6th
    }

    // =====================================================================
    //  1b. Speed-scaled threshold tests
    // =====================================================================

    #[test]
    fn thresholds_at_reference_speed() {
        // V_REF = 60 km/h: t1_raw=1.5→1→floored to 2, t2_raw=3→3
        let (t1, t2) = thresholds_for_speed(60.0);
        assert_eq!(t1, MIN_FRAMES);
        assert_eq!(t2, 3);
        assert!(t1 <= t2);
    }

    #[test]
    fn thresholds_at_highway_speed() {
        // 100 km/h: scale=0.6, t1_raw=0.9→0→2, t2_raw=1.8→1→2
        let (t1, t2) = thresholds_for_speed(100.0);
        assert_eq!(t1, MIN_FRAMES);
        assert_eq!(t2, MIN_FRAMES);
    }

    #[test]
    fn thresholds_at_low_speed() {
        // 30 km/h: scale=2.0, t1_raw=3, t2_raw=6
        let (t1, t2) = thresholds_for_speed(30.0);
        assert_eq!(t1, 3);
        assert_eq!(t2, 6);
    }

    #[test]
    fn thresholds_at_zero_speed_clamped() {
        // speed <= MIN_SPEED_KMH should use MIN_SPEED_KMH for scaling
        let (t1_zero, t2_zero) = thresholds_for_speed(0.0);
        let (t1_min, t2_min) = thresholds_for_speed(MIN_SPEED_KMH);
        assert_eq!(t1_zero, t1_min);
        assert_eq!(t2_zero, t2_min);
    }

    #[test]
    fn thresholds_with_nan_clamped() {
        // NaN speed must not propagate; falls back to MIN_SPEED_KMH behaviour
        let (t1, t2) = thresholds_for_speed(f32::NAN);
        let (t1_min, t2_min) = thresholds_for_speed(MIN_SPEED_KMH);
        assert_eq!(t1, t1_min);
        assert_eq!(t2, t2_min);
    }

    #[test]
    fn thresholds_invariant_t1_le_t2() {
        for speed in [5.0, 10.0, 30.0, 60.0, 90.0, 120.0, 200.0] {
            let (t1, t2) = thresholds_for_speed(speed);
            assert!(t1 >= MIN_FRAMES, "speed={}: t1={} below floor", speed, t1);
            assert!(t1 <= t2, "speed={}: t1={} > t2={}", speed, t1, t2);
        }
    }

    #[test]
    fn scaled_alert1_fires_at_highway_speed() {
        // At 120 km/h both thresholds are at MIN_FRAMES=2
        let (t1, t2) = thresholds_for_speed(120.0);
        let mut a = AlertState::new();
        assert_eq!(a.update_with_thresholds(true, t1, t2), AlertLevel::None);
        // 2nd frame: Alert1 (and Alert2, since t2==t1==2)
        let level = a.update_with_thresholds(true, t1, t2);
        assert_eq!(level, AlertLevel::Alert2);
    }

    #[test]
    fn scaled_alert_levels_distinct_at_low_speed() {
        // At 30 km/h: t1=3, t2=6 → distinct levels
        let (t1, t2) = thresholds_for_speed(30.0);
        assert_eq!((t1, t2), (3, 6));
        let mut a = AlertState::new();
        for _ in 0..2 {
            assert_eq!(a.update_with_thresholds(true, t1, t2), AlertLevel::None);
        }
        assert_eq!(a.update_with_thresholds(true, t1, t2), AlertLevel::Alert1); // 3rd
        for _ in 0..2 {
            assert_eq!(a.update_with_thresholds(true, t1, t2), AlertLevel::Alert1);
        }
        assert_eq!(a.update_with_thresholds(true, t1, t2), AlertLevel::Alert2); // 6th
    }

    #[test]
    fn mid_escalation_threshold_change_is_safe() {
        // Start at 30 km/h (t1=3). Reach 2 drowsy frames.
        // Speed drops to 10 km/h (t1=9). Next frame should NOT promote yet.
        let (t1_fast, t2_fast) = thresholds_for_speed(30.0); // (3, 6)
        let (t1_slow, t2_slow) = thresholds_for_speed(10.0); // scale=6, (9, 18)
        let mut a = AlertState::new();
        a.update_with_thresholds(true, t1_fast, t2_fast);
        a.update_with_thresholds(true, t1_fast, t2_fast);
        // dcnt=2, level=None. Now threshold gets harder.
        assert_eq!(
            a.update_with_thresholds(true, t1_slow, t2_slow),
            AlertLevel::None,
            "dcnt=3 but t1={} under slow thresholds -> no alert",
            t1_slow
        );
        assert_eq!(a.drowsy_count(), 3);
    }

    // =====================================================================
    //  2. EEG parser unit tests (inlined pure logic from eeg_sensor.rs)
    // =====================================================================

    /// Mirrors EegSensor::parse — "E,alpha,beta" → (alpha, beta).
    /// Beta clamped to 1e-3 min to prevent div-by-zero.
    fn parse_eeg(line: &[u8]) -> Option<(f32, f32)> {
        let s = core::str::from_utf8(line).ok()?;
        let rest = s.trim().strip_prefix("E,")?;
        let mut parts = rest.split(',');
        let alpha = parts.next()?.trim().parse::<f32>().ok()?;
        let beta_raw = parts.next()?.trim().parse::<f32>().ok()?;
        let beta = if beta_raw.abs() < 1e-3 { 1e-3 } else { beta_raw };
        Some((alpha, beta))
    }

    #[test]
    fn eeg_parse_valid() {
        let (a, b) = parse_eeg(b"E,46102.0,11398.0").unwrap();
        assert!((a - 46102.0).abs() < 0.1);
        assert!((b - 11398.0).abs() < 0.1);
    }

    #[test]
    fn eeg_parse_integers() {
        let (a, b) = parse_eeg(b"E,100,50").unwrap();
        assert!((a - 100.0).abs() < 1e-6);
        assert!((b - 50.0).abs() < 1e-6);
    }

    #[test]
    fn eeg_parse_zero_beta_clamped() {
        let (_, b) = parse_eeg(b"E,1.5,0.0").unwrap();
        assert!((b - 1e-3).abs() < 1e-6, "zero beta must clamp to 1e-3");
    }

    #[test]
    fn eeg_parse_tiny_beta_clamped() {
        let (_, b) = parse_eeg(b"E,2.0,0.0005").unwrap();
        assert!((b - 1e-3).abs() < 1e-6, "tiny beta must clamp to 1e-3");
    }

    #[test]
    fn eeg_parse_negative_beta_clamped() {
        let (_, b) = parse_eeg(b"E,2.0,-0.0001").unwrap();
        assert!((b - 1e-3).abs() < 1e-6, "negative tiny beta must clamp");
    }

    #[test]
    fn eeg_parse_with_whitespace() {
        let (a, b) = parse_eeg(b"  E, 3.0 , 2.0  \r\n").unwrap();
        assert!((a - 3.0).abs() < 1e-6);
        assert!((b - 2.0).abs() < 1e-6);
    }

    #[test]
    fn eeg_parse_wrong_prefix_rejected() {
        assert!(parse_eeg(b"S,1.0,2.0").is_none());
    }

    #[test]
    fn eeg_parse_garbage_rejected() {
        assert!(parse_eeg(b"hello world").is_none());
    }

    #[test]
    fn eeg_parse_missing_beta_rejected() {
        assert!(parse_eeg(b"E,1.5").is_none());
    }

    #[test]
    fn eeg_parse_empty_rejected() {
        assert!(parse_eeg(b"").is_none());
    }

    #[test]
    fn eeg_parse_only_prefix_rejected() {
        assert!(parse_eeg(b"E,").is_none());
    }

    #[test]
    fn eeg_parse_non_numeric_rejected() {
        assert!(parse_eeg(b"E,abc,def").is_none());
    }

    // =====================================================================
    //  3. Speed parser unit tests (inlined pure logic from speed_sensor.rs)
    // =====================================================================

    /// Mirrors SpeedSensor::parse — "S,value" → speed f32.
    fn parse_speed(line: &[u8]) -> Option<f32> {
        let s = core::str::from_utf8(line).ok()?;
        let rest = s.trim().strip_prefix("S,")?;
        rest.trim().parse::<f32>().ok()
    }

    #[test]
    fn speed_parse_valid_float() {
        let v = parse_speed(b"S,65.5").unwrap();
        assert!((v - 65.5).abs() < 1e-6);
    }

    #[test]
    fn speed_parse_integer() {
        let v = parse_speed(b"S,80").unwrap();
        assert!((v - 80.0).abs() < 1e-6);
    }

    #[test]
    fn speed_parse_zero() {
        let v = parse_speed(b"S,0").unwrap();
        assert!(v.abs() < 1e-6);
    }

    #[test]
    fn speed_parse_wrong_prefix_rejected() {
        assert!(parse_speed(b"E,1.0,2.0").is_none());
    }

    #[test]
    fn speed_parse_non_numeric_rejected() {
        assert!(parse_speed(b"S,abc").is_none());
    }

    #[test]
    fn speed_parse_empty_rejected() {
        assert!(parse_speed(b"").is_none());
    }

    // =====================================================================
    //  4. Deadman state machine unit tests (inlined pure transition logic)
    //
    //     Hardware calls (timer, clock, I2C) are stripped — only the state
    //     transition graph is tested here, matching deadman.rs exactly.
    // =====================================================================

    #[derive(Clone, Copy, PartialEq, Debug)]
    enum DState {
        Green,
        Yellow,
        Orange,
        Red,
    }

    #[derive(Clone, Copy, PartialEq, Debug)]
    enum DPower {
        Low,
        Full,
    }

    /// Test-only mirror of DeadmanSwitch with hardware calls removed.
    struct TestDeadman {
        state: DState,
    }

    impl TestDeadman {
        fn new() -> Self {
            Self {
                state: DState::Orange,
            }
        }
        fn in_state(s: DState) -> Self {
            Self { state: s }
        }
        fn state(&self) -> DState {
            self.state
        }
        fn power(&self) -> DPower {
            match self.state {
                DState::Green | DState::Yellow => DPower::Low,
                DState::Orange | DState::Red => DPower::Full,
            }
        }
        fn clock_mhz(&self) -> u32 {
            match self.power() {
                DPower::Full => 168,
                DPower::Low => 48,
            }
        }
        fn sampling_ms(&self) -> u32 {
            match self.power() {
                DPower::Full => 100,
                DPower::Low => 500,
            }
        }

        /// Mirrors on_touch_event (pure logic, no timer/clock calls).
        fn on_touch(&mut self, touched: bool) -> DState {
            match (self.state, touched) {
                (DState::Red, _) => {}
                (DState::Green, true) => {}
                (DState::Yellow, true) => self.state = DState::Green,
                (DState::Orange, true) => self.state = DState::Green,
                (DState::Green, false) => self.state = DState::Yellow,
                (DState::Yellow, false) => {}
                (DState::Orange, false) => {}
            }
            self.state
        }

        fn on_grace_timeout(&mut self) -> DState {
            if self.state == DState::Yellow {
                self.state = DState::Orange;
            }
            self.state
        }

        fn on_user_button(&mut self) -> DState {
            match self.state {
                DState::Red => self.state = DState::Orange,
                _ => self.state = DState::Red,
            }
            self.state
        }
    }

    // ---- boot state ----

    #[test]
    fn deadman_boots_in_orange() {
        let d = TestDeadman::new();
        assert_eq!(d.state(), DState::Orange);
        assert_eq!(d.power(), DPower::Full);
        assert_eq!(d.clock_mhz(), 168);
        assert_eq!(d.sampling_ms(), 100);
    }

    // ---- touch transitions ----

    #[test]
    fn deadman_orange_touch_goes_green() {
        let mut d = TestDeadman::new();
        assert_eq!(d.on_touch(true), DState::Green);
        assert_eq!(d.power(), DPower::Low);
    }

    #[test]
    fn deadman_green_release_goes_yellow() {
        let mut d = TestDeadman::in_state(DState::Green);
        assert_eq!(d.on_touch(false), DState::Yellow);
        assert_eq!(d.power(), DPower::Low); // still low during grace
    }

    #[test]
    fn deadman_yellow_touch_goes_green() {
        let mut d = TestDeadman::in_state(DState::Yellow);
        assert_eq!(d.on_touch(true), DState::Green);
    }

    #[test]
    fn deadman_green_stays_on_touch() {
        let mut d = TestDeadman::in_state(DState::Green);
        assert_eq!(d.on_touch(true), DState::Green);
    }

    #[test]
    fn deadman_yellow_stays_on_release() {
        let mut d = TestDeadman::in_state(DState::Yellow);
        assert_eq!(d.on_touch(false), DState::Yellow);
    }

    #[test]
    fn deadman_orange_stays_on_release() {
        let mut d = TestDeadman::in_state(DState::Orange);
        assert_eq!(d.on_touch(false), DState::Orange);
    }

    #[test]
    fn deadman_red_ignores_touch() {
        let mut d = TestDeadman::in_state(DState::Red);
        assert_eq!(d.on_touch(true), DState::Red);
        assert_eq!(d.on_touch(false), DState::Red);
    }

    // ---- grace timeout ----

    #[test]
    fn deadman_yellow_timeout_goes_orange() {
        let mut d = TestDeadman::in_state(DState::Yellow);
        assert_eq!(d.on_grace_timeout(), DState::Orange);
        assert_eq!(d.power(), DPower::Full);
    }

    #[test]
    fn deadman_timeout_ignored_in_other_states() {
        for &s in &[DState::Green, DState::Orange, DState::Red] {
            let mut d = TestDeadman::in_state(s);
            assert_eq!(d.on_grace_timeout(), s);
        }
    }

    // ---- USER button ----

    #[test]
    fn deadman_user_button_enters_red_from_any_non_red() {
        for &s in &[DState::Green, DState::Yellow, DState::Orange] {
            let mut d = TestDeadman::in_state(s);
            assert_eq!(d.on_user_button(), DState::Red);
            assert_eq!(d.power(), DPower::Full);
        }
    }

    #[test]
    fn deadman_user_button_exits_red_to_orange() {
        let mut d = TestDeadman::in_state(DState::Red);
        assert_eq!(d.on_user_button(), DState::Orange);
    }

    #[test]
    fn deadman_user_button_toggle() {
        let mut d = TestDeadman::new(); // Orange
        assert_eq!(d.on_user_button(), DState::Red);
        assert_eq!(d.on_user_button(), DState::Orange);
        assert_eq!(d.on_user_button(), DState::Red);
    }

    // ---- full lifecycle scenario ----

    #[test]
    fn deadman_full_lifecycle() {
        let mut d = TestDeadman::new();
        assert_eq!(d.state(), DState::Orange);

        // Driver touches screen
        d.on_touch(true);
        assert_eq!(d.state(), DState::Green);
        assert_eq!(d.clock_mhz(), 48);
        assert_eq!(d.sampling_ms(), 500);

        // Driver lifts finger
        d.on_touch(false);
        assert_eq!(d.state(), DState::Yellow);

        // Driver touches again within grace
        d.on_touch(true);
        assert_eq!(d.state(), DState::Green);

        // Driver lifts again
        d.on_touch(false);
        assert_eq!(d.state(), DState::Yellow);

        // Grace expires
        d.on_grace_timeout();
        assert_eq!(d.state(), DState::Orange);
        assert_eq!(d.clock_mhz(), 168);
        assert_eq!(d.sampling_ms(), 100);

        // Suspected fault — operator hits USER button
        d.on_user_button();
        assert_eq!(d.state(), DState::Red);

        // Touch ignored in Red
        d.on_touch(true);
        assert_eq!(d.state(), DState::Red);

        // Operator clears override
        d.on_user_button();
        assert_eq!(d.state(), DState::Orange);
    }

    // =====================================================================
    //  5. Dataset integration tests
    //
    //     Real EEG dataset rows fed through the full pipeline:
    //       raw CSV → compute alpha/beta → format "E,a,b" → parse →
    //       compute ratio → threshold → AlertState
    //
    //     Dataset columns (NeuroSky MindWave format):
    //       attention, meditation, delta, theta,
    //       low_alpha, high_alpha, low_beta, high_beta,
    //       low_gamma, mid_gamma, label
    // =====================================================================

    /// One row from the dataset.
    #[allow(dead_code)]
    struct DatasetRow {
        attention: u32,
        meditation: u32,
        delta: u32,
        theta: u32,
        low_alpha: u32,
        high_alpha: u32,
        low_beta: u32,
        high_beta: u32,
        low_gamma: u32,
        mid_gamma: u32,
        label: u32, // 0 = alert, 1 = drowsy (ground truth)
    }

    impl DatasetRow {
        fn alpha(&self) -> f32 {
            (self.low_alpha + self.high_alpha) as f32
        }
        fn beta(&self) -> f32 {
            let b = (self.low_beta + self.high_beta) as f32;
            if b.abs() < 1e-3 { 1e-3 } else { b }
        }
        fn ratio(&self) -> f32 {
            self.alpha() / self.beta()
        }
        fn is_drowsy(&self) -> bool {
            self.ratio() > DROWSY_RATIO_THRESHOLD
        }
        /// Format as the wire protocol string "E,alpha,beta".
        fn to_eeg_packet(&self) -> String {
            format!("E,{},{}", self.alpha(), self.beta())
        }
    }

    fn dataset() -> Vec<DatasetRow> {
        vec![
            DatasetRow { attention:90, meditation:77, delta:10960,  theta:17978, low_alpha:2045,  high_alpha:44057, low_beta:2045,  high_beta:9353,  low_gamma:5007, mid_gamma:1822, label:0 },
            DatasetRow { attention:83, meditation:81, delta:51251,  theta:11540, low_alpha:13036, high_alpha:13609, low_beta:13036, high_beta:6618,  low_gamma:1717, mid_gamma:2679, label:0 },
            DatasetRow { attention:70, meditation:88, delta:144166, theta:26580, low_alpha:16550, high_alpha:32475, low_beta:16550, high_beta:4483,  low_gamma:3885, mid_gamma:4448, label:0 },
            DatasetRow { attention:75, meditation:87, delta:149499, theta:54240, low_alpha:7309,  high_alpha:42355, low_beta:7309,  high_beta:14471, low_gamma:3419, mid_gamma:2269, label:1 },
            DatasetRow { attention:74, meditation:78, delta:102933, theta:30027, low_alpha:10474, high_alpha:24024, low_beta:10474, high_beta:8442,  low_gamma:3262, mid_gamma:2224, label:1 },
            DatasetRow { attention:83, meditation:75, delta:54676,  theta:21765, low_alpha:15556, high_alpha:17596, low_beta:15556, high_beta:7769,  low_gamma:4262, mid_gamma:2231, label:1 },
            DatasetRow { attention:84, meditation:75, delta:48239,  theta:15895, low_alpha:14153, high_alpha:6243,  low_beta:14153, high_beta:5906,  low_gamma:1887, mid_gamma:2739, label:1 },
            DatasetRow { attention:90, meditation:70, delta:149987, theta:21000, low_alpha:13314, high_alpha:10178, low_beta:13314, high_beta:10640, low_gamma:2221, mid_gamma:1331, label:1 },
            DatasetRow { attention:100,meditation:67, delta:6978,   theta:8977,  low_alpha:11507, high_alpha:11823, low_beta:11507, high_beta:21096, low_gamma:2206, mid_gamma:3254, label:1 },
            DatasetRow { attention:97, meditation:61, delta:118567, theta:48796, low_alpha:16690, high_alpha:11229, low_beta:16690, high_beta:8469,  low_gamma:2398, mid_gamma:2137, label:1 },
            DatasetRow { attention:83, meditation:63, delta:121981, theta:59343, low_alpha:14131, high_alpha:50025, low_beta:14131, high_beta:6010,  low_gamma:3678, mid_gamma:3166, label:1 },
            DatasetRow { attention:70, meditation:66, delta:49173,  theta:29077, low_alpha:16803, high_alpha:23120, low_beta:16803, high_beta:6201,  low_gamma:3581, mid_gamma:4100, label:1 },
            DatasetRow { attention:48, meditation:70, delta:234389, theta:41766, low_alpha:28885, high_alpha:30747, low_beta:28885, high_beta:5712,  low_gamma:3497, mid_gamma:1912, label:1 },
            DatasetRow { attention:47, meditation:80, delta:93725,  theta:16741, low_alpha:4705,  high_alpha:25400, low_beta:4705,  high_beta:4152,  low_gamma:1804, mid_gamma:4743, label:1 },
            DatasetRow { attention:38, meditation:77, delta:153743, theta:91845, low_alpha:10871, high_alpha:47713, low_beta:10871, high_beta:3293,  low_gamma:5660, mid_gamma:6384, label:1 },
        ]
    }

    #[test]
    fn dataset_ratio_computation() {
        // Verify alpha/beta ratio for each row against hand-calculated values.
        let rows = dataset();
        let expected_ratios: &[f32] = &[
            4.044,  // row 0: (2045+44057)/(2045+9353)
            1.356,  // row 1: (13036+13609)/(13036+6618)
            2.331,  // row 2: (16550+32475)/(16550+4483)
            2.281,  // row 3: (7309+42355)/(7309+14471)
            1.824,  // row 4: (10474+24024)/(10474+8442)
            1.421,  // row 5: (15556+17596)/(15556+7769)
            1.017,  // row 6: (14153+6243)/(14153+5906)
            0.981,  // row 7: (13314+10178)/(13314+10640)
            0.716,  // row 8: (11507+11823)/(11507+21096)
            1.110,  // row 9: (16690+11229)/(16690+8469)
            3.185,  // row 10
            1.735,  // row 11
            1.724,  // row 12
            3.399,  // row 13
            4.136,  // row 14
        ];
        for (i, (row, &expected)) in rows.iter().zip(expected_ratios).enumerate() {
            let ratio = row.ratio();
            assert!(
                (ratio - expected).abs() < 0.01,
                "row {}: ratio={:.3} expected={:.3}",
                i,
                ratio,
                expected,
            );
        }
    }

    #[test]
    fn dataset_drowsiness_detection() {
        // Verify which rows our threshold detects as drowsy.
        // Rows 0-5: ratio > 1.2 → drowsy=true
        // Rows 6-9: ratio <= 1.2 → drowsy=false
        // Rows 10-14: ratio > 1.2 → drowsy=true
        let rows = dataset();
        let expected_drowsy = [
            true, true, true, true, true, true, // rows 0-5
            false, false, false, false,          // rows 6-9
            true, true, true, true, true,        // rows 10-14
        ];
        for (i, (row, &expected)) in rows.iter().zip(&expected_drowsy).enumerate() {
            assert_eq!(
                row.is_drowsy(),
                expected,
                "row {}: ratio={:.3} expected drowsy={} got={}",
                i,
                row.ratio(),
                expected,
                row.is_drowsy(),
            );
        }
    }

    #[test]
    fn dataset_eeg_packet_round_trip() {
        // Verify "E,alpha,beta" format round-trips through the parser.
        let rows = dataset();
        for (i, row) in rows.iter().enumerate() {
            let packet = row.to_eeg_packet();
            let (a, b) = parse_eeg(packet.as_bytes()).unwrap_or_else(|| {
                panic!("row {}: failed to parse '{}'", i, packet)
            });
            let original_ratio = row.ratio();
            let parsed_ratio = a / b;
            assert!(
                (parsed_ratio - original_ratio).abs() < 0.01,
                "row {}: parsed ratio={:.3} original={:.3}",
                i,
                parsed_ratio,
                original_ratio,
            );
        }
    }

    #[test]
    fn dataset_alert_escalation_sequence() {
        // Feed all 15 dataset rows through AlertState and verify the
        // alert level at each step.
        //
        // Drowsy sequence: T T T T T T F F F F T T T T T
        //   Frame  1: dcnt=1 → None
        //   Frame  2: dcnt=2 → None
        //   Frame  3: dcnt=3 → None
        //   Frame  4: dcnt=4 → None
        //   Frame  5: dcnt=5 → Alert1  ← escalation
        //   Frame  6: dcnt=6 → Alert1
        //   Frame  7: ncnt=1 → Alert1  (normal streak starts)
        //   Frame  8: ncnt=2 → Alert1
        //   Frame  9: ncnt=3 → Alert1
        //   Frame 10: ncnt=4 → Alert1  (4 normal, NOT enough to de-escalate)
        //   Frame 11: dcnt=1 → Alert1  (normal streak broken at 4)
        //   Frame 12: dcnt=2 → Alert1
        //   Frame 13: dcnt=3 → Alert1
        //   Frame 14: dcnt=4 → Alert1
        //   Frame 15: dcnt=5 → Alert1  (already at Alert1)
        let expected_levels = [
            AlertLevel::None,   // 1
            AlertLevel::None,   // 2
            AlertLevel::None,   // 3
            AlertLevel::None,   // 4
            AlertLevel::Alert1, // 5 ← escalation
            AlertLevel::Alert1, // 6
            AlertLevel::Alert1, // 7
            AlertLevel::Alert1, // 8
            AlertLevel::Alert1, // 9
            AlertLevel::Alert1, // 10 (4 normal, no de-escalation)
            AlertLevel::Alert1, // 11 (drowsy breaks normal streak)
            AlertLevel::Alert1, // 12
            AlertLevel::Alert1, // 13
            AlertLevel::Alert1, // 14
            AlertLevel::Alert1, // 15
        ];

        let rows = dataset();
        let mut alert = AlertState::new();

        println!("\n  Frame | Ratio  | Drowsy | dcnt | ncnt | Alert Level");
        println!("  ------+--------+--------+------+------+----------------");
        for (i, (row, &expected)) in rows.iter().zip(&expected_levels).enumerate() {
            let level = alert.update(row.is_drowsy());
            println!(
                "  {:>5} | {:.3} | {:>6} | {:>4} | {:>4} | {:?}",
                i + 1,
                row.ratio(),
                row.is_drowsy(),
                alert.drowsy_count(),
                alert.normal_count(),
                level,
            );
            assert_eq!(
                level, expected,
                "frame {}: drowsy={} ratio={:.3} dcnt={} ncnt={} level={:?} expected={:?}",
                i + 1,
                row.is_drowsy(),
                row.ratio(),
                alert.drowsy_count(),
                alert.normal_count(),
                level,
                expected,
            );
        }
    }

    #[test]
    fn dataset_ground_truth_vs_ratio_detection() {
        // Compare our ratio-based detection against the dataset labels.
        // Our threshold (ratio > 1.2) doesn't perfectly match the labels,
        // which is expected — the dataset uses a more sophisticated model.
        let rows = dataset();
        let mut matches = 0u32;
        let mut mismatches = 0u32;
        for row in &rows {
            let our_detection = row.is_drowsy();
            let ground_truth = row.label == 1;
            if our_detection == ground_truth {
                matches += 1;
            } else {
                mismatches += 1;
            }
        }
        // We expect some mismatches (rows 0-2 are label=0 but ratio>1.2,
        // rows 6-9 are label=1 but ratio<=1.2).
        assert_eq!(matches, 8, "expected 8 matching predictions");
        assert_eq!(mismatches, 7, "expected 7 mismatches");
        // Detection accuracy on this sample
        let accuracy = matches as f32 / (matches + mismatches) as f32;
        assert!(
            accuracy > 0.50,
            "accuracy {:.1}% too low for basic ratio detector",
            accuracy * 100.0
        );
    }

    #[test]
    fn dataset_extended_scenario_reaches_alert2() {
        // If we replay the dataset twice (simulating 30 frames),
        // we can demonstrate Alert2 escalation. The first 6 frames
        // are drowsy, then frames 7-10 break the streak. But on
        // the second replay, frames 11-15 + 1-6 give 11 consecutive
        // drowsy frames → Alert2.
        let rows = dataset();
        let mut alert = AlertState::new();

        // First pass: 15 frames
        println!("\n  --- Pass 1 (15 frames) ---");
        for (i, row) in rows.iter().enumerate() {
            let level = alert.update(row.is_drowsy());
            println!(
                "  frame {:>2}: ratio={:.3} drowsy={:>5} dcnt={} ncnt={} -> {:?}",
                i + 1, row.ratio(), row.is_drowsy(), alert.drowsy_count(), alert.normal_count(), level
            );
        }
        assert_eq!(alert.level(), AlertLevel::Alert1);
        println!("  End of pass 1: {:?}", alert.level());

        // Second pass: frames 11-14 from the 1st pass were drowsy
        // (dcnt was 5 at end). Now frame 0 of 2nd pass is also drowsy,
        // so streak continues: dcnt=6,7,8,9,10...
        println!("\n  --- Pass 2 (dataset replay, drowsy streak continues) ---");
        for (i, row) in rows.iter().enumerate() {
            let level = alert.update(row.is_drowsy());
            println!(
                "  frame {:>2}: ratio={:.3} drowsy={:>5} dcnt={} ncnt={} -> {:?}",
                i + 1, row.ratio(), row.is_drowsy(), alert.drowsy_count(), alert.normal_count(), level
            );
            // After 5 more consecutive drowsy (frames 0-4 of 2nd pass),
            // dcnt reaches 10 → Alert2
            if i == 4 {
                assert_eq!(
                    level,
                    AlertLevel::Alert2,
                    "expected Alert2 at frame {} of 2nd pass, dcnt={}",
                    i,
                    alert.drowsy_count(),
                );
                println!("  >>> ALERT2 reached at frame {} of pass 2!", i + 1);
                break;
            }
        }
    }
}
