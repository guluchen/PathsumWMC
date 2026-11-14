//! Elements of ℚ(i, √2), written
//! `(re0 + re1 √2) + i (im0 + im1 √2)`.
//!
//! Products of `exp(i k π / 4)` stay inside this ring, and so do sums of
//! those products. That covers Clifford+T path weights exactly.

use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, Zero};

use crate::ball::ComplexBall;

#[derive(Clone, Debug)]
pub struct ExactAmp {
    pub re0: BigRational,
    pub re1: BigRational,
    pub im0: BigRational,
    pub im1: BigRational,
}

impl PartialEq for ExactAmp {
    fn eq(&self, other: &Self) -> bool {
        self.re0 == other.re0
            && self.re1 == other.re1
            && self.im0 == other.im0
            && self.im1 == other.im1
    }
}

impl ExactAmp {
    pub fn zero() -> Self {
        Self {
            re0: BigRational::zero(),
            re1: BigRational::zero(),
            im0: BigRational::zero(),
            im1: BigRational::zero(),
        }
    }

    pub fn one() -> Self {
        Self {
            re0: BigRational::one(),
            re1: BigRational::zero(),
            im0: BigRational::zero(),
            im1: BigRational::zero(),
        }
    }

    pub fn sqrt2() -> Self {
        Self {
            re0: BigRational::zero(),
            re1: BigRational::one(),
            im0: BigRational::zero(),
            im1: BigRational::zero(),
        }
    }

    pub fn is_zero(&self) -> bool {
        self.re0.is_zero() && self.re1.is_zero() && self.im0.is_zero() && self.im1.is_zero()
    }

    pub fn is_one(&self) -> bool {
        self.re0.is_one() && self.re1.is_zero() && self.im0.is_zero() && self.im1.is_zero()
    }

    /// `exp(i k π / 4)`, with `k` reduced modulo 8.
    pub fn exp_i_pi_quarters(k: &BigInt) -> Self {
        let m = k.mod_floor(&BigInt::from(8));
        let half = BigRational::new(BigInt::one(), BigInt::from(2));
        let neg_half = -&half;
        match m.to_string().as_str() {
            "0" => Self::one(),
            "1" => Self {
                re0: BigRational::zero(),
                re1: half.clone(),
                im0: BigRational::zero(),
                im1: half,
            },
            "2" => Self {
                re0: BigRational::zero(),
                re1: BigRational::zero(),
                im0: BigRational::one(),
                im1: BigRational::zero(),
            },
            "3" => Self {
                re0: BigRational::zero(),
                re1: neg_half.clone(),
                im0: BigRational::zero(),
                im1: half,
            },
            "4" => Self {
                re0: -BigRational::one(),
                re1: BigRational::zero(),
                im0: BigRational::zero(),
                im1: BigRational::zero(),
            },
            "5" => Self {
                re0: BigRational::zero(),
                re1: neg_half.clone(),
                im0: BigRational::zero(),
                im1: neg_half,
            },
            "6" => Self {
                re0: BigRational::zero(),
                re1: BigRational::zero(),
                im0: -BigRational::one(),
                im1: BigRational::zero(),
            },
            "7" => Self {
                re0: BigRational::zero(),
                re1: half,
                im0: BigRational::zero(),
                im1: neg_half,
            },
            _ => unreachable!("mod 8"),
        }
    }

    pub fn add(&self, rhs: &Self) -> Self {
        Self {
            re0: &self.re0 + &rhs.re0,
            re1: &self.re1 + &rhs.re1,
            im0: &self.im0 + &rhs.im0,
            im1: &self.im1 + &rhs.im1,
        }
    }

    pub fn neg(&self) -> Self {
        Self {
            re0: -&self.re0,
            re1: -&self.re1,
            im0: -&self.im0,
            im1: -&self.im1,
        }
    }

