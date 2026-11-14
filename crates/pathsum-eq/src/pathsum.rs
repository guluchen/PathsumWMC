//! Path-sum semantics for the gate set {H, X, Y, Z, S, T, CX, CZ, Rz, P, CCX}.
//!
//! A path-sum stores Boolean outputs in algebraic normal form and a phase
//! polynomial. Hadamard is the only gate that allocates a fresh path variable.
//! `Rx` and `Ry` are decomposed into `H` and `Rz` before they touch the
//! polynomial, using the Qiskit identities `Rx(θ) = H Rz(θ) H` and
//! `Ry(θ) = Rz(π/2) Rx(θ) Rz(-π/2)`.

use std::collections::BTreeSet;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::One;

use crate::anf::Anf;
use crate::ball::ComplexBall;
use crate::circuit::{Circuit, Gate};
use crate::error::EqError;
use crate::exact::ExactAmp;
use crate::phase::{Angle, Phase};
use crate::zpoly::{zpoly_from_anf, ZPoly};

#[derive(Clone, Debug)]
pub struct PathSum {
    pub qubits: usize,
    pub outputs: Vec<Anf>,
    pub phase: Phase,
    pub path_vars: BTreeSet<u16>,
    pub next_var: u16,
    /// Global amplitude produced when an unused path variable is eliminated.
    pub prefactor: ExactAmp,
    pub steps: Vec<String>,
    /// Reduce after every gate. Weighted-model-counting-only mode leaves this off.
    eager: bool,
}

impl PathSum {
    pub fn identity(qubits: usize) -> Self {
        let outputs = (0..qubits).map(|i| Anf::var(i as u16)).collect();
        Self {
            qubits,
            outputs,
            phase: Phase::default(),
            path_vars: BTreeSet::new(),
            next_var: qubits as u16,
            prefactor: ExactAmp::one(),
            steps: Vec::new(),
            eager: false,
        }
    }

    pub fn from_circuit(circuit: &Circuit) -> Result<Self, EqError> {
        Self::from_circuit_eager(circuit, true)
    }

    fn from_circuit_eager(circuit: &Circuit, eager: bool) -> Result<Self, EqError> {
        let mut ps = Self::identity(circuit.qubits);
        ps.eager = eager;
        for gate in &circuit.gates {
            ps.apply(gate)?;
        }
        ps.phase.normalize();
        Ok(ps)
    }

    /// Path-sum of `U(right) U(left)†`.
    pub fn compose_equivalence(left: &Circuit, right: &Circuit) -> Result<Self, EqError> {
        Self::compose(left, right, true)
    }

    /// `eager` reduces after every gate, so internal paths disappear before the
    /// next Hadamard multiplies them into the phase.
    pub fn compose(left: &Circuit, right: &Circuit, eager: bool) -> Result<Self, EqError> {
        if left.qubits != right.qubits {
            return Err(EqError(format!(
                "the circuits act on {} and {} qubits",
                left.qubits, right.qubits
            )));
        }
        let mut ps = Self::from_circuit_eager(&left.dagger(), eager)?;
        for gate in &right.gates {
            ps.apply(gate)?;
        }
        ps.phase.normalize();
        Ok(ps)
    }

