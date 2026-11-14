//! Three weighted model counters over a commutative semiring.
//!
//! * `enumerate` sums the weight of every model by backtracking.
//! * `dpll` is the same sum with unit propagation and activity branching.
//! * `bucket` is variable elimination on the clause factors.
//!
//! All three implement the paper's weighted model count. They are separate
//! algorithms, so agreement between them is a check on the encoding.

use crate::ball::ComplexBall;
use crate::encode::Encoded;
use crate::error::EqError;
use crate::exact::ExactAmp;
use crate::pathsum::angle_ball;
use crate::phase::Angle;

pub trait Ring: Clone {
    fn zero() -> Self;
    fn one() -> Self;
    fn add(&self, rhs: &Self) -> Self;
    fn mul(&self, rhs: &Self) -> Self;
}

impl Ring for ExactAmp {
    fn zero() -> Self {
        ExactAmp::zero()
    }
    fn one() -> Self {
        ExactAmp::one()
    }
    fn add(&self, rhs: &Self) -> Self {
        ExactAmp::add(self, rhs)
    }
    fn mul(&self, rhs: &Self) -> Self {
        ExactAmp::mul(self, rhs)
    }
}

impl Ring for ComplexBall {
    fn zero() -> Self {
        // Precision is recovered from the other operand in add/mul. A bare
        // zero is only used as an accumulator and immediately added to a
        // weight that carries the working precision.
        ComplexBall::zero(32)
    }
    fn one() -> Self {
        ComplexBall::one(32)
    }
    fn add(&self, rhs: &Self) -> Self {
        ComplexBall::add(self, rhs)
    }
    fn mul(&self, rhs: &Self) -> Self {
        ComplexBall::mul(self, rhs)
    }
}

#[derive(Clone)]
pub struct Problem<T: Clone> {
    pub nvars: u32,
    pub clauses: Vec<Vec<i32>>,
    pub w0: Vec<T>,
    pub w1: Vec<T>,
}

pub fn problem_exact(enc: &Encoded) -> Option<Problem<ExactAmp>> {
    let w0 = vec![ExactAmp::one(); enc.nvars as usize + 1];
    let mut w1 = vec![ExactAmp::one(); enc.nvars as usize + 1];
    for (v, angle) in enc.angles.iter().enumerate().skip(1) {
        if let Some(angle) = angle {
            let k = angle.quarter_index()?;
            w1[v] = ExactAmp::exp_i_pi_quarters(&k);
        }
    }
    Some(Problem {
        nvars: enc.nvars,
        clauses: enc.clauses.clone(),
        w0,
        w1,
    })
}

pub fn problem_ball(enc: &Encoded, prec: u32) -> Problem<ComplexBall> {
    let one = ComplexBall::one(prec);
    let w0 = vec![one.clone(); enc.nvars as usize + 1];
    let mut w1 = vec![one; enc.nvars as usize + 1];
    for (v, angle) in enc.angles.iter().enumerate().skip(1) {
        if let Some(angle) = angle {
            w1[v] = weight_ball(angle, prec);
        }
    }
    Problem {
        nvars: enc.nvars,
        clauses: enc.clauses.clone(),
        w0,
        w1,
    }
}

pub fn weight_ball(angle: &Angle, prec: u32) -> ComplexBall {
    if let Some(k) = angle.quarter_index() {
        return ExactAmp::exp_i_pi_quarters(&k).to_ball(prec);
    }
    ComplexBall::exp_i(&angle_ball(angle, prec))
}

pub fn enumerate<T: Ring>(problem: &Problem<T>, max_nodes: u64) -> Result<T, EqError> {
    if problem.clauses.iter().any(|c| c.is_empty()) {
        return Ok(T::zero());
    }
    let mut asg = vec![0i8; problem.nvars as usize + 1];
    let mut acc = T::zero();
    let mut nodes = 0u64;
    rec_enumerate(problem, &mut asg, &mut acc, &mut nodes, max_nodes, 1)?;
    Ok(acc)
}

