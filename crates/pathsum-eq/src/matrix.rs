//! State-vector reference for circuits small enough to multiply out.
//!
//! Gate matrices are the OpenQASM / Qiskit definitions. `Rx` and `Ry` are
//! applied from their closed forms, not from the path-sum decomposition, so
//! this oracle does not share that code path.

use num_traits::One;

use crate::ball::{ComplexBall, RealBall};
use crate::circuit::{Circuit, Gate};
use crate::error::EqError;
use crate::pathsum::angle_ball;
use crate::phase::Angle;

pub fn matrix_diagonal(left: &Circuit, right: &Circuit, prec: u32) -> Result<ComplexBall, EqError> {
    if left.qubits != right.qubits {
        return Err(EqError("qubit counts differ".into()));
    }
    if left.qubits > 3 {
        return Err(EqError(
            "the matrix reference is limited to 3 qubits in the browser build".into(),
        ));
    }
    let n = left.qubits;
    let dim = 1usize << n;
    let mut sum = ComplexBall::zero(prec);
    let dagger = left.dagger();
    for basis in 0..dim {
        let mut state = vec![ComplexBall::zero(prec); dim];
        state[basis] = ComplexBall::one(prec);
        for gate in &dagger.gates {
            apply_gate(&mut state, n, gate, prec)?;
        }
        for gate in &right.gates {
            apply_gate(&mut state, n, gate, prec)?;
        }
        sum = sum.add(&state[basis]);
    }
    Ok(sum)
}

fn apply_gate(state: &mut [ComplexBall], n: usize, gate: &Gate, prec: u32) -> Result<(), EqError> {
    match gate {
        Gate::H(q) => {
            let scale = ComplexBall::new(RealBall::sqrt2(prec).recip(), RealBall::zero(prec));
            for_pairs(state, n, *q, |a, b| {
                let sum = a.add(b);
                let diff = a.sub(b);
                (scale.mul(&sum), scale.mul(&diff))
            });
        }
        Gate::X(q) => {
            for_pairs(state, n, *q, |a, b| (b.clone(), a.clone()));
        }
        Gate::Y(q) => {
            let i = ComplexBall::i(prec);
            let minus_i = i.neg();
            for_pairs(state, n, *q, |a, b| (minus_i.mul(b), i.mul(a)));
        }
        Gate::Z(q) => phase_on_one(state, n, *q, &ComplexBall::one(prec).neg()),
        Gate::S(q) => phase_on_one(state, n, *q, &ComplexBall::i(prec)),
        Gate::Sdg(q) => phase_on_one(state, n, *q, &ComplexBall::i(prec).neg()),
        Gate::T(q) => {
            let t = exp_angle(
                &Angle::from_pi(num_rational::BigRational::new(
                    num_bigint::BigInt::one(),
                    num_bigint::BigInt::from(4),
                )),
                prec,
            );
            phase_on_one(state, n, *q, &t);
        }
        Gate::Tdg(q) => {
            let t = exp_angle(
                &Angle::from_pi(num_rational::BigRational::new(
                    num_bigint::BigInt::from(-1),
                    num_bigint::BigInt::from(4),
                )),
                prec,
            );
            phase_on_one(state, n, *q, &t);
        }
        Gate::Cx(c, t) => controlled_swap_target(state, &[*c], *t),
        Gate::Cz(c, t) => {
            let minus = ComplexBall::one(prec).neg();
            phase_on_bits(state, &[*c, *t], &minus);
        }
        Gate::Swap(a, b) => swap_qubits(state, n, *a, *b),
        Gate::Ccx(a, b, t) => controlled_swap_target(state, &[*a, *b], *t),
        Gate::Ccz(a, b, t) => {
            let minus = ComplexBall::one(prec).neg();
            phase_on_bits(state, &[*a, *b, *t], &minus);
        }
        Gate::Rz(q, theta) => {
            let half = half_angle(theta, prec);
            let low = ComplexBall::exp_i(&half.neg());
            let high = ComplexBall::exp_i(&half);
            for_pairs(state, n, *q, |a, b| (low.mul(a), high.mul(b)));
        }
        Gate::P(q, theta) => phase_on_one(state, n, *q, &exp_angle(theta, prec)),
        Gate::Rx(q, theta) => {
            let half = half_angle(theta, prec);
            let c = ComplexBall::new(half.cos(), RealBall::zero(prec));
            let minus_i_s = ComplexBall::new(RealBall::zero(prec), half.sin().neg());
            for_pairs(state, n, *q, |a, b| {
                (
                    c.mul(a).add(&minus_i_s.mul(b)),
                    minus_i_s.mul(a).add(&c.mul(b)),
                )
            });
        }
        Gate::Ry(q, theta) => {
            let half = half_angle(theta, prec);
            let c = ComplexBall::new(half.cos(), RealBall::zero(prec));
            let s = ComplexBall::new(half.sin(), RealBall::zero(prec));
            let ns = s.neg();
            for_pairs(state, n, *q, |a, b| {
                (c.mul(a).add(&ns.mul(b)), s.mul(a).add(&c.mul(b)))
            });
        }
        Gate::Cp(c, t, theta) => phase_on_bits(state, &[*c, *t], &exp_angle(theta, prec)),
        Gate::Crz(c, t, theta) => {
            let half = half_angle(theta, prec);
            let low = ComplexBall::exp_i(&half.neg());
            let high = ComplexBall::exp_i(&half);
            let bc = 1usize << c;
            let bt = 1usize << t;
            let snapshot: Vec<_> = state.to_vec();
            for i in 0..state.len() {
                if snapshot.len() != state.len() {
                    break;
                }
                if i & bc == 0 {
                    continue;
                }
                let factor = if i & bt == 0 { &low } else { &high };
                state[i] = factor.mul(&snapshot[i]);
            }
        }
        Gate::Crx(c, t, _) | Gate::Cry(c, t, _) => {
            // Decompose only inside the matrix oracle for the controlled
            // rotations that are not needed as closed forms. The uncontrolled
            // factors are exact matrix gates, and the controlled-Z rotation
            // above is closed form.
            let mut pieces = Vec::new();
            match gate {
                Gate::Crx(_, _, theta) => {
                    pieces.push(Gate::H(*t));
                    pieces.push(Gate::Crz(*c, *t, theta.clone()));
                    pieces.push(Gate::H(*t));
                }
                Gate::Cry(_, _, theta) => {
                    let half = Angle::from_pi(num_rational::BigRational::new(
                        num_bigint::BigInt::one(),
                        num_bigint::BigInt::from(2),
                    ));
                    pieces.push(Gate::Rz(*t, half.neg()));
                    pieces.push(Gate::H(*t));
                    pieces.push(Gate::Crz(*c, *t, theta.clone()));
                    pieces.push(Gate::H(*t));
                    pieces.push(Gate::Rz(*t, half));
                }
                _ => {}
            }
            for piece in pieces {
                apply_gate(state, n, &piece, prec)?;
            }
        }
    }
    Ok(())
}

