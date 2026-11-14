//! Weighted-model-counting encoding of a residual path-sum.
//!
//! Each phase monomial `k · x1 x2 … xt` becomes a selector equivalent to that
//! conjunction, with literal weight `exp(i k)` on true and `1` on false.
//! Diagonal entries are the Boolean constraints `output_i ↔ x_i`. The weighted
//! model count is then the un-normalized sum of diagonal amplitudes.

use std::collections::BTreeMap;

use crate::anf::{Anf, Mono};
use crate::error::EqError;
use crate::pathsum::PathSum;
use crate::phase::Angle;

#[derive(Clone, Debug)]
pub struct Encoded {
    pub clauses: Vec<Vec<i32>>,
    pub nvars: u32,
    /// `angles[v]` is the phase contributed when variable `v` is true.
    /// Index 0 is unused. Missing entries contribute weight 1.
    pub angles: Vec<Option<Angle>>,
    #[allow(dead_code)]
    pub path_vars: u32,
    #[allow(dead_code)]
    pub inputs: u32,
}

pub fn encode(ps: &PathSum) -> Result<Encoded, EqError> {
    let mut enc = Encoder {
        clauses: Vec::new(),
        angles: vec![None],
    };
    let mut map: BTreeMap<u16, i32> = BTreeMap::new();
    for q in 0..ps.qubits as u16 {
        map.insert(q, enc.fresh());
    }
    for y in &ps.path_vars {
        map.insert(*y, enc.fresh());
    }
    for (i, out) in ps.outputs.iter().enumerate() {
        encode_output_matches(&mut enc, out, map[&(i as u16)], &map)?;
    }
    for (mono, angle) in &ps.phase.terms {
        if mono.is_empty() || angle.is_zero() {
            continue;
        }
        let mut bits = Vec::with_capacity(mono.0.len());
        for v in &mono.0 {
            let Some(bit) = map.get(v) else {
                return Err(EqError(format!(
                    "phase mentions variable {v}, which is neither an input nor a path variable"
                )));
            };
            bits.push(*bit);
        }
        let selector = encode_conjunction(&mut enc, &bits);
        if enc.angles[selector as usize].is_some() {
            // The conjunction collapsed to a variable that already carries a
            // phase. Keep both contributions by allocating a fresh selector.
            let fresh = enc.fresh();
            encode_equiv(&mut enc, fresh, selector);
            enc.angles[fresh as usize] = Some(angle.clone());
        } else {
            enc.angles[selector as usize] = Some(angle.clone());
        }
    }
    for (anf, angle) in &ps.phase.packed {
        if angle.is_zero() {
            continue;
        }
        let (mut bits, const_one) = boolean_bits(&mut enc, anf, &map)?;
        let selector = enc.fresh();
        bits.push(selector);
        encode_parity(&mut enc, &bits, const_one);
        enc.angles[selector as usize] = Some(angle.clone());
    }
    Ok(Encoded {
        nvars: (enc.angles.len() - 1) as u32,
        clauses: enc.clauses,
        angles: enc.angles,
        path_vars: ps.path_vars.len() as u32,
        inputs: ps.qubits as u32,
    })
}

struct Encoder {
    clauses: Vec<Vec<i32>>,
    angles: Vec<Option<Angle>>,
}

impl Encoder {
    fn fresh(&mut self) -> i32 {
        let id = self.angles.len() as i32;
        self.angles.push(None);
        id
    }

    fn clause(&mut self, lits: impl IntoIterator<Item = i32>) {
        self.clauses.push(lits.into_iter().collect());
    }
}

fn encode_output_matches(
    enc: &mut Encoder,
    out: &Anf,
    input: i32,
    map: &BTreeMap<u16, i32>,
) -> Result<(), EqError> {
    let (mut lits, const_one) = boolean_bits(enc, out, map)?;
    lits.push(input);
    encode_parity(enc, &lits, const_one);
    Ok(())
}

/// Returns bits whose XOR equals the ANF, plus whether a constant 1 is included.
fn boolean_bits(
    enc: &mut Encoder,
    anf: &Anf,
    map: &BTreeMap<u16, i32>,
) -> Result<(Vec<i32>, bool), EqError> {
    let mut lits = Vec::new();
    let mut const_one = false;
    for term in &anf.terms {
        if term.is_empty() {
            const_one ^= true;
        } else if term.degree() == 1 {
            let v = term.0[0];
            let bit = map
                .get(&v)
                .copied()
                .ok_or_else(|| EqError(format!("variable {v} is missing from the encoding")))?;
            lits.push(bit);
        } else {
            let bits: Result<Vec<i32>, EqError> = term
                .0
                .iter()
                .map(|v| {
                    map.get(v).copied().ok_or_else(|| {
                        EqError(format!("variable {v} is missing from the encoding"))
                    })
                })
                .collect();
            lits.push(encode_conjunction(enc, &bits?));
        }
    }
    Ok((lits, const_one))
}

fn encode_conjunction(enc: &mut Encoder, bits: &[i32]) -> i32 {
    match bits {
        [] => {
            let s = enc.fresh();
            enc.clause([s]);
            s
        }
        [only] => *only,
        _ => {
            let mut prev = bits[0];
            for bit in &bits[1..] {
                let n = enc.fresh();
                // n ↔ prev ∧ bit
                enc.clause([-n, prev]);
                enc.clause([-n, *bit]);
                enc.clause([n, -prev, -*bit]);
                prev = n;
            }
            prev
        }
    }
}

fn encode_equiv(enc: &mut Encoder, a: i32, b: i32) {
    enc.clause([a, -b]);
    enc.clause([-a, b]);
}

/// `XOR(lits) = want_one`.
fn encode_parity(enc: &mut Encoder, lits: &[i32], want_one: bool) {
    if lits.is_empty() {
        if want_one {
            enc.clause([]);
        }
        return;
    }
    if lits.len() == 1 {
        if want_one {
            enc.clause([lits[0]]);
        } else {
            enc.clause([-lits[0]]);
        }
        return;
    }
    let mut prev = lits[0];
    for bit in &lits[1..lits.len() - 1] {
        let n = enc.fresh();
        // n ↔ prev ⊕ bit
        enc.clause([-prev, -*bit, -n]);
        enc.clause([-prev, *bit, n]);
        enc.clause([prev, -*bit, n]);
        enc.clause([prev, *bit, -n]);
        prev = n;
    }
    let last = lits[lits.len() - 1];
    if want_one {
        enc.clause([prev, last]);
        enc.clause([-prev, -last]);
    } else {
        enc.clause([prev, -last]);
        enc.clause([-prev, last]);
    }
}

/// Silence a warning if Mono is considered unused by a future edit.
#[allow(dead_code)]
fn _mono_ty(m: Mono) -> Mono {
    m
}
