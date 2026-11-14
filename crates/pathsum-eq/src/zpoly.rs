//! Multilinear integer polynomials, with the relation `x^2 = x`.
//!
//! These are the 0/1-valued expansions of Boolean functions:
//! `a ⊕ b = a + b − 2ab`, `¬a = 1 − a`.

use std::collections::BTreeMap;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

use crate::anf::{Anf, Mono};
use crate::error::EqError;

#[derive(Clone, Debug)]
pub struct ZPoly {
    pub terms: BTreeMap<Mono, BigRational>,
}

impl ZPoly {
    pub fn zero() -> Self {
        Self {
            terms: BTreeMap::new(),
        }
    }

    pub fn singleton(m: Mono) -> Self {
        let mut terms = BTreeMap::new();
        terms.insert(m, BigRational::one());
        Self { terms }
    }

    pub fn add(&self, rhs: &Self) -> Self {
        let mut out = self.clone();
        for (m, c) in &rhs.terms {
            out.add_term(m.clone(), c.clone());
        }
        out
    }

    pub fn sub(&self, rhs: &Self) -> Self {
        self.add(&rhs.scale(&-BigRational::one()))
    }

    pub fn scale(&self, k: &BigRational) -> Self {
        if k.is_zero() {
            return Self::zero();
        }
        let mut terms = BTreeMap::new();
        for (m, c) in &self.terms {
            let p = c * k;
            if !p.is_zero() {
                terms.insert(m.clone(), p);
            }
        }
        Self { terms }
    }

    pub fn mul(&self, rhs: &Self) -> Self {
        let mut out = Self::zero();
        for (m1, c1) in &self.terms {
            for (m2, c2) in &rhs.terms {
                out.add_term(m1.union(m2), c1 * c2);
            }
        }
        out
    }

    pub fn mul_mono(&self, m: &Mono) -> Self {
        let mut out = Self::zero();
        for (m1, c) in &self.terms {
            out.add_term(m1.union(m), c.clone());
        }
        out
    }

    fn add_term(&mut self, m: Mono, c: BigRational) {
        if c.is_zero() {
            return;
        }
        if let Some(slot) = self.terms.get_mut(&m) {
            *slot += c;
            if slot.is_zero() {
                self.terms.remove(&m);
            }
            return;
        }
        self.terms.insert(m, c);
    }
}

pub fn zpoly_from_anf(anf: &Anf) -> Result<ZPoly, EqError> {
    const MAX_TERMS: usize = 8_192;
    let mut acc = ZPoly::zero();
    let mut started = false;
    for term in &anf.terms {
        let t = ZPoly::singleton(term.clone());
        acc = if !started {
            started = true;
            t
        } else {
            let cross = acc
                .mul(&t)
                .scale(&BigRational::from_integer(BigInt::from(2)));
            acc.add(&t).sub(&cross)
        };
        if acc.terms.len() > MAX_TERMS {
            return Err(EqError(
                "the phase polynomial exceeded 8192 monomials while expanding a Boolean function"
                    .into(),
            ));
        }
    }
    Ok(acc)
}