fn for_pairs(
    state: &mut [ComplexBall],
    n: usize,
    q: usize,
    mut f: impl FnMut(&ComplexBall, &ComplexBall) -> (ComplexBall, ComplexBall),
) {
    let bit = 1usize << q;
    let _ = n;
    for i in 0..state.len() {
        if i & bit == 0 {
            let j = i | bit;
            let (ni, nj) = f(&state[i], &state[j]);
            state[i] = ni;
            state[j] = nj;
        }
    }
}

fn phase_on_one(state: &mut [ComplexBall], _n: usize, q: usize, factor: &ComplexBall) {
    let bit = 1usize << q;
    for amp in state.iter_mut().enumerate() {
        if amp.0 & bit != 0 {
            *amp.1 = factor.mul(amp.1);
        }
    }
}

fn phase_on_bits(state: &mut [ComplexBall], qs: &[usize], factor: &ComplexBall) {
    let mask = qs.iter().fold(0usize, |acc, q| acc | (1usize << q));
    for (i, amp) in state.iter_mut().enumerate() {
        if i & mask == mask {
            *amp = factor.mul(amp);
        }
    }
}

fn controlled_swap_target(state: &mut [ComplexBall], controls: &[usize], target: usize) {
    let mask = controls.iter().fold(0usize, |acc, q| acc | (1usize << q));
    let bt = 1usize << target;
    for i in 0..state.len() {
        if i & mask == mask && i & bt == 0 {
            let j = i | bt;
            state.swap(i, j);
        }
    }
}

fn swap_qubits(state: &mut [ComplexBall], _n: usize, a: usize, b: usize) {
    if a == b {
        return;
    }
    let ba = 1usize << a;
    let bb = 1usize << b;
    for i in 0..state.len() {
        if i & ba == 0 && i & bb != 0 {
            let j = (i | ba) & !bb;
            state.swap(i, j);
        }
    }
}

fn exp_angle(angle: &Angle, prec: u32) -> ComplexBall {
    if let Some(k) = angle.quarter_index() {
        crate::exact::ExactAmp::exp_i_pi_quarters(&k).to_ball(prec)
    } else {
        ComplexBall::exp_i(&angle_ball(angle, prec))
    }
}

fn half_angle(angle: &Angle, prec: u32) -> RealBall {
    angle_ball(angle, prec).mul(&RealBall::from_rational(
        &num_rational::BigRational::new(num_bigint::BigInt::one(), num_bigint::BigInt::from(2)),
        prec,
    ))
}

trait SubBall {
    fn sub(&self, rhs: &Self) -> Self;
}
impl SubBall for ComplexBall {
    fn sub(&self, rhs: &Self) -> Self {
        self.add(&rhs.neg())
    }
}