    pub fn apply(&mut self, gate: &Gate) -> Result<(), EqError> {
        for q in gate.qubits() {
            if q >= self.qubits {
                return Err(EqError(format!(
                    "qubit {q} is outside a circuit of {} qubits",
                    self.qubits
                )));
            }
        }
        match gate {
            Gate::H(q) => self.apply_h(*q)?,
            Gate::X(q) => {
                let t = self.outputs[*q].not();
                self.outputs[*q] = t;
            }
            Gate::Y(q) => {
                // Y|x⟩ = i (−1)^x |¬x⟩.
                self.add_phase_anf(
                    &self.outputs[*q].clone(),
                    &Angle::from_pi(BigRational::one()),
                )?;
                self.add_constant(Angle::from_pi(BigRational::new(
                    BigInt::one(),
                    BigInt::from(2),
                )));
                let t = self.outputs[*q].not();
                self.outputs[*q] = t;
            }
            Gate::Z(q) => {
                self.add_phase_anf(
                    &self.outputs[*q].clone(),
                    &Angle::from_pi(BigRational::one()),
                )?;
            }
            Gate::S(q) => self.apply_p(
                *q,
                Angle::from_pi(BigRational::new(BigInt::one(), BigInt::from(2))),
            )?,
            Gate::Sdg(q) => self.apply_p(
                *q,
                Angle::from_pi(BigRational::new(BigInt::from(-1), BigInt::from(2))),
            )?,
            Gate::T(q) => self.apply_p(
                *q,
                Angle::from_pi(BigRational::new(BigInt::one(), BigInt::from(4))),
            )?,
            Gate::Tdg(q) => self.apply_p(
                *q,
                Angle::from_pi(BigRational::new(BigInt::from(-1), BigInt::from(4))),
            )?,
            Gate::Cx(c, t) => {
                let x = self.outputs[*c].clone().xor(&self.outputs[*t]);
                self.outputs[*t] = x;
            }
            Gate::Cz(c, t) => self.apply_cp(*c, *t, Angle::from_pi(BigRational::one()))?,
            Gate::Swap(a, b) => self.outputs.swap(*a, *b),
            Gate::Ccx(a, b, t) => {
                let and = self.outputs[*a].clone().and(&self.outputs[*b]);
                let x = self.outputs[*t].clone().xor(&and);
                self.outputs[*t] = x;
            }
            Gate::Ccz(a, b, t) => {
                let prod = self.outputs[*a]
                    .clone()
                    .and(&self.outputs[*b])
                    .and(&self.outputs[*t]);
                self.add_integer_pi(&prod, true);
            }
            Gate::Rz(q, theta) => {
                // diag(e^{-iθ/2}, e^{iθ/2}) = e^{-iθ/2} diag(1, e^{iθ}).
                self.apply_p(*q, theta.clone())?;
                self.add_constant(
                    theta.scale(&BigRational::new(BigInt::from(-1), BigInt::from(2))),
                );
            }
            Gate::P(q, theta) => self.apply_p(*q, theta.clone())?,
            Gate::Rx(q, theta) => {
                self.apply(&Gate::H(*q))?;
                self.apply(&Gate::Rz(*q, theta.clone()))?;
                self.apply(&Gate::H(*q))?;
            }
            Gate::Ry(q, theta) => {
                // Ry(θ) = Rz(π/2) Rx(θ) Rz(−π/2). The rightmost factor is applied first.
                let half = Angle::from_pi(BigRational::new(BigInt::one(), BigInt::from(2)));
                self.apply(&Gate::Rz(*q, half.neg()))?;
                self.apply(&Gate::Rx(*q, theta.clone()))?;
                self.apply(&Gate::Rz(*q, half))?;
            }
            Gate::Cp(c, t, theta) => self.apply_cp(*c, *t, theta.clone())?,
            Gate::Crz(c, t, theta) => {
                // Controlled rz: phase θ(t − 1/2) when the control is 1.
                self.apply_cp(*c, *t, theta.clone())?;
                let half = theta.scale(&BigRational::new(BigInt::from(-1), BigInt::from(2)));
                self.add_phase_anf(&self.outputs[*c].clone(), &half)?;
            }
            Gate::Crx(c, t, theta) => {
                self.apply(&Gate::H(*t))?;
                self.apply(&Gate::Crz(*c, *t, theta.clone()))?;
                self.apply(&Gate::H(*t))?;
            }
            Gate::Cry(c, t, theta) => {
                let half = Angle::from_pi(BigRational::new(BigInt::one(), BigInt::from(2)));
                self.apply(&Gate::Rz(*t, half.neg()))?;
                self.apply(&Gate::Crx(*c, *t, theta.clone()))?;
                self.apply(&Gate::Rz(*t, half))?;
            }
        }
        if self.eager {
            crate::reduce::reduce(self)?;
        }
        Ok(())
    }

    fn apply_h(&mut self, q: usize) -> Result<(), EqError> {
        let y = self.fresh();
        let current = self.outputs[q].clone();
        let ymono = crate::anf::Mono::var(y);
        // π y f with f in {0,1}. The integer cross terms are multiples of 2π.
        for term in &current.terms {
            self.phase
                .add_mono(term.union(&ymono), Angle::from_pi(BigRational::one()));
        }
        self.outputs[q] = Anf::var(y);
        Ok(())
    }

    fn apply_p(&mut self, q: usize, theta: Angle) -> Result<(), EqError> {
        self.add_phase_anf(&self.outputs[q].clone(), &theta)
    }

    fn apply_cp(&mut self, c: usize, t: usize, theta: Angle) -> Result<(), EqError> {
        let prod = self.outputs[c].clone().and(&self.outputs[t]);
        self.add_phase_anf(&prod, &theta)
    }

    fn add_phase_anf(&mut self, anf: &Anf, angle: &Angle) -> Result<(), EqError> {
        if let Some(odd) = angle.integer_pi_parity() {
            self.add_integer_pi(anf, odd);
            return Ok(());
        }
        if anf.terms.len() <= 8 {
            let poly = zpoly_from_anf(anf)?;
            self.add_phase_poly(&poly, angle);
            return Ok(());
        }
        self.phase.add_packed(anf.clone(), angle.clone());
        Ok(())
    }

    fn add_integer_pi(&mut self, anf: &Anf, odd: bool) {
        if !odd {
            return;
        }
        let kept = Angle::from_pi(BigRational::one());
        for term in &anf.terms {
            self.phase.add_mono(term.clone(), kept.clone());
        }
    }

    fn add_phase_poly(&mut self, poly: &ZPoly, angle: &Angle) {
        for (mono, coeff) in &poly.terms {
            self.phase.add_mono(mono.clone(), angle.scale(coeff));
        }
    }

