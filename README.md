# PathsumWMC

**A reimplementation of our paper, built with Cursor, using ball arithmetic.**

PathsumWMC reimplements our paper, *Equivalence Checking of Quantum Circuits via Path-Sum and Weighted Model Counting* (Huang, Chareton, Chen, Chung, Hsieh, Laarman, and Mei; TACAS 2026; [arXiv:2604.24504](https://arxiv.org/abs/2604.24504)). It checks quantum-circuit equivalence up to a global phase with a Rust library and a browser workbench. Quarter-turn weights use exact arithmetic; general angles use complex ball arithmetic.

- [Browser workbench](https://guluchen.github.io/PathsumWMC/)
- [Source code](https://github.com/guluchen/PathsumWMC)
- [MIT license](LICENSE)
- [Original-paper browser benchmark: results and limitations](benchmarks/paper-2026/results/FINDINGS.md)
- [Direct comparison with original QuPRS: native and WASM](benchmarks/quprs-compare/results/FINDINGS.md)

The decision procedure follows Huang, Chareton, Chen, Chung, Hsieh, Laarman, and Mei, “Equivalence Checking of Quantum Circuits via Path-Sum and Weighted Model Counting” ([arXiv:2604.24504](https://arxiv.org/abs/2604.24504), TACAS 2026). It builds the path-sum of \(U_2 U_1^\dagger\), applies the [HH] and [ω] reductions, and encodes whatever remains as a weighted model counting problem. The circuits are equivalent up to a global phase when the sum of the diagonal amplitudes has magnitude \(2^n\).

Three counters solve that problem inside the library:

- **enumerate** — backtracking over the CNF, pruning conflicts
- **dpll** — unit propagation with activity branching
- **bucket** — variable elimination on clause and weight factors

The browser runs GPMC and GANAK in disposable WebAssembly workers, with Stop and a 200-second whole-request limit. GPMC uses our exact GMP complex-rational numeric adapter on the pinned upstream counting engine. GANAK uses the pinned upstream counting algorithm, rebuilt with static WASM dependencies, in deterministic exact complex-rational mode (`--mode 2 --prob 0`). A parity/carry extension reconstructs pi/4 weights exactly in Q(i, sqrt(2)) using at most two rational counter jobs. The Rust ball adapter propagates all phase-weight approximation error with a telescoping product bound; the external count itself is rational and exact. A nonzero-radius overlap remains inconclusive. Engine revisions, hashes, licenses and complete corresponding-source archives are linked from the site's About section.

The native optional `--counter gpmc` / `--counter ganak` subprocess integrations still use the earlier floating-point diagnostic path. They do not certify an accumulated rounding-error bound or provide a certified distance. The new exact-rational worker adapter is a separate explicit API (`prepare_external` / `finish_external`).

A direct path enumeration and, for at most three qubits, a state-vector product are reference checks. If a weighted counter and a reference disagree, the verdict is inconclusive.

## Frobenius distance

The main result is an inclusive distance interval. For n qubits and V = U₂U₁†,

```text
δ = minφ ||U₂ − exp(iφ)U₁||F / sqrt(2^n)
  = sqrt(2 − 2 |tr(V)| / 2^n).
```

This is the normalized Frobenius distance, ignoring global phase. Its range is 0 to sqrt(2). A bound such as `[0, 1e-12]` establishes closeness within this metric; it does not by itself establish exact equality. Increasing precision can tighten the interval even when an exact-equivalence verdict remains inconclusive. This is an average matrix distance, not the worst-case input error.

Display endpoints are rounded outward to six significant decimal digits. Reports also contain exact rational endpoints in `frobenius_distance.lower_exact` and `upper_exact`. Symbolic proofs and exact global-phase equivalence produce distance zero. No distance is reported for mismatched dimensions, inconsistent counter evidence, or heuristic external counts.

## Exact values and ball arithmetic

Angles are rational. `pi/2` and `0.3*pi` are exact (`0.3` is `3/10`). When every weight is a multiple of \(\pi/4\), the amplitude lives in \(\mathbb{Q}(i, \sqrt{2})\) and the decision is exact.

Any other angle is enclosed by a complex ball. \(\pi\) comes from Machin’s formula, and sine and cosine are Taylor polynomials with an explicit remainder. A ball that is disjoint from the equivalence threshold proves the circuits differ. An interval that still contains the threshold is inconclusive, however narrow it is. The checker raises the working precision until the maximum you set. More precision can separate close but unequal circuits; interval overlap alone cannot prove exact equality. Symbolic reduction or exact arithmetic can still establish equivalence. A ball with nonzero radius is never treated as an exact proof.

Qiskit’s `rz(θ)` is \(\mathrm{diag}(e^{-i\theta/2}, e^{i\theta/2})\). It differs from `p(θ)` by the global phase \(e^{-i\theta/2}\).

## Supported gates

`id`, `h`, `x`, `y`, `z`, `s`, `sdg`, `t`, `tdg`, `cx` / `cnot`, `cz`, `swap`, `ccx` / `toffoli`, `ccz`, `rx`, `ry`, `rz`, `p` / `u1` / `phase`, `crx`, `cry`, `crz`, `cp` / `cphase` / `cu1`.

OpenQASM 2 `qreg` declarations are required. Measurement, reset, and custom `gate` definitions are rejected.

## Command line

```bash
cargo run -p pathsum-eq -- examples/cz.qasm examples/hcxh.qasm
cargo run -p pathsum-eq -- left.qasm right.qasm --mode hybrid --counter all --precision 72
```

`--mode` is `hybrid` (default), `reduction`, or `wmc`. `--counter` is `all`, `enumerate`, `dpll`, `bucket`, `gpmc`, or `ganak`. The program prints a JSON report.

Rust 1.76 or newer is enough. Tests:

```bash
cargo test --manifest-path crates/pathsum-eq/Cargo.toml
```

## Browser

The site is a static Vite app. The checker is WebAssembly, so nothing is sent to a server.

```bash
# once: Rust target and the bindgen CLI that matches the crate
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --locked --version 0.2.100

cd web
npm install
npm run build:wasm
npm run dev
```

The dev server listens on port 4317. `npm run build` writes `web/dist` with a relative base, so the same files work at a domain root or under `/PathsumWMC/`. GitHub Actions rebuilds the WASM from Rust, runs the Rust and browser checks, and deploys only after they pass. Generated WASM bindings are not checked into source.

## Layout

- `crates/pathsum-eq` — path-sums, reductions, encoding, the three counters, exact arithmetic, and balls
- `crates/pathsum-wasm` — the `check_equivalence` export
- `web` — the English workbench
- `examples` — the CZ versus H–CX–H pair from the paper

## Precision experiment

`cargo run --release -p pathsum-eq --example precision_probe` writes JSONL for 32, 64, 128, 256, and 512 bits, with automatic escalation disabled in each cell. Cases include known equivalent circuits, forced WMC on general-angle identities, and phase differences of 10^-6, 10^-12, and 10^-30 radians. CI preserves complete reports as the `validation` artifact. Timing repetitions are not proofs; a finite-precision overlap remains inconclusive.

## License

Our implementation is licensed under the [MIT License](LICENSE). Dependencies and bundled fonts retain their own licenses; the deployed site includes third-party license notices.
