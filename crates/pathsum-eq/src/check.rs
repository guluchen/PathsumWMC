//! Glue between path-sum construction, reduction, and the model counters.

use web_time::Instant;

use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};

use crate::ball::{ComplexBall, RealBall};
use crate::circuit::Circuit;
use crate::distance::DistanceBounds;
use crate::encode::{self, Encoded};
use crate::error::EqError;
use crate::exact::ExactAmp;
use crate::matrix::matrix_diagonal;
use crate::pathsum::{angle_ball, syntactic_global_phase, AmpValue, PathSum};
use crate::phase::Angle;
use crate::qasm::parse_qasm;
use crate::reduce::reduce;
use crate::wmc::{self, Problem};

#[derive(Clone, Debug, Deserialize)]
pub struct CheckRequest {
    pub circuit1: String,
    pub circuit2: String,
    pub mode: String,
    pub counter: String,
    pub precision: u32,
    pub max_precision: u32,
    pub max_nodes: u64,
}

impl Default for CheckRequest {
    fn default() -> Self {
        Self {
            circuit1: String::new(),
            circuit2: String::new(),
            mode: "hybrid".into(),
            counter: "all".into(),
            precision: 72,
            max_precision: 256,
            max_nodes: 200_000,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CounterReport {
    pub name: String,
    pub kind: String,
    pub verdict: String,
    pub value: String,
    pub magnitude_squared: String,
    pub frobenius_distance: Option<DistanceBounds>,
    pub elapsed_ms: u64,
    pub error: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub verdict: String,
    pub headline: String,
    pub detail: String,
    pub phase: String,
    pub arithmetic: String,
    pub qubits: u32,
    pub gates1: u32,
    pub gates2: u32,
    pub path_variables_before: u32,
    pub path_variables_after: u32,
    pub monomials: u32,
    pub variables: u32,
    pub clauses: u32,
    pub frobenius_distance: Option<DistanceBounds>,
    pub precision_bits: u32,
    pub reduction_steps: Vec<String>,
    pub counters: Vec<CounterReport>,
    pub notes: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Hybrid,
    Reduction,
    Wmc,
}

pub fn check(req: &CheckRequest) -> Result<Report, EqError> {
    let left = parse_qasm(&req.circuit1)?;
    let right = parse_qasm(&req.circuit2)?;
    if left.qubits != right.qubits {
        return Ok(dimension_report(&left, &right));
    }
    let mode = parse_mode(&req.mode)?;
    let mut ps = PathSum::compose(&left, &right, mode != Mode::Wmc)?;
    let before = ps.path_var_count();
    if mode != Mode::Wmc {
        reduce(&mut ps)?;
    }
    let after = ps.path_var_count();
    if mode != Mode::Wmc {
        if let Some(angle) = syntactic_global_phase(&ps) {
            return Ok(symbolic_report(&left, &right, &ps, before, after, &angle));
        }
    }
    if mode == Mode::Reduction {
        return Ok(unknown_report(&left, &right, &ps, before, after));
    }
    let enc = encode::encode(&ps)?;
    let mut prec = req.precision.clamp(32, 4096);
    let max_prec = req.max_precision.max(prec).min(4096);
    let mut last = None;
    loop {
        let report = wmc_report(&left, &right, &ps, &enc, before, after, prec, req)?;
        let retry = report.verdict == "inconclusive" && prec < max_prec;
        if retry {
            prec = (prec.saturating_mul(2)).min(max_prec);
            last = Some(report);
            continue;
        }
        let _ = last;
        return Ok(report);
    }
}

fn wmc_report(
    left: &Circuit,
    right: &Circuit,
    ps: &PathSum,
    enc: &Encoded,
    before: u32,
    after: u32,
    prec: u32,
    req: &CheckRequest,
) -> Result<Report, EqError> {
    let names = counter_names(&req.counter)?;
    let mut counters = Vec::new();
    for name in &names {
        counters.push(run_named(name, ps, enc, prec, req.max_nodes));
    }
    if left.qubits + ps.path_vars.len() <= 12 {
        counters.push(run_direct(ps, prec));
    }
    if left.qubits <= 3 {
        counters.push(run_matrix(left, right, prec));
    }
    let (verdict, phase, arithmetic, frobenius_distance) = consensus(&counters);
    let headline = headline_for(&verdict);
    let mut notes = base_notes(&arithmetic);
    if counters
        .iter()
        .any(|c| c.kind == "wmc" && !c.error.is_empty())
    {
        notes.push(
            "A counter that reports an error did not finish inside its node or width limit. The verdict uses the counters that finished."
                .into(),
        );
    }
    if verdict == "inconclusive" {
        notes.push(
            "The ball around the diagonal sum still contains the equivalence threshold. Raise the working precision, or simplify the circuit with reduction first."
                .into(),
        );
    }
    if counters
        .iter()
        .any(|c| (c.name == "gpmc" || c.name == "ganak") && c.error.is_empty())
    {
        notes.push(
            "The external floating-point count uses a heuristic 1e-8 slack, not a certified accumulated-error bound; it does not establish a proof."
                .into(),
        );
    }
    Ok(Report {
        verdict,
        headline,
        detail: phase.clone(),
        phase,
        arithmetic,
        qubits: left.qubits as u32,
        gates1: left.len() as u32,
        gates2: right.len() as u32,
        path_variables_before: before,
        path_variables_after: after,
        monomials: ps.phase.terms.len() as u32,
        variables: enc.nvars,
        clauses: enc.clauses.len() as u32,
        frobenius_distance,
        precision_bits: prec,
        reduction_steps: ps.steps.clone(),
        counters,
        notes,
    })
}

fn run_named(name: &str, ps: &PathSum, enc: &Encoded, prec: u32, max_nodes: u64) -> CounterReport {
    let started = Instant::now();
    if name == "gpmc" || name == "ganak" {
        let result = (if name == "gpmc" {
            crate::gpmc::weighted_count(enc, prec)
        } else {
            crate::ganak::weighted_count(enc, prec)
        })
        .map(|wmc| finalize(ps, AmpValue::Ball(wmc), prec));
        let mut report = finish_counter(name, "wmc", started, result);
        // These integrations use a heuristic floating-point slack, not a
        // certified bound on the external program's accumulated error.
        if report.error.is_empty() {
            report.verdict = "inconclusive".into();
            report.frobenius_distance = None;
        }
        return report;
    }
    let result = if selectors_exact(enc) {
        match wmc::problem_exact(enc) {
            Some(problem) => dispatch_exact(name, &problem, max_nodes)
                .map(|wmc| finalize(ps, AmpValue::Exact(wmc), prec)),
            None => dispatch_ball(name, &wmc::problem_ball(enc, prec), max_nodes)
                .map(|wmc| finalize(ps, AmpValue::Ball(wmc), prec)),
        }
    } else {
        dispatch_ball(name, &wmc::problem_ball(enc, prec), max_nodes)
            .map(|wmc| finalize(ps, AmpValue::Ball(wmc), prec))
    };
    finish_counter(name, "wmc", started, result)
}

fn dispatch_exact(
    name: &str,
    problem: &Problem<ExactAmp>,
    max_nodes: u64,
) -> Result<ExactAmp, EqError> {
    match name {
        "enumerate" => wmc::enumerate(problem, max_nodes),
        "dpll" => wmc::dpll(problem, max_nodes),
        "bucket" => wmc::bucket(problem, 18),
        other => Err(EqError(format!("unknown counter `{other}`"))),
    }
}

fn dispatch_ball(
    name: &str,
    problem: &Problem<ComplexBall>,
    max_nodes: u64,
) -> Result<ComplexBall, EqError> {
    match name {
        "enumerate" => wmc::enumerate(problem, max_nodes),
        "dpll" => wmc::dpll(problem, max_nodes),
        "bucket" => wmc::bucket(problem, 16),
        other => Err(EqError(format!("unknown counter `{other}`"))),
    }
}

fn run_direct(ps: &PathSum, prec: u32) -> CounterReport {
    let started = Instant::now();
    let result = ps
        .diagonal_sum_direct(prec)
        .map(|amp| judge(&amp, ps.qubits as u32, prec));
    finish_counter("direct", "reference", started, result)
}

fn run_matrix(left: &Circuit, right: &Circuit, prec: u32) -> CounterReport {
    let started = Instant::now();
    let result = matrix_diagonal(left, right, prec)
        .map(|amp| judge(&AmpValue::Ball(amp), left.qubits as u32, prec));
    finish_counter("matrix", "reference", started, result)
}

struct Judgement {
    verdict: String,
    value: String,
    magnitude_squared: String,
    frobenius_distance: Option<DistanceBounds>,
}

fn finalize(ps: &PathSum, wmc_value: AmpValue, prec: u32) -> Judgement {
    let amp = normalize(ps, wmc_value, prec);
    judge(&amp, ps.qubits as u32, prec)
}

fn normalize(ps: &PathSum, wmc_value: AmpValue, prec: u32) -> AmpValue {
    let constant = ps.phase.constant();
    match wmc_value {
        AmpValue::Exact(wmc) if constant.is_quarter_turn() => {
            let phase =
                ExactAmp::exp_i_pi_quarters(&constant.quarter_index().expect("quarter-turn angle"));
            let amp = ps
                .prefactor
                .mul(&phase)
                .mul(&wmc)
                .div_sqrt2_pow(ps.path_var_count());
            AmpValue::Exact(amp)
        }
        AmpValue::Exact(wmc) => {
            let ball = wmc
                .to_ball(prec)
                .mul(&ps.prefactor.to_ball(prec))
                .mul(&ComplexBall::exp_i(&angle_ball(&constant, prec)))
                .div_sqrt2_pow(ps.path_var_count());
            AmpValue::Ball(ball)
        }
        AmpValue::Ball(wmc) => {
            let ball = wmc
                .mul(&ps.prefactor.to_ball(prec))
                .mul(&ComplexBall::exp_i(&angle_ball(&constant, prec)))
                .div_sqrt2_pow(ps.path_var_count());
            AmpValue::Ball(ball)
        }
    }
}

fn judge(amp: &AmpValue, n: u32, prec: u32) -> Judgement {
    match amp {
        AmpValue::Exact(z) => {
            let ratio = z.div_pow2(n);
            let (c0, c1) = ratio.norm_sqr();
            let magnitude_squared = format!(
                "{} + {}√2",
                crate::exact::fmt_rat(&c0),
                crate::exact::fmt_rat(&c1)
            );
            let value = ratio.pretty();
            let verdict = if c1.is_zero() && c0.is_one() {
                if ratio.is_one() {
                    "proven_equivalent"
                } else {
                    "proven_global_phase"
                }
            } else {
                "proven_not_equivalent"
            };
            let frobenius_distance = if c1.is_zero() && c0.is_one() {
                Some(DistanceBounds::zero())
            } else {
                DistanceBounds::from_normalized_trace(&ratio.to_ball(prec), prec)
            };
            Judgement {
                verdict: verdict.into(), value, magnitude_squared, frobenius_distance,
            }
        }
        AmpValue::Ball(z) => {
            let mag2 = z.norm_sqr();
            let target = RealBall::pow2(2 * n, z.re.prec().max(32));
            let ratio = z.div_pow2(n);
            let value = ratio.pretty();
            let magnitude_squared = ratio.norm_sqr().pretty();
            let verdict = if mag2.certainly_lt(&target) || mag2.certainly_gt(&target) {
                "proven_not_equivalent"
            } else {
                // Overlap, however narrow, does not prove exact equality.
                "inconclusive"
            };
            Judgement {
                verdict: verdict.into(), value, magnitude_squared,
                frobenius_distance: DistanceBounds::from_normalized_trace(&ratio, prec),
            }
        }
    }
}

fn consensus(counters: &[CounterReport]) -> (String, String, String, Option<DistanceBounds>) {
    let decided: Vec<&CounterReport> = counters
        .iter()
        .filter(|c| c.error.is_empty() && c.kind == "wmc")
        .collect();
    if decided.is_empty() {
        if let Some(c) = counters.iter().find(|c| c.error.is_empty()) {
            return (c.verdict.clone(), c.value.clone(), arithmetic_from(c), c.frobenius_distance.clone());
        }
        if let Some(c) = counters.iter().find(|c| !c.error.is_empty()) {
            return ("error".into(), c.error.clone(), "unavailable".into(), None);
        }
        return (
            "inconclusive".into(),
            "no counter finished".into(),
            "unavailable".into(), None,
        );
    }
    let verdict = decided[0].verdict.clone();
    let agree = decided.iter().all(|c| c.verdict == verdict);
    let references_ok = counters
        .iter()
        .filter(|c| c.kind == "reference" && c.error.is_empty())
        .all(|c| compatible(&verdict, &c.verdict));
    let phase = decided[0].value.clone();
    let arithmetic = arithmetic_from(decided[0]);
    if !agree || !references_ok {
        return (
            "inconclusive".into(),
            format!("{phase} The finished procedures do not agree."),
            arithmetic, None,
        );
    }
    (verdict, phase, arithmetic, decided[0].frobenius_distance.clone())
}

fn compatible(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let equiv = |s: &str| {
        matches!(
            s,
            "proven_equivalent" | "proven_global_phase"
        )
    };
    let not = |s: &str| s == "proven_not_equivalent";
    if equiv(a) && equiv(b) {
        return true;
    }
    if not(a) && not(b) {
        return true;
    }
    if a == "inconclusive" || b == "inconclusive" {
        return true;
    }
    false
}

fn arithmetic_from(counter: &CounterReport) -> String {
    if counter.magnitude_squared.contains('±')
        || counter.verdict == "inconclusive"
    {
        "ball".into()
    } else {
        "exact".into()
    }
}

fn finish_counter(
    name: &str,
    kind: &str,
    started: Instant,
    result: Result<Judgement, EqError>,
) -> CounterReport {
    let elapsed_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok(j) => CounterReport {
            name: name.into(),
            kind: kind.into(),
            verdict: j.verdict,
            value: j.value,
            magnitude_squared: j.magnitude_squared,
            frobenius_distance: j.frobenius_distance,
            elapsed_ms,
            error: String::new(),
        },
        Err(err) => CounterReport {
            name: name.into(),
            kind: kind.into(),
            verdict: "error".into(),
            value: String::new(),
            magnitude_squared: String::new(),
            frobenius_distance: None,
            elapsed_ms,
            error: err.0,
        },
    }
}

fn selectors_exact(enc: &Encoded) -> bool {
    enc.angles.iter().flatten().all(Angle::is_quarter_turn)
}

fn counter_names(counter: &str) -> Result<Vec<String>, EqError> {
    match counter {
        "all" | "" => Ok(vec!["enumerate".into(), "dpll".into(), "bucket".into()]),
        "enumerate" | "dpll" | "bucket" | "gpmc" | "ganak" => Ok(vec![counter.into()]),
        other => Err(EqError(format!(
            "unknown counter `{other}`; use all, enumerate, dpll, bucket, gpmc, or ganak"
        ))),
    }
}

fn parse_mode(mode: &str) -> Result<Mode, EqError> {
    match mode {
        "hybrid" | "" => Ok(Mode::Hybrid),
        "reduction" | "rr" => Ok(Mode::Reduction),
        "wmc" => Ok(Mode::Wmc),
        other => Err(EqError(format!(
            "unknown mode `{other}`; use hybrid, reduction, or wmc"
        ))),
    }
}

fn headline_for(verdict: &str) -> String {
    match verdict {
        "proven_equivalent" => "The circuits are equivalent.".into(),
        "proven_global_phase" => "The circuits are equivalent up to a global phase.".into(),
        "proven_not_equivalent" => {
            "The circuits are not equivalent, even up to a global phase.".into()
        }
        "unknown" => "Reduction did not reach an identity. No model counter was run.".into(),
        "error" => "The selected engine did not finish.".into(),
        _ => "The enclosure does not establish exact equivalence or inequivalence.".into(),
    }
}

fn base_notes(arithmetic: &str) -> Vec<String> {
    let mut notes = vec![
        "Huang, Chareton, Chen, Chung, Hsieh, Laarman, and Mei. Equivalence Checking of Quantum Circuits via Path-Sum and Weighted Model Counting. arXiv:2604.24504.".into(),
    ];
    if arithmetic == "exact" {
        notes.push(
            "Every weight is a power of exp(iπ/4). The diagonal sum is an element of Q(i, √2), with no rounding.".into(),
        );
    } else if arithmetic == "ball" {
        notes.push(
            "At least one angle is not a multiple of π/4. That weight is a complex ball from a Taylor enclosure of sine and cosine, and π is enclosed by Machin's formula.".into(),
        );
    }
    notes
}

fn skeleton(left: &Circuit, right: &Circuit, ps: &PathSum, before: u32, after: u32) -> Report {
    Report {
        verdict: String::new(),
        headline: String::new(),
        detail: String::new(),
        phase: String::new(),
        arithmetic: "exact".into(),
        qubits: left.qubits as u32,
        gates1: left.len() as u32,
        gates2: right.len() as u32,
        path_variables_before: before,
        path_variables_after: after,
        monomials: ps.phase.terms.len() as u32,
        variables: 0,
        clauses: 0,
        frobenius_distance: None,
        precision_bits: 0,
        reduction_steps: ps.steps.clone(),
        counters: Vec::new(),
        notes: base_notes("exact"),
    }
}

fn symbolic_report(
    left: &Circuit,
    right: &Circuit,
    ps: &PathSum,
    before: u32,
    after: u32,
    angle: &Angle,
) -> Report {
    let mut report = skeleton(left, right, ps, before, after);
    if angle.is_multiple_of_two_pi() {
        report.verdict = "proven_equivalent".into();
        report.phase = "1".into();
    } else {
        report.verdict = "proven_global_phase".into();
        report.phase = angle.pretty();
    }
    report.frobenius_distance = Some(DistanceBounds::zero());
    report.headline = headline_for(&report.verdict);
    report.detail = format!(
        "Path-sum reduction reached the identity with global phase {}.",
        report.phase
    );
    report.notes.push(
        "The [HH] and [ω] rules were enough. Weighted model counting was not required.".into(),
    );
    report
}

fn unknown_report(
    left: &Circuit,
    right: &Circuit,
    ps: &PathSum,
    before: u32,
    after: u32,
) -> Report {
    let mut report = skeleton(left, right, ps, before, after);
    report.verdict = "unknown".into();
    report.headline = headline_for(&report.verdict);
    report.detail = "The residual path-sum is not syntactically an identity. Run hybrid or wmc mode to decide it.".into();
    report.arithmetic = "symbolic".into();
    report
}

fn dimension_report(left: &Circuit, right: &Circuit) -> Report {
    Report {
        verdict: "proven_not_equivalent".into(),
        headline: "The circuits are not equivalent, even up to a global phase.".into(),
        detail: format!(
            "They act on {} and {} qubits, so their unitaries have different dimensions.",
            left.qubits, right.qubits
        ),
        phase: String::new(),
        arithmetic: "exact".into(),
        qubits: left.qubits as u32,
        gates1: left.len() as u32,
        gates2: right.len() as u32,
        path_variables_before: 0,
        path_variables_after: 0,
        monomials: 0,
        variables: 0,
        clauses: 0,
        frobenius_distance: None,
        precision_bits: 0,
        reduction_steps: Vec::new(),
        counters: Vec::new(),
        notes: base_notes("exact"),
    }
}

// Browser-worker bridge. The request is revalidated and its CNF reconstructed
// at finalization; a caller cannot inject a radius or a normalization factor.
pub fn prepare_external(req:&CheckRequest)->Result<serde_json::Value,EqError> {
    let (left,right,ps,before,after,prec)=external_setup(req)?;
    if left.qubits!=right.qubits { return Ok(serde_json::json!({"report":dimension_report(&left,&right)})); }
    let mode=parse_mode(&req.mode)?;
    if mode!=Mode::Wmc {
        if let Some(angle)=syntactic_global_phase(&ps) {
            return Ok(serde_json::json!({"report":symbolic_report(&left,&right,&ps,before,after,&angle)}));
        }
    }
    if mode==Mode::Reduction { return Ok(serde_json::json!({"report":unknown_report(&left,&right,&ps,before,after)})); }
    let enc=encode::encode(&ps)?;
    let (cnfs,_)=crate::external::cnf(&enc,prec,&req.counter);
    Ok(serde_json::json!({"cnfs":cnfs,"precision":prec,"engine":req.counter}))
}
fn external_setup(req:&CheckRequest)->Result<(Circuit,Circuit,PathSum,u32,u32,u32),EqError> {
    if req.counter!="gpmc" && req.counter!="ganak" { return Err(EqError("Expected GPMC or GANAK.".into())); }
    let left=parse_qasm(&req.circuit1)?;
    let right=parse_qasm(&req.circuit2)?;
    if left.qubits!=right.qubits {
        let ps=PathSum::compose(&left,&left,true)?;
        return Ok((left,right,ps,0,0,req.precision.clamp(32,4096)));
    }
    let mode=parse_mode(&req.mode)?;
    let mut ps=PathSum::compose(&left,&right,mode!=Mode::Wmc)?;
    let before=ps.path_var_count();
    if mode!=Mode::Wmc { reduce(&mut ps)?; }
    let after=ps.path_var_count();
    Ok((left,right,ps,before,after,req.precision.clamp(32,4096)))
}
pub fn finish_external(req:&CheckRequest,stdout:&str,elapsed_ms:u64)->Result<Report,EqError> {
    let (left,right,ps,before,after,prec)=external_setup(req)?;
    if left.qubits!=right.qubits { return Ok(dimension_report(&left,&right)); }
    let enc=encode::encode(&ps)?;
    let (jobs,radius)=crate::external::cnf(&enc,prec,&req.counter);
    let outputs:Vec<String>=serde_json::from_str(stdout).map_err(|_|EqError("Invalid counter result envelope.".into()))?;
    if outputs.len()!=jobs.len() {return Err(EqError("Incomplete algebraic counter jobs.".into()))}
    let values:Vec<_>=outputs.iter().map(|s|crate::external::count(s)).collect::<Result<_,_>>()?;
    let mid=ExactAmp {re0:values[0].0.clone(),im0:values[0].1.clone(),re1:values.get(1).map(|v|v.0.clone()).unwrap_or_default(),im1:values.get(1).map(|v|v.1.clone()).unwrap_or_default()};
    let value=if radius.is_zero() {AmpValue::Exact(mid)} else {
        let b=mid.to_ball(prec);
        let (rl,rh)=b.re.rational_bounds();let (il,ih)=b.im.rational_bounds();
        AmpValue::Ball(ComplexBall {re:RealBall::span_rationals(&(rl-&radius),&(rh+&radius),prec),im:RealBall::span_rationals(&(il-&radius),&(ih+&radius),prec)})
    };
    let j=finalize(&ps,value,prec);
    // An exact counter result that cannot intersect the physically possible
    // normalized trace is an adapter/counter failure, not a valid distance.
    if j.frobenius_distance.is_none() { return Err(EqError("The counter result violates the unitary trace bound.".into())); }
    let counter=CounterReport { name:req.counter.clone(),kind:"wmc".into(),verdict:j.verdict.clone(),value:j.value.clone(),magnitude_squared:j.magnitude_squared,frobenius_distance:j.frobenius_distance.clone(),elapsed_ms,error:String::new() };
    Ok(Report {
        verdict:j.verdict.clone(),headline:headline_for(&j.verdict),detail:j.value.clone(),phase:j.value,
        arithmetic:if radius.is_zero() {"exact".into()} else {"ball".into()},
        qubits:left.qubits as u32,gates1:left.len() as u32,gates2:right.len() as u32,
        path_variables_before:before,path_variables_after:after,monomials:ps.phase.terms.len() as u32,
        variables:enc.nvars,clauses:enc.clauses.len() as u32,frobenius_distance:j.frobenius_distance,
        precision_bits:prec,reduction_steps:ps.steps,counters:vec![counter],
        notes:vec!["The selected WebAssembly counter computed exact complex-rational midpoint sums; a parity/carry extension preserves pi/4 weights exactly in Q(i,sqrt(2)). Ball arithmetic encloses all phase-weight approximation error using a telescoping product bound; no floating-point acceptance band is used.".into()],
    })
}
