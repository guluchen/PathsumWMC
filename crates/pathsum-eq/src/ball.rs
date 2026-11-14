//! Real and complex balls: a midpoint interval with outward rounding.
//!
//! Every elementary operation returns a ball that contains the true image.
//! `π` is enclosed by Machin's formula, and `cos` / `sin` use a Taylor
//! polynomial plus an explicit remainder bound. Equivalence is claimed from
//! these enclosures only when the ball excludes the decision threshold.
//! Overlap remains inconclusive. Distance bounds retain outward endpoints.

use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

#[derive(Clone, Debug)]
pub struct RealBall {
    /// Inclusive endpoints, both scaled by `2^exp`.
    lo: BigInt,
    hi: BigInt,
    exp: i32,
    prec: u32,
}

#[derive(Clone, Debug)]
pub struct ComplexBall {
    pub re: RealBall,
    pub im: RealBall,
}

impl RealBall {
    pub fn prec(&self) -> u32 {
        self.prec
    }

    pub fn zero(prec: u32) -> Self {
        Self {
            lo: BigInt::zero(),
            hi: BigInt::zero(),
            exp: 0,
            prec,
        }
    }

    pub fn one(prec: u32) -> Self {
        Self::from_integer(1, prec)
    }

    pub fn pow2(k: u32, prec: u32) -> Self {
        Self {
            lo: BigInt::one(),
            hi: BigInt::one(),
            exp: k as i32,
            prec,
        }
    }

    pub fn from_integer(n: i64, prec: u32) -> Self {
        Self {
            lo: BigInt::from(n),
            hi: BigInt::from(n),
            exp: 0,
            prec,
        }
    }

    /// Outward enclosure of a rational.
    pub fn from_rational(r: &BigRational, prec: u32) -> Self {
        if r.is_zero() {
            return Self::zero(prec);
        }
        let k = prec.saturating_add(4) as usize;
        let shifted = r.numer() << k;
        let (q, rem) = shifted.div_rem(r.denom());
        let exp = -(k as i32);
        if rem.is_zero() {
            return Self {
                lo: q.clone(),
                hi: q,
                exp,
                prec,
            }
            .tighten();
        }
        let (lo, hi) = if shifted.sign() != Sign::Minus {
            (q.clone(), q + 1)
        } else {
            (q.clone() - 1, q)
        };
        Self { lo, hi, exp, prec }.tighten()
    }

    pub fn span_rationals(lo: &BigRational, hi: &BigRational, prec: u32) -> Self {
        Self::from_rational(lo, prec).union(&Self::from_rational(hi, prec))
    }

    pub fn pi(prec: u32) -> Self {
        let (lo, hi) = pi_bounds(prec.saturating_add(8));
        Self::span_rationals(&lo, &hi, prec)
    }

    pub fn sqrt2(prec: u32) -> Self {
        sqrt_rational(&BigRational::from_integer(BigInt::from(2)), prec)
    }

    pub fn neg(&self) -> Self {
        Self {
            lo: -&self.hi,
            hi: -&self.lo,
            exp: self.exp,
            prec: self.prec,
        }
    }

    pub fn abs(&self) -> Self {
        if self.lo.sign() != Sign::Minus {
            return self.clone();
        }
        if self.hi.sign() == Sign::Minus {
            return self.neg();
        }
        let mag = self.lo.abs().max(self.hi.clone());
        Self {
            lo: BigInt::zero(),
            hi: mag,
            exp: self.exp,
            prec: self.prec,
        }
    }

    /// Smallest symmetric ball containing `self` and its negation.
    fn symmetric(&self) -> Self {
        let mag = self.abs();
        Self {
            lo: -&mag.hi,
            hi: mag.hi,
            exp: mag.exp,
            prec: mag.prec,
        }
    }