    fn add_constant(&mut self, angle: Angle) {
        self.phase.add_mono(crate::anf::Mono::empty(), angle);
    }

    fn fresh(&mut self) -> u16 {
        let y = self.next_var;
        self.next_var += 1;
        self.path_vars.insert(y);
        y
    }

    pub fn path_var_count(&self) -> u32 {
        self.path_vars.len() as u32
    }

    /// Sum of diagonal amplitudes, by enumerating paths. Used as an oracle
    /// for the weighted-model-counting encoding on small instances.
    pub fn diagonal_sum_direct(&self, prec: u32) -> Result<AmpValue, EqError> {
        let inputs: Vec<u16> = (0..self.qubits as u16).collect();
        let paths: Vec<u16> = self.path_vars.iter().copied().collect();
        let total = inputs.len() + paths.len();
        if total > 16 {
            return Err(EqError(
                "direct path enumeration supports at most 16 input and path variables".into(),
            ));
        }
        let quarters = self.phase.terms.values().all(|a| a.is_quarter_turn())
            && self.phase.packed.iter().all(|(_, a)| a.is_quarter_turn());
        if quarters {
            let mut sum = ExactAmp::zero();
            for mask in 0..(1u32 << total) {
                let env = |v: u16| lookup_bit(v, &inputs, &paths, mask);
                if !self.outputs_match(&env) {
                    continue;
                }
                let mut amp = ExactAmp::one();
                for (mono, angle) in &self.phase.terms {
                    if mono_true(mono, &env) {
                        let k = angle.quarter_index().expect("quarter");
                        amp = amp.mul(&ExactAmp::exp_i_pi_quarters(&k));
                    }
                }
                for (anf, angle) in &self.phase.packed {
                    if anf.eval(&env) {
                        let k = angle.quarter_index().expect("quarter");
                        amp = amp.mul(&ExactAmp::exp_i_pi_quarters(&k));
                    }
                }
                sum = sum.add(&amp);
            }
            sum = self.prefactor.mul(&sum).div_sqrt2_pow(paths.len() as u32);
            Ok(AmpValue::Exact(sum))
        } else {
            let mut sum = ComplexBall::zero(prec);
            for mask in 0..(1u32 << total) {
                let env = |v: u16| lookup_bit(v, &inputs, &paths, mask);
                if !self.outputs_match(&env) {
                    continue;
                }
                let mut theta = crate::ball::RealBall::zero(prec);
                for (mono, angle) in &self.phase.terms {
                    if mono_true(mono, &env) {
                        theta = theta.add(&angle_ball(angle, prec));
                    }
                }
                for (anf, angle) in &self.phase.packed {
                    if anf.eval(&env) {
                        theta = theta.add(&angle_ball(angle, prec));
                    }
                }
                sum = sum.add(&ComplexBall::exp_i(&theta));
            }
            sum = sum
                .mul(&self.prefactor.to_ball(prec))
                .div_sqrt2_pow(paths.len() as u32);
            Ok(AmpValue::Ball(sum))
        }
    }

    fn outputs_match(&self, env: &dyn Fn(u16) -> bool) -> bool {
        self.outputs
            .iter()
            .enumerate()
            .all(|(i, anf)| anf.eval(env) == env(i as u16))
    }
}

/// Diagonal sum, either exact in ℚ(i, √2) or a complex ball.
#[derive(Clone, Debug)]
pub enum AmpValue {
    Exact(ExactAmp),
    Ball(ComplexBall),
}

fn lookup_bit(v: u16, inputs: &[u16], paths: &[u16], mask: u32) -> bool {
    if let Some(i) = inputs.iter().position(|x| *x == v) {
        return (mask >> i) & 1 == 1;
    }
    if let Some(i) = paths.iter().position(|x| *x == v) {
        return (mask >> (inputs.len() + i)) & 1 == 1;
    }
    false
}

fn mono_true(mono: &crate::anf::Mono, env: &dyn Fn(u16) -> bool) -> bool {
    mono.0.iter().all(|v| env(*v))
}

pub fn angle_ball(angle: &Angle, prec: u32) -> crate::ball::RealBall {
    let rad = crate::ball::RealBall::from_rational(&angle.rad, prec);
    let pi =
        crate::ball::RealBall::pi(prec).mul(&crate::ball::RealBall::from_rational(&angle.pi, prec));
    rad.add(&pi)
}

/// Syntactic identity, up to a constant phase. `None` when path variables or
/// a non-trivial output remain.
pub fn syntactic_global_phase(ps: &PathSum) -> Option<Angle> {
    if !ps.path_vars.is_empty() || !ps.prefactor.is_one() {
        return None;
    }
    for (i, out) in ps.outputs.iter().enumerate() {
        if out != &Anf::var(i as u16) {
            return None;
        }
    }
    let mut constant = Angle::zero();
    if !ps.phase.packed.is_empty() {
        return None;
    }
    for (mono, angle) in &ps.phase.terms {
        if mono.is_empty() {
            constant = constant.add(angle);
        } else {
            return None;
        }
    }
    Some(constant)
}
