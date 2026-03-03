use crate::serpi::{SerPi, canonical_cbor_bytes};
use serde::{Serialize, Deserialize};

/// Tension: rational representation of ambiguity level.
/// Theta = theta_numerator / theta_denominator = log2(remaining_survivors).
/// No floats -- fully deterministic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tension {
    pub theta_numerator: u64,
    pub theta_denominator: u64,
    pub remaining_survivors: u64,
}

impl Tension {
    /// Compute tension from a survivor count.
    /// Theta = floor(log2(survivors)) if survivors > 0, else 0.
    /// Denominator = 1 for integer representation.
    pub fn from_survivors(survivors: u64) -> Self {
        if survivors == 0 {
            return Tension {
                theta_numerator: 0,
                theta_denominator: 1,
                remaining_survivors: 0,
            };
        }
        // Integer log2: number of bits minus 1.
        let theta = 63 - survivors.leading_zeros() as u64;
        Tension {
            theta_numerator: theta,
            theta_denominator: 1,
            remaining_survivors: survivors,
        }
    }

    /// Is tension zero? (resolved: 0 or 1 survivors)
    pub fn is_resolved(&self) -> bool {
        self.remaining_survivors <= 1
    }
}

impl SerPi for Tension {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.theta_numerator.ser_pi());
        buf.extend_from_slice(&self.theta_denominator.ser_pi());
        buf.extend_from_slice(&self.remaining_survivors.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Delta in tension from one step to the next.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensionDelta {
    pub delta_theta_num: i64,
    pub delta_theta_den: u64,
    pub delta_cost: u64,
}

impl SerPi for TensionDelta {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.delta_theta_num.ser_pi());
        buf.extend_from_slice(&self.delta_theta_den.ser_pi());
        buf.extend_from_slice(&self.delta_cost.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tension_from_survivors() {
        let t = Tension::from_survivors(8);
        assert_eq!(t.theta_numerator, 3); // log2(8) = 3
        assert_eq!(t.theta_denominator, 1);
        assert_eq!(t.remaining_survivors, 8);
    }

    #[test]
    fn tension_zero_survivors() {
        let t = Tension::from_survivors(0);
        assert!(t.is_resolved());
        assert_eq!(t.theta_numerator, 0);
    }

    #[test]
    fn tension_one_survivor() {
        let t = Tension::from_survivors(1);
        assert!(t.is_resolved());
        assert_eq!(t.theta_numerator, 0);
    }

    #[test]
    fn tension_deterministic() {
        let t = Tension::from_survivors(42);
        assert_eq!(t.ser_pi(), t.ser_pi());
    }

    #[test]
    fn tension_delta_deterministic() {
        let d = TensionDelta {
            delta_theta_num: -3,
            delta_theta_den: 1,
            delta_cost: 5,
        };
        assert_eq!(d.ser_pi(), d.ser_pi());
    }
}