    pub fn add(&self, rhs: &Self) -> Self {
        let prec = self.prec.max(rhs.prec);
        let exp = self.exp.min(rhs.exp);
        let lo = shift_endpoint(&self.lo, self.exp, exp, true)
            + shift_endpoint(&rhs.lo, rhs.exp, exp, true);
        let hi = shift_endpoint(&self.hi, self.exp, exp, false)
            + shift_endpoint(&rhs.hi, rhs.exp, exp, false);
        Self { lo, hi, exp, prec }.tighten()
    }

    pub fn sub(&self, rhs: &Self) -> Self {
        self.add(&rhs.neg())
    }

    pub fn mul(&self, rhs: &Self) -> Self {
        let prec = self.prec.max(rhs.prec);
        let exp = self.exp + rhs.exp;
        let corners = [
            &self.lo * &rhs.lo,
            &self.lo * &rhs.hi,
            &self.hi * &rhs.lo,
            &self.hi * &rhs.hi,
        ];
        let lo = corners.iter().min().cloned().unwrap_or_else(BigInt::zero);
        let hi = corners.iter().max().cloned().unwrap_or_else(BigInt::zero);
        Self { lo, hi, exp, prec }.tighten()
    }

    pub fn mul_integer(&self, k: i64) -> Self {
        self.mul(&Self::from_integer(k, self.prec))
    }

    pub fn contains_zero(&self) -> bool {
        let z = BigInt::zero();
        match self.exp.cmp(&0) {
            std::cmp::Ordering::Equal => self.lo <= z && self.hi >= z,
            std::cmp::Ordering::Greater => {
                // endpoints * 2^exp, exp>0, so the integer value has the sign of the endpoint
                // unless the endpoint is 0.
                self.lo <= z && self.hi >= z
            }
            std::cmp::Ordering::Less => self.lo <= z && self.hi >= z,
        }
    }

    pub fn certainly_positive(&self) -> bool {
        // 2^exp is positive, so the sign is the sign of the mantissa.
        self.lo > BigInt::zero()
    }

    pub fn certainly_negative(&self) -> bool {
        self.hi < BigInt::zero()
    }

    pub fn recip(&self) -> Self {
        if self.contains_zero() {
            // A denominator enclosure that touches zero carries no finite bound.
            // Callers in this crate divide only by powers of two and √2.
            return Self {
                lo: BigInt::from(-1),
                hi: BigInt::one(),
                exp: 10_000,
                prec: self.prec,
            };
        }
        let lo_r = endpoint_rational(&self.lo, self.exp);
        let hi_r = endpoint_rational(&self.hi, self.exp);
        let a = recip_rational(&hi_r);
        let b = recip_rational(&lo_r);
        let (u, v) = if a <= b { (a, b) } else { (b, a) };
        Self::span_rationals(&u, &v, self.prec)
    }

    pub fn div(&self, rhs: &Self) -> Self {
        self.mul(&rhs.recip())
    }

    pub fn sqrt(&self) -> Self {
        if self.certainly_negative() {
            return Self::zero(self.prec);
        }
        let lo_r = endpoint_rational(&self.lo, self.exp).max(BigRational::zero());
        let hi_r = endpoint_rational(&self.hi, self.exp).max(BigRational::zero());
        if hi_r.is_zero() {
            return Self::zero(self.prec);
        }
        let slo = if lo_r.is_zero() {
            Self::zero(self.prec)
        } else {
            sqrt_rational(&lo_r, self.prec)
        };
        let shi = sqrt_rational(&hi_r, self.prec);
        slo.union(&shi)
    }

    pub fn union(&self, rhs: &Self) -> Self {
        let prec = self.prec.max(rhs.prec);
        let exp = self.exp.min(rhs.exp);
        let lo = shift_endpoint(&self.lo, self.exp, exp, true)
            .min(shift_endpoint(&rhs.lo, rhs.exp, exp, true));
        let hi = shift_endpoint(&self.hi, self.exp, exp, false)
            .max(shift_endpoint(&rhs.hi, rhs.exp, exp, false));
        Self { lo, hi, exp, prec }.tighten()
    }