    pub fn mul(&self, rhs: &Self) -> Self {
        let (rr0, rr1) = mul_s(&self.re0, &self.re1, &rhs.re0, &rhs.re1);
        let (ii0, ii1) = mul_s(&self.im0, &self.im1, &rhs.im0, &rhs.im1);
        let (ri0, ri1) = mul_s(&self.re0, &self.re1, &rhs.im0, &rhs.im1);
        let (ir0, ir1) = mul_s(&self.im0, &self.im1, &rhs.re0, &rhs.re1);
        Self {
            re0: rr0 - ii0,
            re1: rr1 - ii1,
            im0: ri0 + ir0,
            im1: ri1 + ir1,
        }
    }

    pub fn div_pow2(&self, k: u32) -> Self {
        let d = BigRational::from_integer(BigInt::one() << k);
        Self {
            re0: &self.re0 / &d,
            re1: &self.re1 / &d,
            im0: &self.im0 / &d,
            im1: &self.im1 / &d,
        }
    }

    /// Divide by `√2`. `(a0 + a1 √2) / √2 = a1 + a0 √2 / 2`.
    pub fn div_sqrt2(&self) -> Self {
        let two = BigRational::from_integer(BigInt::from(2));
        Self {
            re0: self.re1.clone(),
            re1: &self.re0 / &two,
            im0: self.im1.clone(),
            im1: &self.im0 / &two,
        }
    }

    pub fn div_sqrt2_pow(&self, m: u32) -> Self {
        let mut z = self.clone();
        for _ in 0..m {
            z = z.div_sqrt2();
        }
        z
    }

    /// Squared modulus as `c0 + c1 √2`.
    pub fn norm_sqr(&self) -> (BigRational, BigRational) {
        let (r0, r1) = mul_s(&self.re0, &self.re1, &self.re0, &self.re1);
        let (i0, i1) = mul_s(&self.im0, &self.im1, &self.im0, &self.im1);
        (r0 + i0, r1 + i1)
    }

    pub fn to_ball(&self, prec: u32) -> ComplexBall {
        let s = crate::ball::RealBall::sqrt2(prec);
        let re = crate::ball::RealBall::from_rational(&self.re0, prec)
            .add(&crate::ball::RealBall::from_rational(&self.re1, prec).mul(&s));
        let im = crate::ball::RealBall::from_rational(&self.im0, prec)
            .add(&crate::ball::RealBall::from_rational(&self.im1, prec).mul(&s));
        ComplexBall::new(re, im)
    }

    pub fn pretty(&self) -> String {
        format!(
            "({} + {}√2) + i({} + {}√2)",
            fmt_rat(&self.re0),
            fmt_rat(&self.re1),
            fmt_rat(&self.im0),
            fmt_rat(&self.im1)
        )
    }
}

fn mul_s(
    a0: &BigRational,
    a1: &BigRational,
    b0: &BigRational,
    b1: &BigRational,
) -> (BigRational, BigRational) {
    let two = BigRational::from_integer(BigInt::from(2));
    let p0 = a0 * b0 + &two * a1 * b1;
    let p1 = a0 * b1 + a1 * b0;
    (p0, p1)
}

pub fn fmt_rat(r: &BigRational) -> String {
    if r.denom().is_one() {
        r.numer().to_string()
    } else {
        format!("{}/{}", r.numer(), r.denom())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eighth_root_has_unit_modulus() {
        for k in 0..8 {
            let z = ExactAmp::exp_i_pi_quarters(&BigInt::from(k));
            let (n0, n1) = z.norm_sqr();
            assert_eq!(n0, BigRational::one(), "k={k}");
            assert!(n1.is_zero(), "k={k}");
        }
    }

    #[test]
    fn hadamard_scale() {
        let h = ExactAmp::one().div_sqrt2();
        let (n0, n1) = h.norm_sqr();
        assert_eq!(n0, BigRational::new(BigInt::one(), BigInt::from(2)));
        assert!(n1.is_zero());
    }
}
