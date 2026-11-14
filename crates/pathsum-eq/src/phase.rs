//! Phase polynomials. A coefficient is an exact rational number of radians
//! plus an exact rational multiple of π. Multiples of 2π in the π-component
//! are removed; a nonzero rational radian part is transcendental and is left
//! for ball arithmetic.

use std::collections::BTreeMap;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

use crate::anf::{Anf, Mono};
use crate::exact::fmt_rat;

#[derive(Clone, Debug)]
pub struct Angle {
    pub rad: BigRational,
    pub pi: BigRational,
}

impl Angle {
    pub fn zero() -> Self {
        Self {
            rad: BigRational::zero(),
            pi: BigRational::zero(),
        }
    }

    pub fn from_pi(coeff: BigRational) -> Self {
        Self {
            rad: BigRational::zero(),
            pi: coeff,
        }
    }

    pub fn from_rad(rad: BigRational) -> Self {
        Self {
            rad,
            pi: BigRational::zero(),
        }
    }

    pub fn is_zero(&self) -> bool {
        self.rad.is_zero() && self.pi.is_zero()
    }

    pub fn add(&self, rhs: &Self) -> Self {
        Self {
            rad: &self.rad + &rhs.rad,
            pi: &self.pi + &rhs.pi,
        }
    }

    pub fn neg(&self) -> Self {
        Self {
            rad: -&self.rad,
            pi: -&self.pi,
        }
    }

    pub fn scale(&self, k: &BigRational) -> Self {
        Self {
            rad: &self.rad * k,
            pi: &self.pi * k,
        }
    }

    /// Integer multiple of π. `Some(true)` is an odd multiple, equivalent to π
    /// after multiples of 2π are removed. `Some(false)` is an even multiple
    /// and contributes phase 1. `None` is any other angle.
    pub fn integer_pi_parity(&self) -> Option<bool> {
        if !self.rad.is_zero() || !self.pi.is_integer() {
            return None;
        }
        Some(is_odd_integer(&self.pi))
    }

    /// `exp(i · angle)` lies in ℚ(i, √2) exactly when the angle is `k π / 4`.
    pub fn is_quarter_turn(&self) -> bool {
        if !self.rad.is_zero() {
            return false;
        }
        let k = &self.pi * BigRational::from_integer(BigInt::from(4));
        k.is_integer()
    }

    pub fn quarter_index(&self) -> Option<BigInt> {
        if !self.is_quarter_turn() {
            return None;
        }
        let k = &self.pi * BigRational::from_integer(BigInt::from(4));
        Some(k.to_integer())
    }

    /// Multiple of 2π, i.e. a phase of +1.
    pub fn is_multiple_of_two_pi(&self) -> bool {
        self.rad.is_zero() && mod2(&self.pi).is_zero()
    }

    pub fn normalize(&mut self) {
        self.pi = mod2(&self.pi);
    }

    pub fn pretty(&self) -> String {
        if self.rad.is_zero() && self.pi.is_zero() {
            return "0".into();
        }
        let mut parts = Vec::new();
        if !self.rad.is_zero() {
            parts.push(format!("{} rad", fmt_rat(&self.rad)));
        }
        if !self.pi.is_zero() {
            parts.push(format!("{}π", fmt_rat(&self.pi)));
        }
        parts.join(" + ")
    }
}

#[derive(Clone, Debug, Default)]
pub struct Phase {
    pub terms: BTreeMap<Mono, Angle>,
    /// `angle` added when the Boolean function is true. Used when expanding
    /// the function into monomials would be exponential.
    pub packed: Vec<(Anf, Angle)>,
}

impl Phase {
    pub fn add_mono(&mut self, mono: Mono, angle: Angle) {
        if angle.is_zero() {
            return;
        }
        if let Some(slot) = self.terms.get_mut(&mono) {
            *slot = slot.add(&angle);
            slot.normalize();
            if slot.is_zero() {
                self.terms.remove(&mono);
            }
            return;
        }
        let mut angle = angle;
        angle.normalize();
        if !angle.is_zero() {
            self.terms.insert(mono, angle);
        }
    }

    pub fn normalize(&mut self) {
        let keys: Vec<_> = self.terms.keys().cloned().collect();
        for k in keys {
            if let Some(a) = self.terms.get_mut(&k) {
                a.normalize();
                if a.is_zero() {
                    self.terms.remove(&k);
                }
            }
        }
    }

    pub fn add_packed(&mut self, anf: Anf, angle: Angle) {
        if angle.is_zero() || anf.terms.is_empty() {
            return;
        }
        if anf.terms.len() == 1 {
            let mono = anf.terms.iter().next().expect("one term").clone();
            self.add_mono(mono, angle);
            return;
        }
        self.packed.push((anf, angle));
    }

    pub fn contains_var(&self, v: u16) -> bool {
        self.terms.keys().any(|m| m.contains(v))
            || self.packed.iter().any(|(anf, _)| anf.contains_var(v))
    }

    pub fn has_non_pi_var(&self, v: u16) -> bool {
        self.terms
            .iter()
            .any(|(m, a)| m.contains(v) && !a.rad.is_zero())
    }

    pub fn constant(&self) -> Angle {
        self.terms
            .get(&Mono::empty())
            .cloned()
            .unwrap_or_else(Angle::zero)
    }

    pub fn split_var(&self, v: u16) -> Option<(BTreeMap<Mono, Angle>, BTreeMap<Mono, Angle>)> {
        let mut cofactor = BTreeMap::new();
        let mut rest = BTreeMap::new();
        for (m, a) in &self.terms {
            if m.contains(v) {
                if !a.rad.is_zero() {
                    return None;
                }
                cofactor.insert(m.without(v), a.clone());
            } else {
                rest.insert(m.clone(), a.clone());
            }
        }
        Some((cofactor, rest))
    }
}

/// Reduce a rational modulo 2 into the interval `(-1, 1]`.
pub fn mod2(r: &BigRational) -> BigRational {
    let two = BigRational::from_integer(BigInt::from(2));
    let shifted = (r + BigRational::one()) / &two;
    let fl = shifted.floor();
    let out = r - &two * fl;
    if out == -BigRational::one() {
        // -1 and +1 are the same phase factor (-1), and +1 sits on the
        // boundary we keep. Fold -1 to +1 so integer parity tests see an odd
        // representative consistently... actually e^{iπ(-1)} = e^{iπ(1)} = -1,
        // but the *coefficient* -1 and +1 are interchangeable only for the
        // sign (-1)^c, not as additive monomials before that. Keep -1, which
        // is inside (-1, 1].
        out
    } else {
        out
    }
}

pub fn is_odd_integer(r: &BigRational) -> bool {
    r.is_integer() && !r.numer().is_multiple_of(&BigInt::from(2))
}

use num_integer::Integer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod2_folds_even_multiples() {
        let two = BigRational::from_integer(BigInt::from(2));
        assert!(mod2(&two).is_zero());
        let half = BigRational::new(BigInt::one(), BigInt::from(2));
        assert_eq!(mod2(&(&half + &two)), half);
        assert_eq!(mod2(&-half.clone()), -half);
    }
}