    pub fn cos(&self) -> Self {
        trig_ball(self, true)
    }

    pub fn sin(&self) -> Self {
        trig_ball(self, false)
    }

    pub fn certainly_lt(&self, other: &Self) -> bool {
        cmp_endpoints(&self.hi, self.exp, &other.lo, other.exp).is_lt()
    }

    pub fn certainly_gt(&self, other: &Self) -> bool {
        cmp_endpoints(&self.lo, self.exp, &other.hi, other.exp).is_gt()
    }

    pub fn contains_rational(&self, r: &BigRational) -> bool {
        let lo = endpoint_rational(&self.lo, self.exp);
        let hi = endpoint_rational(&self.hi, self.exp);
        lo <= *r && *r <= hi
    }

    pub fn relative_width(&self) -> BigRational {
        let lo = endpoint_rational(&self.lo, self.exp);
        let hi = endpoint_rational(&self.hi, self.exp);
        let width = &hi - &lo;
        let scale = hi.abs().max(lo.abs()).max(BigRational::one());
        width / scale
    }

    pub fn pretty(&self) -> String {
        let mid = self.mid_f64();
        let rad = self.rad_f64();
        if mid.is_finite() && rad.is_finite() {
            format!("{mid:.8} ± {rad:.2e}")
        } else {
            format!("[{}, {}] · 2^{}", self.lo, self.hi, self.exp)
        }
    }

    /// Half the width of the interval, as a rational.
    pub fn radius_rational(&self) -> BigRational {
        let lo = endpoint_rational(&self.lo, self.exp);
        let hi = endpoint_rational(&self.hi, self.exp);
        (&hi - &lo) / BigRational::from_integer(BigInt::from(2))
    }

    /// Exact inclusive endpoints, before any decimal display rounding.
    pub fn rational_bounds(&self) -> (BigRational, BigRational) {
        (
            endpoint_rational(&self.lo, self.exp),
            endpoint_rational(&self.hi, self.exp),
        )
    }

    /// Nearest decimal to the midpoint, with `digits` digits after the point.
    pub fn midpoint_decimal(&self, digits: u32) -> String {
        let lo = endpoint_rational(&self.lo, self.exp);
        let hi = endpoint_rational(&self.hi, self.exp);
        let mid = (lo + hi) / BigRational::from_integer(BigInt::from(2));
        format_fixed(&mid, digits)
    }

    fn full_unit(prec: u32) -> Self {
        Self {
            lo: -BigInt::one(),
            hi: BigInt::one(),
            exp: 0,
            prec,
        }
    }

    fn as_exact_zero(&self) -> Option<()> {
        if self.lo.is_zero() && self.hi.is_zero() {
            Some(())
        } else {
            None
        }
    }

    fn mid_f64(&self) -> f64 {
        let mid = (&self.lo + &self.hi) / 2;
        dyadic_f64(&mid, self.exp)
    }

    fn rad_f64(&self) -> f64 {
        let width = &self.hi - &self.lo;
        dyadic_f64(&width, self.exp) * 0.5
    }

    fn rem_tau(&self) -> Self {
        let (plo, phi) = pi_bounds(self.prec.saturating_add(8));
        let two_lo = &plo * BigRational::from_integer(BigInt::from(2));
        let two_hi = &phi * BigRational::from_integer(BigInt::from(2));
        let xlo = endpoint_rational(&self.lo, self.exp);
        let xhi = endpoint_rational(&self.hi, self.exp);
        let approx = (&two_lo + &two_hi) / BigRational::from_integer(BigInt::from(2));
        let mid = (&xlo + &xhi) / BigRational::from_integer(BigInt::from(2));
        let q = (&mid / &approx).round();
        let (ylo, yhi) = if q.numer().sign() != Sign::Minus && !q.is_zero() {
            (&xlo - &q * &two_hi, &xhi - &q * &two_lo)
        } else if q.is_zero() {
            (xlo, xhi)
        } else {
            // q < 0, multiplying by q reverses the 2π bounds
            (&xlo - &q * &two_lo, &xhi - &q * &two_hi)
        };
        Self::span_rationals(&ylo, &yhi, self.prec)
    }

