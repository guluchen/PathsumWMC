use pathsum_eq::{check, CheckRequest};

fn main() {
    let cases = [
        ("hh_identity", "equivalent", "hybrid", "h q[0]; h q[0];", "id q[0];"),
        ("clifford_global_phase", "equivalent", "wmc", "s q[0];", "rz(pi/2) q[0];"),
        ("rational_inverse_hybrid", "equivalent", "hybrid", "rx(1/5) q[0]; rx(-1/5) q[0];", "id q[0];"),
        ("rational_inverse_wmc", "equivalent", "wmc", "rx(1/5) q[0]; rx(-1/5) q[0];", "id q[0];"),
        ("rational_self_wmc", "equivalent", "wmc", "rx(1/5) q[0];", "rx(1/5) q[0];"),
        ("separated_rotations", "not_equivalent", "wmc", "rx(1/5) q[0];", "rx(1/6) q[0];"),
        ("tiny_phase_1e_6", "not_equivalent", "wmc", "id q[0];", "p(1/1000000) q[0];"),
        ("tiny_phase_1e_12", "not_equivalent", "wmc", "id q[0];", "p(1/1000000000000) q[0];"),
        ("tiny_phase_1e_30", "not_equivalent", "wmc", "id q[0];", "p(1/1000000000000000000000000000000) q[0];"),
    ];
    for (name, truth, mode, left, right) in cases {
        for bits in [32, 64, 128, 256, 512] {
            let start = std::time::Instant::now();
            let report = check(&CheckRequest {
                circuit1: format!("OPENQASM 2.0; qreg q[1]; {left}"),
                circuit2: format!("OPENQASM 2.0; qreg q[1]; {right}"),
                mode: mode.into(), counter: "all".into(),
                precision: bits, max_precision: bits, max_nodes: 200_000,
            }).expect("precision probe failed");
            let equivalent = matches!(report.verdict.as_str(), "proven_equivalent" | "proven_global_phase");
            assert!(!(truth == "not_equivalent" && equivalent), "false equality: {name}, {bits}");
            assert!(!(truth == "equivalent" && report.verdict == "proven_not_equivalent"), "false inequality: {name}, {bits}");
            if matches!(name, "hh_identity" | "clifford_global_phase" | "rational_inverse_hybrid") {
                assert!(equivalent, "lost exact/symbolic result: {name}, {bits}");
            }
            println!("{}", serde_json::json!({
                "case": name, "ground_truth": truth, "mode": mode,
                "fixed_precision_bits": bits, "wall_ms": start.elapsed().as_millis(), "report": report,
            }));
        }
    }
}
