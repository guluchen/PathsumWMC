use pathsum_eq::{check, parse_qasm, CheckRequest};

fn qasm(body: &str) -> String {
    format!("OPENQASM 2.0;\nqreg q[8];\n{body}\n")
}

fn run(a: &str, b: &str, mode: &str) -> pathsum_eq::Report {
    let qubits = qubits_of(a).max(qubits_of(b));
    let req = CheckRequest {
        circuit1: format!("OPENQASM 2.0;\nqreg q[{qubits}];\n{a}\n"),
        circuit2: format!("OPENQASM 2.0;\nqreg q[{qubits}];\n{b}\n"),
        mode: mode.into(),
        counter: "all".into(),
        precision: 64,
        max_precision: 128,
        max_nodes: 200_000,
    };
    check(&req).unwrap_or_else(|err| panic!("{err}"))
}

fn qubits_of(body: &str) -> usize {
    let mut max = 1usize;
    for token in body.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
        if let Some(rest) = token.strip_prefix('q') {
            if let Ok(n) = rest.parse::<usize>() {
                max = max.max(n + 1);
            }
        }
    }
    // q[3] is tokenized as q and 3 separately by this scan, so also look at digits after '['.
    let mut n = 1usize;
    for part in body.split('[') {
        if let Some(idx) = part.split(']').next() {
            if let Ok(v) = idx.trim().parse::<usize>() {
                n = n.max(v + 1);
            }
        }
    }
    n.max(max)
}

fn assert_equiv(verdict: &str) {
    assert!(
        matches!(
            verdict,
            "proven_equivalent" | "proven_global_phase"
        ),
        "{verdict}"
    );
}

#[test]
fn hadamards_cancel_by_reduction() {
    let report = run("h q[0];\nh q[0];", "id q[0];", "hybrid");
    assert_eq!(
        report.verdict, "proven_equivalent",
        "{:?}",
        report.reduction_steps
    );
}

#[test]
fn hadamards_agree_across_counters() {
    let report = run("h q[0];\nh q[0];", "id q[0];", "wmc");
    assert_eq!(report.verdict, "proven_equivalent");
    let wmc: Vec<_> = report.counters.iter().filter(|c| c.kind == "wmc").collect();
    assert!(wmc.len() >= 3);
    assert!(wmc.iter().all(|c| c.error.is_empty()), "{wmc:?}");
    assert!(wmc.iter().all(|c| c.verdict == "proven_equivalent"));
}

#[test]
fn cz_matches_hadamard_cnot_sandwich() {
    let report = run("cz q[0],q[1];", "h q[1];\ncx q[0],q[1];\nh q[1];", "hybrid");
    assert_eq!(
        report.verdict, "proven_equivalent",
        "{:?}",
        report.reduction_steps
    );
    let forced = run("cz q[0],q[1];", "h q[1];\ncx q[0],q[1];\nh q[1];", "wmc");
    assert_eq!(forced.verdict, "proven_equivalent", "{:?}", forced.counters);
}

#[test]
fn four_t_gates_are_z() {
    let report = run("t q[0];\nt q[0];\nt q[0];\nt q[0];", "z q[0];", "hybrid");
    assert_eq!(report.verdict, "proven_equivalent");
}

#[test]
fn s_and_qiskit_rz_share_a_global_phase() {
    let report = run("s q[0];", "rz(pi/2) q[0];", "wmc");
    assert_eq!(
        report.verdict, "proven_global_phase",
        "{:?}",
        report.counters
    );
}

#[test]
fn ganak_floating_counts_do_not_claim_proofs() {
    let bundled = "/tmp/ganak/ganak";
    let bin = std::env::var("PATHSUM_GANAK").unwrap_or_else(|_| bundled.to_string());
    if !std::path::Path::new(&bin).is_file() {
        eprintln!("skipping GANAK test; set PATHSUM_GANAK to the ganak binary");
        return;
    }
    std::env::set_var("PATHSUM_GANAK", &bin);
    let same = check(&CheckRequest {
        circuit1: "OPENQASM 2.0;\nqreg q[1];\nh q[0];\n".into(),
        circuit2: "OPENQASM 2.0;\nqreg q[1];\nh q[0];\n".into(),
        mode: "wmc".into(),
        counter: "ganak".into(),
        ..CheckRequest::default()
    })
    .unwrap();
    assert_eq!(same.verdict, "inconclusive", "{}", same.detail);
    let differ = check(&CheckRequest {
        circuit1: "OPENQASM 2.0;\nqreg q[1];\nh q[0];\n".into(),
        circuit2: "OPENQASM 2.0;\nqreg q[1];\n".into(),
        mode: "wmc".into(),
        counter: "ganak".into(),
        ..CheckRequest::default()
    })
    .unwrap();
    assert_eq!(differ.verdict, "inconclusive", "{}", differ.detail);
}