    fn tighten(mut self) -> Self {
        if self.lo > self.hi {
            std::mem::swap(&mut self.lo, &mut self.hi);
        }
        let bits = self.lo.bits().max(self.hi.bits());
        let budget = self.prec as u64 + 6;
        if bits > budget && bits > 8 {
            let shift = (bits - budget) as u32;
            self.lo = floor_div_pow2(&self.lo, shift);
            self.hi = ceil_div_pow2(&self.hi, shift);
            self.exp += shift as i32;
            if self.lo > self.hi {
                std::mem::swap(&mut self.lo, &mut self.hi);
            }
        }
        self
    }
}

impl ComplexBall {
    pub fn new(re: RealBall, im: RealBall) -> Self {
        let prec = re.prec.max(im.prec);
        Self {
            re: RealBall { prec, ..re },
            im: RealBall { prec, ..im },
        }
    }

    pub fn zero(prec: u32) -> Self {
        Self::new(RealBall::zero(prec), RealBall::zero(prec))
    }

    pub fn one(prec: u32) -> Self {
        Self::new(RealBall::one(prec), RealBall::zero(prec))
    }

    pub fn i(prec: u32) -> Self {
        Self::new(RealBall::zero(prec), RealBall::one(prec))
    }

    pub fn neg(&self) -> Self {
        Self::new(self.re.neg(), self.im.neg())
    }

    pub fn add(&self, rhs: &Self) -> Self {
        Self::new(self.re.add(&rhs.re), self.im.add(&rhs.im))
    }

    pub fn mul(&self, rhs: &Self) -> Self {
        let re = self.re.mul(&rhs.re).sub(&self.im.mul(&rhs.im));
        let im = self.re.mul(&rhs.im).add(&self.im.mul(&rhs.re));
        Self::new(re, im)
    }

    pub fn mul_real(&self, r: &RealBall) -> Self {
        Self::new(self.re.mul(r), self.im.mul(r))
    }

    pub fn div_sqrt2_pow(&self, m: u32) -> Self {
        let prec = self.re.prec;
        let mut z = self.clone();
        let whole = m / 2;
        if whole > 0 {
            let scale = RealBall::from_rational(
                &BigRational::new(BigInt::one(), BigInt::one() << whole),
                prec,
            );
            z = z.mul_real(&scale);
        }
        if m % 2 == 1 {
            z = z.mul_real(&RealBall::sqrt2(prec).recip());
        }
        z
    }

    pub fn div_pow2(&self, k: u32) -> Self {
        if k == 0 {
            return self.clone();
        }
        let scale = RealBall::from_rational(
            &BigRational::new(BigInt::one(), BigInt::one() << k),
            self.re.prec,
        );
        self.mul_real(&scale)
    }

    pub fn norm_sqr(&self) -> RealBall {
        self.re.mul(&self.re).add(&self.im.mul(&self.im))
    }

    pub fn exp_i(theta: &RealBall) -> Self {
        Self::new(theta.cos(), theta.sin())
    }

    pub fn pretty(&self) -> String {
        format!("{} + i({})", self.re.pretty(), self.im.pretty())
    }
}

fn trig_ball(x: &RealBall, cosine: bool) -> RealBall {
    if cosine && x.as_exact_zero().is_some() {
        return RealBall::one(x.prec);
    }
    if !cosine && x.as_exact_zero().is_some() {
        return RealBall::zero(x.prec);
    }
    let reduced = x.rem_tau();
    let lo = endpoint_rational(&reduced.lo, reduced.exp);
    let hi = endpoint_rational(&reduced.hi, reduced.exp);
    let width = &hi - &lo;
    if width > BigRational::from_integer(BigInt::from(2)) {
        return RealBall::full_unit(x.prec);
    }
    let two = BigRational::from_integer(BigInt::from(2));
    let radius = &width / &two;
    // Series at the exact midpoint. Ball multiplication tightens to the
    // working precision, so the denominators stay a few hundred bits instead
    // of growing with every Taylor term.
    let mid = RealBall {
        lo: &reduced.lo + &reduced.hi,
        hi: &reduced.lo + &reduced.hi,
        exp: reduced.exp - 1,
        prec: reduced.prec,
    };
    let center = trig_series(&mid, cosine);
    let slack = RealBall::span_rationals(&-&radius, &radius, x.prec);
    center.add(&slack)
}