fn rec_enumerate<T: Ring>(
    problem: &Problem<T>,
    asg: &mut [i8],
    acc: &mut T,
    nodes: &mut u64,
    max_nodes: u64,
    next: u32,
) -> Result<(), EqError> {
    *nodes += 1;
    if *nodes > max_nodes {
        return Err(EqError(
            "exhaustive enumeration exceeded its node limit; try dpll or bucket elimination".into(),
        ));
    }
    if conflict(problem, asg) {
        return Ok(());
    }
    if next > problem.nvars {
        *acc = acc.add(&assignment_weight(problem, asg));
        return Ok(());
    }
    asg[next as usize] = -1;
    rec_enumerate(problem, asg, acc, nodes, max_nodes, next + 1)?;
    asg[next as usize] = 1;
    rec_enumerate(problem, asg, acc, nodes, max_nodes, next + 1)?;
    asg[next as usize] = 0;
    Ok(())
}

pub fn dpll<T: Ring>(problem: &Problem<T>, max_nodes: u64) -> Result<T, EqError> {
    if problem.clauses.iter().any(|c| c.is_empty()) {
        return Ok(T::zero());
    }
    let mut asg = vec![0i8; problem.nvars as usize + 1];
    let mut nodes = 0u64;
    rec_dpll(problem, &mut asg, &mut nodes, max_nodes)
}

fn rec_dpll<T: Ring>(
    problem: &Problem<T>,
    asg: &mut [i8],
    nodes: &mut u64,
    max_nodes: u64,
) -> Result<T, EqError> {
    *nodes += 1;
    if *nodes > max_nodes {
        return Err(EqError(
            "DPLL exceeded its node limit; try bucket elimination or a smaller circuit".into(),
        ));
    }
    if conflict(problem, asg) {
        return Ok(T::zero());
    }
    let mut implied = Vec::new();
    while let Some((var, val)) = find_unit(problem, asg) {
        asg[var] = val;
        implied.push(var);
        if conflict(problem, asg) {
            for v in &implied {
                asg[*v] = 0;
            }
            return Ok(T::zero());
        }
    }
    let branch = pick_branch(problem, asg);
    let result = if let Some(var) = branch {
        asg[var] = -1;
        let left = rec_dpll(problem, asg, nodes, max_nodes);
        asg[var] = 1;
        let right = rec_dpll(problem, asg, nodes, max_nodes);
        asg[var] = 0;
        match (left, right) {
            (Ok(a), Ok(b)) => Ok(a.add(&b)),
            (Err(e), _) | (_, Err(e)) => Err(e),
        }
    } else {
        Ok(assignment_weight(problem, asg))
    };
    for v in implied {
        asg[v] = 0;
    }
    result
}

pub fn bucket<T: Ring>(problem: &Problem<T>, max_width: usize) -> Result<T, EqError> {
    if problem.clauses.iter().any(|c| c.is_empty()) {
        return Ok(T::zero());
    }
    let mut factors: Vec<Factor<T>> = Vec::new();
    for clause in &problem.clauses {
        if let Some(factor) = clause_factor(clause)? {
            factors.push(factor);
        }
    }
    for v in 1..=problem.nvars {
        factors.push(Factor {
            vars: vec![v],
            table: vec![
                problem.w0[v as usize].clone(),
                problem.w1[v as usize].clone(),
            ],
        });
    }
    while let Some(var) = cheapest_var(&factors, problem.nvars) {
        let mut joined: Option<Factor<T>> = None;
        let mut rest = Vec::new();
        for factor in factors.drain(..) {
            if factor.vars.binary_search(&var).is_ok() {
                joined = Some(match joined {
                    None => factor,
                    Some(prev) => join(&prev, &factor, max_width)?,
                });
            } else {
                rest.push(factor);
            }
        }
        if let Some(factor) = joined {
            rest.push(sum_out(&factor, var));
        }
        factors = rest;
    }
    let mut acc = T::one();
    for factor in factors {
        if factor.vars.is_empty() {
            acc = acc.mul(&factor.table[0]);
        } else {
            return Err(EqError(
                "bucket elimination left a factor with uneliminated variables".into(),
            ));
        }
    }
    Ok(acc)
}