#[test]
fn gpmc_floating_counts_do_not_claim_proofs() {
    let bundled = "/tmp/GPMC/bin/gpmc";
    let bin = std::env::var("PATHSUM_GPMC").unwrap_or_else(|_| bundled.to_string());
    if !std::path::Path::new(&bin).is_file() {
        eprintln!("skipping GPMC test; set PATHSUM_GPMC to the complex-weight binary");
        return;
    }
    std::env::set_var("PATHSUM_GPMC", &bin);
    let same = check(&CheckRequest {
        circuit1: "OPENQASM 2.0;\nqreg q[1];\nh q[0];\n".into(),
        circuit2: "OPENQASM 2.0;\nqreg q[1];\nh q[0];\n".into(),
        mode: "wmc".into(),
        counter: "gpmc".into(),
        ..CheckRequest::default()
    })
    .unwrap();
    assert_eq!(same.verdict, "inconclusive", "{}", same.detail);
    let differ = check(&CheckRequest {
        circuit1: "OPENQASM 2.0;\nqreg q[1];\nh q[0];\n".into(),
        circuit2: "OPENQASM 2.0;\nqreg q[1];\n".into(),
        mode: "wmc".into(),
        counter: "gpmc".into(),
        ..CheckRequest::default()
    })
    .unwrap();
    assert_eq!(differ.verdict, "inconclusive", "{}", differ.detail);
}

#[test]
fn rz_pi_versus_z_is_plus_i() {
    // Rz(π) = −i Z, so Z · Rz(π)† = Z · Rz(−π) = i I.
    let report = run("rz(pi) q[0];", "z q[0];", "wmc");
    assert_eq!(report.verdict, "proven_global_phase", "{}", report.detail);
    assert!(
        report.detail.contains("i(1"),
        "expected phase i, got {}",
        report.detail
    );
}

#[test]
fn ry_rz_matches_hadamard_up_to_phase() {
    // Qiskit: Ry(-π/2) then Rz(-π) equals i H.
    let report = run("h q[0];", "ry(-pi/2) q[0];\nrz(-pi) q[0];", "hybrid");
    assert_eq!(report.verdict, "proven_global_phase", "{}", report.detail);
}

#[test]
fn hadamard_is_not_identity() {
    let report = run("h q[0];", "id q[0];", "wmc");
    assert_eq!(
        report.verdict, "proven_not_equivalent",
        "{:?}",
        report.counters
    );
}

#[test]
fn opposite_rotations_cancel_exactly() {
    let report = run("rx(1/5) q[0];\nrx(-1/5) q[0];", "id q[0];", "hybrid");
    assert_eq!(report.verdict, "proven_equivalent", "{}", report.detail);
}

#[test]
fn nearby_rotations_are_separated_by_balls() {
    let report = run("rx(1/5) q[0];", "rx(1/6) q[0];", "wmc");
    assert_eq!(
        report.verdict, "proven_not_equivalent",
        "{:?} {}",
        report.counters, report.detail
    );
    assert_eq!(report.arithmetic, "ball");
}

#[test]
fn hzh_is_x() {
    let report = run("h q[0];\nz q[0];\nh q[0];", "x q[0];", "reduction");
    assert_eq!(
        report.verdict, "proven_equivalent",
        "{:?}",
        report.reduction_steps
    );
}

#[test]
fn bell_circuit_matches_itself() {
    let body = "h q[0];\ncx q[0],q[1];";
    let report = run(body, body, "hybrid");
    assert_equiv(&report.verdict);
}