/// Taylor polynomial of sin or cos, plus a Lagrange remainder.
///
/// `x` is a point (possibly tightened outward on the first operation). The
/// returned ball contains the true sin or cos of every value in `x`.
fn trig_series(x: &RealBall, cosine: bool) -> RealBall {
    let n_terms = (x.prec / 2 + 8).max(8);
    let mut term = if cosine {
        RealBall::one(x.prec)
    } else {
        x.clone()
    };
    let mut sum = term.clone();
    let x2 = x.mul(x);
    for k in 1..n_terms {
        let denom = if cosine {
            (2 * k as i64 - 1) * (2 * k as i64)
        } else {
            (2 * k as i64) * (2 * k as i64 + 1)
        };
        term = term
            .neg()
            .mul(&x2)
            .div(&RealBall::from_integer(denom, x.prec));
        sum = sum.add(&term);
    }
    let power = if cosine { 2 * n_terms } else { 2 * n_terms + 1 };
    // Lagrange remainder |R| ≤ |x|^power / power!. Its upper end covers every
    // point of `x`, so the polynomial ball plus ±R contains sin or cos there.
    let rem = monomial_over_factorial(&x.abs(), power);
    sum.add(&rem.symmetric())
}

/// Enclosure of `|x|^power / power!`.
fn monomial_over_factorial(abs_x: &RealBall, power: u32) -> RealBall {
    let mut acc = RealBall::one(abs_x.prec);
    for i in 1..=power {
        acc = acc
            .mul(abs_x)
            .div(&RealBall::from_integer(i as i64, abs_x.prec));
    }
    acc
}

fn format_fixed(value: &BigRational, digits: u32) -> String {
    if value.is_zero() {
        return "0".into();
    }
    let negative = value.is_negative();
    let scale = BigInt::from(10).pow(digits);
    let shifted = value.abs() * BigRational::from_integer(scale);
    let (mut rounded, rem) = shifted.numer().div_rem(shifted.denom());
    if (&rem * 2) >= *shifted.denom() {
        rounded += 1;
    }
    let mut digits_s = rounded.to_string();
    let width = digits as usize + 1;
    if digits_s.len() < width {
        digits_s = format!("{digits_s:0>width$}");
    }
    let split = digits_s.len() - digits as usize;
    let (whole, frac) = digits_s.split_at(split);
    let frac = frac.trim_end_matches('0');
    let body = if frac.is_empty() {
        whole.to_string()
    } else {
        format!("{whole}.{frac}")
    };
    if negative {
        format!("-{body}")
    } else {
        body
    }
}

fn endpoint_rational(man: &BigInt, exp: i32) -> BigRational {
    if exp >= 0 {
        BigRational::from_integer(man << exp)
    } else {
        BigRational::new(man.clone(), BigInt::one() << (-exp as usize))
    }
}

fn recip_rational(r: &BigRational) -> BigRational {
    BigRational::new(r.denom().clone(), r.numer().clone())
}

fn cmp_endpoints(a: &BigInt, a_exp: i32, b: &BigInt, b_exp: i32) -> std::cmp::Ordering {
    if a_exp == b_exp {
        return a.cmp(b);
    }
    let exp = a_exp.min(b_exp);
    let aa = shift_endpoint(a, a_exp, exp, false);
    let bb = shift_endpoint(b, b_exp, exp, true);
    // Conservative: if we rounded, this comparison can be equal when a strict
    // inequality holds. Callers use it only for certainly_lt / certainly_gt,
    // so a false Equal is the safe direction. The shifts below are exact when
    // the target exponent is finer.
    let _ = (aa, bb);
    endpoint_rational(a, a_exp).cmp(&endpoint_rational(b, b_exp))
}