struct Factor<T> {
    vars: Vec<u32>,
    table: Vec<T>,
}

fn clause_factor<T: Ring>(clause: &[i32]) -> Result<Option<Factor<T>>, EqError> {
    let mut vars = Vec::new();
    for lit in clause {
        let v = lit.unsigned_abs();
        if !vars.contains(&v) {
            vars.push(v);
        }
    }
    vars.sort_unstable();
    if clause.iter().any(|lit| clause.contains(&-lit)) {
        return Ok(None);
    }
    if vars.len() > 22 {
        return Err(EqError(
            "a clause is wider than bucket elimination allows".into(),
        ));
    }
    let mut table = vec![T::zero(); 1usize << vars.len()];
    for (mask, slot) in table.iter_mut().enumerate() {
        let sat = clause.iter().any(|lit| {
            let v = lit.unsigned_abs();
            let idx = vars.iter().position(|x| *x == v).unwrap();
            let bit = (mask >> idx) & 1 == 1;
            if *lit > 0 {
                bit
            } else {
                !bit
            }
        });
        *slot = if sat { T::one() } else { T::zero() };
    }
    Ok(Some(Factor { vars, table }))
}

fn join<T: Ring>(a: &Factor<T>, b: &Factor<T>, max_width: usize) -> Result<Factor<T>, EqError> {
    let mut vars = a.vars.clone();
    for v in &b.vars {
        if !vars.contains(v) {
            vars.push(*v);
        }
    }
    vars.sort_unstable();
    if vars.len() > max_width {
        return Err(EqError(format!(
            "bucket elimination reached width {}, above the limit of {max_width}",
            vars.len()
        )));
    }
    let mut table = Vec::with_capacity(1usize << vars.len());
    for mask in 0..(1usize << vars.len()) {
        let ia = project(mask, &vars, &a.vars);
        let ib = project(mask, &vars, &b.vars);
        table.push(a.table[ia].mul(&b.table[ib]));
    }
    Ok(Factor { vars, table })
}

fn sum_out<T: Ring>(factor: &Factor<T>, var: u32) -> Factor<T> {
    let Some(pos) = factor.vars.iter().position(|v| *v == var) else {
        return Factor {
            vars: factor.vars.clone(),
            table: factor.table.clone(),
        };
    };
    let vars: Vec<u32> = factor.vars.iter().copied().filter(|v| *v != var).collect();
    let mut table = vec![T::zero(); 1usize << vars.len()];
    for (mask, value) in factor.table.iter().enumerate() {
        let low = mask & ((1usize << pos) - 1);
        let high = mask >> (pos + 1);
        let new_mask = low | (high << pos);
        table[new_mask] = table[new_mask].add(value);
    }
    Factor { vars, table }
}

fn project(mask: usize, big: &[u32], small: &[u32]) -> usize {
    let mut out = 0;
    for (i, var) in small.iter().enumerate() {
        let pos = big.iter().position(|v| v == var).unwrap();
        if (mask >> pos) & 1 == 1 {
            out |= 1usize << i;
        }
    }
    out
}

fn cheapest_var<T>(factors: &[Factor<T>], nvars: u32) -> Option<u32> {
    let mut degree = vec![0u32; nvars as usize + 1];
    for factor in factors {
        for v in &factor.vars {
            degree[*v as usize] += 1;
        }
    }
    let mut best: Option<(u32, u32)> = None;
    for (var, deg) in degree.iter().enumerate().skip(1) {
        if *deg == 0 {
            continue;
        }
        let better = match best {
            None => true,
            Some((best_deg, best_var)) => {
                *deg < best_deg || (*deg == best_deg && (var as u32) < best_var)
            }
        };
        if better {
            best = Some((*deg, var as u32));
        }
    }
    best.map(|(_, var)| var)
}

