//! Normalized Frobenius distance, minimized over global phase.
//! For z = tr(U U'†)/d, the distance is sqrt(2 - 2|z|).

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};
use serde::Serialize;

use crate::ball::{ComplexBall, RealBall};

#[derive(Clone, Debug, Serialize)]
pub struct DistanceBounds {
    /// Directed, six-significant-digit decimal bounds for display.
    pub lower: String,
    pub upper: String,
    /// Exact rational endpoints for machine checks and reproducibility.
    pub lower_exact: String,
    pub upper_exact: String,
    pub normalization: String,
    pub global_phase_ignored: bool,
}

impl DistanceBounds {
    pub(crate) fn zero() -> Self {
        Self::from_bounds(BigRational::zero(), BigRational::zero())
    }

    pub(crate) fn from_normalized_trace(z: &ComplexBall, precision: u32) -> Option<Self> {
        let (lo, hi) = z.norm_sqr().sqrt().rational_bounds();
        // Unitarity gives 0 <= |tr(V)|/d <= 1. An empty intersection is
        // inconsistent evidence, not permission to fabricate distance zero.
        let lo = lo.max(BigRational::zero());
        let hi = hi.min(BigRational::one());
        if lo > hi {
            return None;
        }
        let two = BigRational::from_integer(BigInt::from(2));
        let squared_lo = &two - &two * hi;
        let squared_hi = &two - &two * lo;
        let (lower, upper) = RealBall::span_rationals(&squared_lo, &squared_hi, precision)
            .sqrt()
            .rational_bounds();
        Some(Self::from_bounds(lower.max(BigRational::zero()), upper))
    }

    fn from_bounds(lo: BigRational, hi: BigRational) -> Self {
        Self {
            lower: decimal_bound(&lo, false),
            upper: decimal_bound(&hi, true),
            lower_exact: lo.to_string(),
            upper_exact: hi.to_string(),
            normalization: "sqrt(2^n)".into(),
            global_phase_ignored: true,
        }
    }
}

/// Convert a nonnegative rational directly to a directed decimal bound.
/// No binary floating-point conversion or nearest-rounding display is used.
fn decimal_bound(value: &BigRational, upper: bool) -> String {
    assert!(value >= &BigRational::zero());
    if value.is_zero() {
        return "0".into();
    }
    let n = value.numer();
    let d = value.denom();
    let ten = BigInt::from(10);
    let mut exponent = n.to_string().len() as i32 - d.to_string().len() as i32;
    let below = if exponent >= 0 {
        n < &(d * ten.pow(exponent as u32))
    } else {
        &(n * ten.pow((-exponent) as u32)) < d
    };
    if below {
        exponent -= 1;
    }
    let scale = 5 - exponent;
    let (num, den) = if scale >= 0 {
        (n * ten.pow(scale as u32), d.clone())
    } else {
        (n.clone(), d * ten.pow((-scale) as u32))
    };
    let mut q = &num / &den;
    if upper && &num % &den != BigInt::zero() {
        q += 1;
    }
    let digits = q.to_string();
    let output_exponent = digits.len() as i32 - 1 + exponent - 5;
    let tail = digits[1..].trim_end_matches('0');
    let mantissa = if tail.is_empty() {
        digits[..1].to_string()
    } else {
        format!("{}.{}", &digits[..1], tail)
    };
    format!("{mantissa}e{output_exponent}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directed_decimal_bounds_do_not_round_inward() {
        let third = BigRational::new(1.into(), 3.into());
        assert_eq!(decimal_bound(&third, false), "3.33333e-1");
        assert_eq!(decimal_bound(&third, true), "3.33334e-1");
        let carry = BigRational::new(9_999_999.into(), 1_000_000.into());
        assert_eq!(decimal_bound(&carry, false), "9.99999e0");
        assert_eq!(decimal_bound(&carry, true), "1e1");
        let tiny = BigRational::new(BigInt::one(), BigInt::from(10).pow(100));
        assert_eq!(decimal_bound(&tiny, false), "1e-100");
        assert_eq!(decimal_bound(&tiny, true), "1e-100");
    }

    #[test]
    fn normalized_unitary_trace_distances() {
        let one = RealBall::one(128);
        let zero = RealBall::zero(128);
        let identity = DistanceBounds::from_normalized_trace(
            &ComplexBall::new(one.clone(), zero.clone()), 128,
        ).unwrap();
        assert_eq!(identity.lower, "0");
        assert_eq!(identity.upper, "0");
        let phase = DistanceBounds::from_normalized_trace(
            &ComplexBall::new(zero.clone(), one), 128,
        ).unwrap();
        assert_eq!(phase.upper, "0");
        let half = RealBall::from_rational(&BigRational::new(1.into(), 2.into()), 128);
        let distance = DistanceBounds::from_normalized_trace(
            &ComplexBall::new(half, zero.clone()), 128,
        ).unwrap();
        assert_eq!(distance.lower_exact, "1");
        assert_eq!(distance.upper_exact, "1");
        let trace_zero = DistanceBounds::from_normalized_trace(
            &ComplexBall::new(zero.clone(), zero), 128,
        ).unwrap();
        assert_eq!(trace_zero.lower, "1.41421e0");
        assert_eq!(trace_zero.upper, "1.41422e0");
    }

    #[test]
    fn impossible_unitary_trace_has_no_distance_claim() {
        assert!(DistanceBounds::from_normalized_trace(
            &ComplexBall::new(RealBall::from_integer(2, 64), RealBall::zero(64)), 64,
        ).is_none());
    }
}
