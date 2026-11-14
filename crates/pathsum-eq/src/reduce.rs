//! Path-sum reduction rules from Amy's calculus, as used by the paper.
//!
//! `[HH]` cancels a pair of path variables whose phase is `π y0 (y1 + Q)`.
//! `[ω]` sums a path variable whose phase is `π y0 (Q ± 1/2)`.
//! Both rules are applied only when `y0` is absent from the output, so the
//! eliminated variable is an internal path. A path variable that disappears
//! from the phase and the output contributes a factor `√2`.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

use crate::anf::{Anf, Mono};
use crate::error::EqError;
use crate::exact::ExactAmp;
use crate::pathsum::PathSum;
use crate::phase::{is_odd_integer, Angle, Phase};
use crate::zpoly::zpoly_from_anf;

pub fn reduce(ps: &mut PathSum) -> Result<(), EqError> {
    for _ in 0..10_000 {
        ps.phase.normalize();
        let dropped = eliminate_dead_paths(ps);
        if dropped > 0 {
            ps.steps.push(format!(
                "Removed {dropped} path variable{} absent from the phase and the output, multiplying the global factor by √2 for each.",
                if dropped == 1 { "" } else { "s" }
            ));
            continue;
        }
        if apply_omega(ps)? {
            continue;
        }
        if apply_hh(ps)? {
            continue;
        }
        break;
    }
    ps.phase.normalize();
    Ok(())
}

fn eliminate_dead_paths(ps: &mut PathSum) -> usize {
    let dead: Vec<u16> = ps
        .path_vars
        .iter()
        .copied()
        .filter(|y| !ps.phase.contains_var(*y) && ps.outputs.iter().all(|o| !o.contains_var(*y)))
        .collect();
    for y in &dead {
        ps.path_vars.remove(y);
        ps.prefactor = ps.prefactor.mul(&ExactAmp::sqrt2());
    }
    dead.len()
}

fn apply_omega(ps: &mut PathSum) -> Result<bool, EqError> {
    let vars: Vec<u16> = ps.path_vars.iter().copied().collect();
    for &y0 in &vars {
        if ps.outputs.iter().any(|o| o.contains_var(y0)) {
            continue;
        }
        let Some((cofactor, rest)) = ps.phase.split_var(y0) else {
            continue;
        };
        if cofactor.is_empty() {
            continue;
        }
        let half = BigRational::new(BigInt::one(), BigInt::from(2));
        let mut q = Anf::zero();
        let mut const_coeff = BigRational::zero();
        let mut shape_ok = true;
        for (mono, angle) in &cofactor {
            if !angle.rad.is_zero() {
                shape_ok = false;
                break;
            }
            if mono.is_empty() {
                const_coeff = angle.pi.clone();
            } else if angle.pi.is_integer() {
                if is_odd_integer(&angle.pi) {
                    q.xor_mono(mono.clone());
                }
            } else {
                shape_ok = false;
                break;
            }
        }
        if !shape_ok || (const_coeff != half && const_coeff != -&half) {
            continue;
        }
        let positive = const_coeff == half;
        if ps.phase.packed.iter().any(|(anf, _)| anf.contains_var(y0)) {
            continue;
        }
        let mut phase = Phase {
            terms: rest,
            packed: ps.phase.packed.clone(),
        };
        let correction = if positive {
            Angle::from_pi(BigRational::new(BigInt::one(), BigInt::from(4)))
        } else {
            Angle::from_pi(BigRational::new(BigInt::from(-1), BigInt::from(4)))
        };
        phase.add_mono(Mono::empty(), correction);
        let sign = if positive {
            BigRational::new(BigInt::from(-1), BigInt::from(2))
        } else {
            BigRational::new(BigInt::one(), BigInt::from(2))
        };
        if q.terms.len() <= 8 {
            let poly = zpoly_from_anf(&q)?;
            for (mono, coeff) in poly.terms {
                phase.add_mono(mono, Angle::from_pi(&sign * &coeff));
            }
        } else {
            phase.add_packed(q, Angle::from_pi(sign));
        }
        ps.phase = phase;
        ps.path_vars.remove(&y0);
        ps.steps.push(format!(
            "Applied [ω] to path variable y{y0} and removed it."
        ));
        return Ok(true);
    }
    Ok(false)
}

fn apply_hh(ps: &mut PathSum) -> Result<bool, EqError> {
    let vars: Vec<u16> = ps.path_vars.iter().copied().collect();
    for &y0 in &vars {
        if ps.outputs.iter().any(|o| o.contains_var(y0)) {
            continue;
        }
        let Some((cofactor, rest)) = ps.phase.split_var(y0) else {
            continue;
        };
        if cofactor.is_empty() {
            continue;
        }
        if cofactor
            .values()
            .any(|a| !a.rad.is_zero() || !a.pi.is_integer())
        {
            continue;
        }
        let mut anf = Anf::zero();
        for (mono, angle) in &cofactor {
            if is_odd_integer(&angle.pi) {
                anf.xor_mono(mono.clone());
            }
        }
        let Some(y1) = anf.linear_path_pivot(&vars) else {
            continue;
        };
        if y1 == y0 {
            continue;
        }
        let q = anf.without_linear(y1);
        // The rule is π y0 (y1 + Q) with Q free of y0 and y1. A higher monomial
        // that still contains the pivot is a different polynomial.
        if q.contains_var(y1) || q.contains_var(y0) {
            continue;
        }
        if ps.phase.packed.iter().any(|(anf, _)| anf.contains_var(y0)) {
            continue;
        }
        let mut phase = substitute_phase(rest, y1, &q)?;
        for (anf, angle) in std::mem::take(&mut ps.phase.packed) {
            phase.add_packed(anf.substitute(y1, &q), angle);
        }
        for out in &mut ps.outputs {
            *out = out.substitute(y1, &q);
        }
        ps.phase = phase;
        ps.path_vars.remove(&y0);
        ps.path_vars.remove(&y1);
        ps.steps.push(format!(
            "Applied [HH] to path variables y{y0} and y{y1}, substituting y{y1} by its parity constraint."
        ));
        return Ok(true);
    }
    Ok(false)
}

pub(crate) fn substitute_phase(
    terms: std::collections::BTreeMap<Mono, Angle>,
    var: u16,
    repl: &Anf,
) -> Result<Phase, EqError> {
    let mut phase = Phase::default();
    let mut expanded: Option<crate::zpoly::ZPoly> = None;
    for (mono, angle) in terms {
        if !mono.contains(var) {
            phase.add_mono(mono, angle);
            continue;
        }
        if let Some(odd) = angle.integer_pi_parity() {
            if odd {
                let mut piece = repl.clone();
                for v in &mono.without(var).0 {
                    piece = piece.and(&Anf::var(*v));
                }
                let kept = Angle::from_pi(BigRational::one());
                for term in piece.terms {
                    phase.add_mono(term, kept.clone());
                }
            }
            continue;
        }
        if repl.terms.len() > 8 {
            let mut piece = repl.clone();
            for v in &mono.without(var).0 {
                piece = piece.and(&Anf::var(*v));
            }
            phase.add_packed(piece, angle);
            continue;
        }
        if expanded.is_none() {
            expanded = Some(zpoly_from_anf(repl)?);
        }
        let piece = expanded
            .as_ref()
            .expect("expanded just above")
            .mul_mono(&mono.without(var));
        for (m2, coeff) in piece.terms {
            phase.add_mono(m2, angle.scale(&coeff));
        }
    }
    Ok(phase)
}