fn assignment_weight<T: Ring>(problem: &Problem<T>, asg: &[i8]) -> T {
    let mut acc = T::one();
    for v in 1..=problem.nvars as usize {
        let weight = if asg[v] > 0 {
            &problem.w1[v]
        } else {
            &problem.w0[v]
        };
        acc = acc.mul(weight);
    }
    acc
}

fn conflict<T: Clone>(problem: &Problem<T>, asg: &[i8]) -> bool {
    problem.clauses.iter().any(|clause| {
        !clause.is_empty() && clause.iter().all(|lit| lit_value(*lit, asg) == Some(false))
    })
}

fn find_unit<T: Clone>(problem: &Problem<T>, asg: &[i8]) -> Option<(usize, i8)> {
    for clause in &problem.clauses {
        let mut satisfied = false;
        let mut open: Option<i32> = None;
        let mut extra = false;
        for lit in clause {
            match lit_value(*lit, asg) {
                Some(true) => {
                    satisfied = true;
                    break;
                }
                Some(false) => {}
                None => {
                    if open.is_some() {
                        extra = true;
                    }
                    open = Some(*lit);
                }
            }
        }
        if satisfied || extra {
            continue;
        }
        if let Some(lit) = open {
            let var = lit.unsigned_abs() as usize;
            let val: i8 = if lit > 0 { 1 } else { -1 };
            return Some((var, val));
        }
    }
    None
}

fn pick_branch<T: Clone>(problem: &Problem<T>, asg: &[i8]) -> Option<usize> {
    let mut count = vec![0u32; problem.nvars as usize + 1];
    for clause in &problem.clauses {
        if clause.iter().any(|lit| lit_value(*lit, asg) == Some(true)) {
            continue;
        }
        for lit in clause {
            if lit_value(*lit, asg).is_none() {
                count[lit.unsigned_abs() as usize] += 1;
            }
        }
    }
    let best = count
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, c)| **c > 0)
        .max_by_key(|(_, c)| *c)
        .map(|(i, _)| i);
    best.or_else(|| (1..=problem.nvars as usize).find(|v| asg[*v] == 0))
}

fn lit_value(lit: i32, asg: &[i8]) -> Option<bool> {
    let var = lit.unsigned_abs() as usize;
    match asg.get(var).copied().unwrap_or(0) {
        0 => None,
        value => {
            let bit = value > 0;
            Some(if lit > 0 { bit } else { !bit })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;

    fn three_models() -> Problem<ExactAmp> {
        // (x ∨ y), weights 1. Models 01, 10, 11. Count 3.
        Problem {
            nvars: 2,
            clauses: vec![vec![1, 2]],
            w0: vec![ExactAmp::one(), ExactAmp::one(), ExactAmp::one()],
            w1: vec![ExactAmp::one(), ExactAmp::one(), ExactAmp::one()],
        }
    }

    #[test]
    fn counters_agree_on_a_clause() {
        let p = three_models();
        let a = enumerate(&p, 10_000).unwrap();
        let b = dpll(&p, 10_000).unwrap();
        let c = bucket(&p, 12).unwrap();
        let expect = ExactAmp::from_three();
        assert_eq!(a, expect);
        assert_eq!(b, expect);
        assert_eq!(c, expect);
    }

    impl ExactAmp {
        fn from_three() -> Self {
            ExactAmp::one().add(&ExactAmp::one()).add(&ExactAmp::one())
        }
    }

    #[test]
    fn weighted_unit_clause() {
        let i = ExactAmp::exp_i_pi_quarters(&BigInt::from(2));
        let p = Problem {
            nvars: 1,
            clauses: vec![vec![1]],
            w0: vec![ExactAmp::one(), ExactAmp::one()],
            w1: vec![ExactAmp::one(), i.clone()],
        };
        assert_eq!(enumerate(&p, 100).unwrap(), i);
        assert_eq!(dpll(&p, 100).unwrap(), i);
        assert_eq!(bucket(&p, 8).unwrap(), i);
    }
}