fn shift_endpoint(man: &BigInt, exp: i32, target: i32, lower: bool) -> BigInt {
    let shift = exp - target;
    if shift >= 0 {
        if shift > 100_000 {
            let huge = sign_of(man) * (BigInt::one() << 100_000);
            return huge;
        }
        man << (shift as usize)
    } else {
        let s = (-shift) as u32;
        if lower {
            floor_div_pow2(man, s)
        } else {
            ceil_div_pow2(man, s)
        }
    }
}

fn floor_div_pow2(x: &BigInt, shift: u32) -> BigInt {
    if shift == 0 {
        return x.clone();
    }
    let d = BigInt::one() << shift;
    let (q, r) = x.div_rem(&d);
    if r.is_zero() || x.sign() != Sign::Minus {
        q
    } else {
        q - 1
    }
}

fn ceil_div_pow2(x: &BigInt, shift: u32) -> BigInt {
    if shift == 0 {
        return x.clone();
    }
    let d = BigInt::one() << shift;
    let (q, r) = x.div_rem(&d);
    if r.is_zero() || x.sign() == Sign::Minus {
        q
    } else {
        q + 1
    }
}

fn dyadic_f64(man: &BigInt, exp: i32) -> f64 {
    if man.is_zero() {
        return 0.0;
    }
    let bits = man.bits();
    let (mant, e) = if bits <= 53 {
        (man.clone(), exp)
    } else {
        let shift = (bits - 53) as u32;
        let rounded = if man.sign() == Sign::Minus {
            -floor_div_pow2(&man.abs(), shift)
        } else {
            floor_div_pow2(man, shift)
        };
        (rounded, exp.saturating_add(shift as i32))
    };
    let mf = mant.to_f64().unwrap_or(f64::NAN);
    if !(-1200..=1200).contains(&e) {
        return if mf.is_sign_negative() {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    mf * 2f64.powi(e)
}

fn sqrt_rational(r: &BigRational, prec: u32) -> RealBall {
    if r.is_zero() {
        return RealBall::zero(prec);
    }
    let k = prec.saturating_add(4);
    let s = r.numer() * r.denom();
    let shifted = &s << (2 * k);
    let root = isqrt(&shifted);
    let den = r.denom() << k;
    if &root * &root == shifted {
        return RealBall::from_rational(&BigRational::new(root, den), prec);
    }
    let lo = BigRational::new(root.clone(), den.clone());
    let hi = BigRational::new(root + 1, den);
    RealBall::span_rationals(&lo, &hi, prec)
}

fn isqrt(n: &BigInt) -> BigInt {
    if *n <= BigInt::one() {
        return n.clone().max(BigInt::zero());
    }
    let mut x = BigInt::one() << ((n.bits() / 2) + 1);
    loop {
        let y = (&x + (n / &x)) >> 1;
        if y >= x {
            return x;
        }
        x = y;
    }
}

/// Rigorous enclosure of π from Machin: π = 16 atan(1/5) − 4 atan(1/239).
fn pi_bounds(prec: u32) -> (BigRational, BigRational) {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<u32, (BigRational, BigRational)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache.lock().expect("pi cache");
    if let Some(found) = guard.get(&prec) {
        return found.clone();
    }
    let computed = pi_bounds_uncached(prec);
    guard.insert(prec, computed.clone());
    computed
}

fn pi_bounds_uncached(prec: u32) -> (BigRational, BigRational) {
    let a = atan_small(BigInt::one(), BigInt::from(5), prec);
    let b = atan_small(BigInt::one(), BigInt::from(239), prec);
    let sixteen = BigRational::from_integer(BigInt::from(16));
    let four = BigRational::from_integer(BigInt::from(4));
    let lo = &sixteen * &a.0 - &four * &b.1;
    let hi = &sixteen * &a.1 - &four * &b.0;
    (lo, hi)
}

fn atan_small(num: BigInt, den: BigInt, prec: u32) -> (BigRational, BigRational) {
    let x = BigRational::new(num, den);
    let x2 = &x * &x;
    let mut term = x.clone();
    let mut sum = term.clone();
    let tol = BigRational::new(BigInt::one(), BigInt::one() << (prec as usize + 8));
    let mut n: i32 = 0;
    loop {
        term = -term * &x2 * BigRational::from_integer(BigInt::from(2 * n + 1))
            / BigRational::from_integer(BigInt::from(2 * n + 3));
        n += 1;
        if term.abs() < tol {
            break;
        }
        sum += &term;
        if n > 10_000 {
            break;
        }
    }
    if term.is_zero() {
        return (sum.clone(), sum);
    }
    let other = &sum + &term;
    if sum <= other {
        (sum, other)
    } else {
        (other, sum)
    }
}

fn sign_of(man: &BigInt) -> BigInt {
    match man.sign() {
        Sign::Minus => -BigInt::one(),
        Sign::NoSign => BigInt::zero(),
        Sign::Plus => BigInt::one(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pi_contains_known_digits() {
        let p = RealBall::pi(64);
        let below = RealBall::from_rational(
            &BigRational::new(BigInt::from(3141592653i64), BigInt::from(1_000_000_000i64)),
            64,
        );
        let above = RealBall::from_rational(
            &BigRational::new(BigInt::from(3141592654i64), BigInt::from(1_000_000_000i64)),
            64,
        );
        assert!(below.certainly_lt(&p), "{}", p.pretty());
        assert!(p.certainly_lt(&above), "{}", p.pretty());
    }

    #[test]
    fn sqrt2_square_contains_two() {
        let s = RealBall::sqrt2(80);
        let sq = s.mul(&s);
        assert!(sq.contains_rational(&BigRational::from_integer(BigInt::from(2))));
        assert!(sq.relative_width() < BigRational::new(BigInt::one(), BigInt::one() << 40));
    }

    #[test]
    fn cos_zero_and_pi() {
        let c0 = RealBall::zero(48).cos();
        assert!(c0.contains_rational(&BigRational::one()));
        let cpi = RealBall::pi(64).cos();
        assert!(
            cpi.contains_rational(&-BigRational::one()),
            "{:?} pretty {}",
            cpi,
            cpi.pretty()
        );
        assert!(cpi.certainly_negative(), "{}", cpi.pretty());
    }

    #[test]
    fn cos_of_half_radian_stays_tight() {
        let x = RealBall::from_rational(&BigRational::new(BigInt::one(), BigInt::from(2)), 72);
        let c = x.cos();
        // cos(0.5) = 0.8775825618903728…
        let below = RealBall::from_rational(
            &BigRational::new(BigInt::from(8775825618i64), BigInt::from(10_000_000_000i64)),
            72,
        );
        let above = RealBall::from_rational(
            &BigRational::new(BigInt::from(8775825619i64), BigInt::from(10_000_000_000i64)),
            72,
        );
        assert!(below.certainly_lt(&c), "{}", c.pretty());
        assert!(c.certainly_lt(&above), "{}", c.pretty());
        assert!(
            c.relative_width() < BigRational::new(BigInt::one(), BigInt::one() << 40),
            "{}",
            c.pretty()
        );
    }

    #[test]
    fn exp_i_pi_is_minus_one() {
        let z = ComplexBall::exp_i(&RealBall::pi(72));
        assert!(
            z.re.contains_rational(&-BigRational::one()),
            "{}",
            z.pretty()
        );
        assert!(
            z.im.contains_rational(&BigRational::zero()),
            "{}",
            z.pretty()
        );
    }
}
