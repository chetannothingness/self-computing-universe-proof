// Phase 5: Calibration Tracker
//
// Tracks predictions with confidence levels and computes calibration error.
// A well-calibrated system's stated confidence matches its actual accuracy:
//   - Predictions binned into 10 deciles (0-99, 100-199, ..., 900-999 milli)
//   - Per-bin error = |midpoint_confidence - actual_pass_rate|
//   - Mean absolute calibration error = sum(per-bin errors) / (num_active_bins * 1000)
//
// All arithmetic is integer (i64/u64), zero floats.

use serde::{Serialize, Deserialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Tracks a sequence of predictions with stated confidence and actual outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationTracker {
    pub predictions: Vec<CalibrationEntry>,
}

/// A single prediction: confidence in milli-units (0-1000) and actual outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationEntry {
    /// Confidence in milli-units: 0 = 0%, 500 = 50%, 1000 = 100%.
    pub confidence_milli: i64,
    /// Whether the predicted event actually occurred.
    pub actual_pass: bool,
}

impl CalibrationTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        CalibrationTracker {
            predictions: Vec::new(),
        }
    }

    /// Add a prediction.
    pub fn record(&mut self, confidence_milli: i64, actual_pass: bool) {
        self.predictions.push(CalibrationEntry {
            confidence_milli,
            actual_pass,
        });
    }

    /// Number of predictions recorded.
    pub fn len(&self) -> usize {
        self.predictions.len()
    }

    /// Whether the tracker has no predictions.
    pub fn is_empty(&self) -> bool {
        self.predictions.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Calibration error computation
// ---------------------------------------------------------------------------

/// Compute mean absolute calibration error.
///
/// Algorithm:
///   1. Bin predictions into 10 deciles by confidence_milli:
///      bin 0 = [0, 100), bin 1 = [100, 200), ..., bin 9 = [900, 1000]
///   2. For each non-empty bin:
///      - midpoint = bin_index * 100 + 50  (milli-units)
///      - actual_rate = (pass_count * 1000) / total_count  (milli-units)
///      - bin_error = |midpoint - actual_rate|
///   3. Mean error = sum(bin_errors) / (num_active_bins * 1000)
///
/// Returns (numerator, denominator) where the error = num / den.
/// Perfect calibration => (0, den) for some positive den.
pub fn calibration_error(tracker: &CalibrationTracker) -> (i64, u64) {
    if tracker.predictions.is_empty() {
        return (0, 1);
    }

    // Bin counts: index 0..9
    let mut bin_total = [0i64; 10];
    let mut bin_pass = [0i64; 10];

    for entry in &tracker.predictions {
        // Clamp confidence to [0, 999] for binning; 1000 goes into bin 9.
        let bin = (entry.confidence_milli / 100).min(9).max(0) as usize;
        bin_total[bin] += 1;
        if entry.actual_pass {
            bin_pass[bin] += 1;
        }
    }

    let mut error_sum = 0i64;
    let mut bin_count = 0u64;

    for i in 0..10 {
        if bin_total[i] > 0 {
            // Midpoint of this decile in milli-units
            let expected_milli = (i as i64) * 100 + 50;
            // Actual pass rate in milli-units
            let actual_milli = bin_pass[i] * 1000 / bin_total[i];
            error_sum += (expected_milli - actual_milli).abs();
            bin_count += 1;
        }
    }

    if bin_count == 0 {
        (0, 1)
    } else {
        // error = error_sum / (bin_count * 1000)
        (error_sum, bin_count * 1000)
    }
}

/// Compute per-bin calibration details for diagnostics.
/// Returns a Vec of (bin_index, expected_milli, actual_milli, abs_error)
/// for each non-empty bin.
pub fn calibration_bins(tracker: &CalibrationTracker) -> Vec<(usize, i64, i64, i64)> {
    let mut bin_total = [0i64; 10];
    let mut bin_pass = [0i64; 10];

    for entry in &tracker.predictions {
        let bin = (entry.confidence_milli / 100).min(9).max(0) as usize;
        bin_total[bin] += 1;
        if entry.actual_pass {
            bin_pass[bin] += 1;
        }
    }

    let mut result = Vec::new();
    for i in 0..10 {
        if bin_total[i] > 0 {
            let expected_milli = (i as i64) * 100 + 50;
            let actual_milli = bin_pass[i] * 1000 / bin_total[i];
            let abs_error = (expected_milli - actual_milli).abs();
            result.push((i, expected_milli, actual_milli, abs_error));
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibration_perfect_score() {
        // Perfect calibration: in each decile, the actual pass rate equals the midpoint.
        let mut tracker = CalibrationTracker::new();

        // Bin 0 (confidence 0-99, midpoint 50): 0% pass rate would give error 50.
        // To get actual_milli = 50 we need pass_count * 1000 / total = 50
        // => 1 pass out of 20 total: 1*1000/20 = 50. Perfect.
        for _ in 0..19 {
            tracker.record(50, false);
        }
        tracker.record(50, true);

        // Bin 5 (confidence 500-599, midpoint 550): 55% pass rate
        // 11 pass out of 20: 11*1000/20 = 550. Perfect.
        for _ in 0..9 {
            tracker.record(550, false);
        }
        for _ in 0..11 {
            tracker.record(550, true);
        }

        // Bin 9 (confidence 900-999, midpoint 950): 95% pass rate
        // 19 pass out of 20: 19*1000/20 = 950. Perfect.
        tracker.record(950, false);
        for _ in 0..19 {
            tracker.record(950, true);
        }

        let (num, den) = calibration_error(&tracker);
        // Each bin has 0 error => error_sum = 0
        assert_eq!(num, 0);
        assert!(den > 0);
    }

    #[test]
    fn calibration_overconfident_detected() {
        // Overconfident: claims 900-999 confidence (midpoint=950) but never passes.
        let mut tracker = CalibrationTracker::new();
        for _ in 0..10 {
            tracker.record(950, false);
        }

        let (num, den) = calibration_error(&tracker);
        // Bin 9: expected=950, actual=0, error=950
        // Mean error = 950 / (1 * 1000) = 950/1000
        assert_eq!(num, 950);
        assert_eq!(den, 1000);
    }

    #[test]
    fn calibration_underconfident_detected() {
        // Underconfident: claims 0-99 confidence (midpoint=50) but always passes.
        let mut tracker = CalibrationTracker::new();
        for _ in 0..10 {
            tracker.record(50, true);
        }

        let (num, den) = calibration_error(&tracker);
        // Bin 0: expected=50, actual=1000, error=950
        assert_eq!(num, 950);
        assert_eq!(den, 1000);
    }

    #[test]
    fn calibration_empty_tracker() {
        let tracker = CalibrationTracker::new();
        let (num, den) = calibration_error(&tracker);
        assert_eq!(num, 0);
        assert_eq!(den, 1);
    }

    #[test]
    fn calibration_single_bin() {
        let mut tracker = CalibrationTracker::new();
        // Bin 5 (midpoint=550), 5 pass out of 10 => actual=500
        for _ in 0..5 {
            tracker.record(500, true);
            tracker.record(500, false);
        }
        let (num, den) = calibration_error(&tracker);
        // expected=550, actual=500, error=50
        // mean = 50 / (1 * 1000)
        assert_eq!(num, 50);
        assert_eq!(den, 1000);
    }

    #[test]
    fn calibration_multiple_bins() {
        let mut tracker = CalibrationTracker::new();

        // Bin 2 (confidence 200-299, midpoint=250): 3 pass out of 10 => actual=300
        for _ in 0..7 {
            tracker.record(250, false);
        }
        for _ in 0..3 {
            tracker.record(250, true);
        }

        // Bin 7 (confidence 700-799, midpoint=750): 8 pass out of 10 => actual=800
        for _ in 0..2 {
            tracker.record(750, false);
        }
        for _ in 0..8 {
            tracker.record(750, true);
        }

        let (num, den) = calibration_error(&tracker);
        // Bin 2: |250 - 300| = 50
        // Bin 7: |750 - 800| = 50
        // error_sum = 100, bin_count = 2
        // mean = 100 / (2 * 1000) = 100/2000
        assert_eq!(num, 100);
        assert_eq!(den, 2000);
    }

    #[test]
    fn calibration_confidence_1000_goes_to_bin_9() {
        let mut tracker = CalibrationTracker::new();
        // confidence_milli=1000 should be clamped to bin 9
        tracker.record(1000, true);

        let bins = calibration_bins(&tracker);
        assert_eq!(bins.len(), 1);
        assert_eq!(bins[0].0, 9); // bin index 9
    }

    #[test]
    fn calibration_bins_diagnostic() {
        let mut tracker = CalibrationTracker::new();
        for _ in 0..10 {
            tracker.record(150, true); // bin 1
        }
        for _ in 0..10 {
            tracker.record(850, false); // bin 8
        }

        let bins = calibration_bins(&tracker);
        assert_eq!(bins.len(), 2);

        // Bin 1: expected=150, actual=1000 (all pass), error=850
        let bin1 = bins.iter().find(|b| b.0 == 1).unwrap();
        assert_eq!(bin1.1, 150);
        assert_eq!(bin1.2, 1000);
        assert_eq!(bin1.3, 850);

        // Bin 8: expected=850, actual=0 (none pass), error=850
        let bin8 = bins.iter().find(|b| b.0 == 8).unwrap();
        assert_eq!(bin8.1, 850);
        assert_eq!(bin8.2, 0);
        assert_eq!(bin8.3, 850);
    }

    #[test]
    fn calibration_tracker_record_and_len() {
        let mut tracker = CalibrationTracker::new();
        assert!(tracker.is_empty());
        assert_eq!(tracker.len(), 0);

        tracker.record(500, true);
        tracker.record(500, false);
        assert!(!tracker.is_empty());
        assert_eq!(tracker.len(), 2);
    }

    #[test]
    fn calibration_negative_confidence_clamped() {
        // Negative confidence should be clamped to bin 0
        let mut tracker = CalibrationTracker::new();
        tracker.record(-100, true);
        let bins = calibration_bins(&tracker);
        assert_eq!(bins.len(), 1);
        assert_eq!(bins[0].0, 0); // bin index 0
    }
}