#[test]
fn random_clifford_t_matches_direct_evaluation() {
    let mut state = 0x1234_5678u32;
    let mut next = || {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        state
    };
    for _ in 0..12 {
        let n = 1 + (next() as usize % 2);
        let depth = 1 + (next() as usize % 5);
        let mut gates = String::new();
        for _ in 0..depth {
            let q = next() as usize % n;
            let kind = next() % 6;
            let line = match kind {
                0 => format!("h q[{q}];"),
                1 => format!("s q[{q}];"),
                2 => format!("t q[{q}];"),
                3 => format!("x q[{q}];"),
                4 => format!("z q[{q}];"),
                _ if n == 2 => format!("cx q[0],q[1];"),
                _ => format!("s q[{q}];"),
            };
            gates.push_str(&line);
            gates.push('\n');
        }
        let report = run(&gates, &gates, "wmc");
        assert_equiv(&report.verdict);
        let direct = report.counters.iter().find(|c| c.name == "direct");
        if let Some(direct) = direct {
            assert!(direct.error.is_empty(), "{}", direct.error);
            assert_equiv(&direct.verdict);
        }
    }
}

#[test]
fn parser_accepts_the_running_example() {
    let circuit = parse_qasm(&qasm("h q[1];\ncx q[0],q[1];\nh q[1];")).unwrap();
    assert_eq!(circuit.gates.len(), 3);
}

#[test]
fn tiny_phase_must_not_be_certified() {
    // diag(1,exp(i*10^-30)) is not a scalar identity. At 32 bits its
    // amplitude enclosure overlaps the exact-equivalence threshold.
    let report = check(&CheckRequest {
        circuit1: "OPENQASM 2.0; qreg q[1]; id q[0];".into(),
        circuit2: "OPENQASM 2.0; qreg q[1]; p(1/1000000000000000000000000000000) q[0];".into(),
        mode: "wmc".into(),
        counter: "all".into(),
        precision: 32,
        max_precision: 32,
        max_nodes: 200_000,
    }).unwrap();
    assert_eq!(report.verdict, "inconclusive", "{:?}", report.counters);
    assert!(report.counters.iter().all(|c| !matches!(c.verdict.as_str(), "proven_equivalent" | "proven_global_phase" | "certified_equivalent" | "certified_global_phase")));
}

#[test]
fn distance_zero_ignores_global_phase() {
    for (left, right, mode) in [
        ("h q[0]; h q[0];", "id q[0];", "hybrid"),
        ("s q[0];", "rz(pi/2) q[0];", "wmc"),
    ] {
        let report = run(left, right, mode);
        let distance = report.frobenius_distance.expect("distance missing");
        assert_eq!(distance.lower_exact, "0");
        assert_eq!(distance.upper_exact, "0");
        assert!(distance.global_phase_ignored);
    }
}

#[test]
fn general_angle_distance_encloses_analytic_value() {
    // tr(Rz(2*pi/3))/2 = cos(pi/3) = 1/2, so delta = 1.
    let report = run("rz(2*pi/3) q[0];", "id q[0];", "wmc");
    let distance = report.frobenius_distance.expect("distance missing");
    let lo: num_rational::BigRational = distance.lower_exact.parse().unwrap();
    let hi: num_rational::BigRational = distance.upper_exact.parse().unwrap();
    let one = num_rational::BigRational::from_integer(1.into());
    assert!(lo <= one && one <= hi);
}

#[test]
fn tiny_phase_distance_remains_bounded_when_equality_is_unknown() {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    let request = |bits| CheckRequest {
        circuit1: "OPENQASM 2.0; qreg q[1]; id q[0];".into(),
        circuit2: "OPENQASM 2.0; qreg q[1]; p(1/1000000000000000000000000000000) q[0];".into(),
        mode: "wmc".into(), counter: "all".into(), precision: bits,
        max_precision: bits, max_nodes: 200_000,
    };
    let low = check(&request(32)).unwrap();
    let high = check(&request(256)).unwrap();
    let l = low.frobenius_distance.expect("low-precision distance missing");
    let h = high.frobenius_distance.expect("high-precision distance missing");
    let low_hi: BigRational = l.upper_exact.parse().unwrap();
    let high_hi: BigRational = h.upper_exact.parse().unwrap();
    let high_lo: BigRational = h.lower_exact.parse().unwrap();
    assert!(high_hi < low_hi);
    // delta = 2*sin(epsilon/4). For 0<epsilon<1:
    // epsilon/2 - epsilon^3/192 <= delta <= epsilon/2.
    let epsilon = BigRational::new(1.into(), BigInt::from(10).pow(30));
    let analytic_upper = &epsilon / BigRational::from_integer(2.into());
    let analytic_lower = &analytic_upper - &epsilon * &epsilon * &epsilon / BigRational::from_integer(192.into());
    assert!(high_lo <= analytic_upper && high_hi >= analytic_lower);
    assert!(high_lo > BigRational::from_integer(0.into()));
}
