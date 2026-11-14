# Paper-artifact browser benchmark

Original paper-era artifact snapshot; 37 MQT paired inputs and 36 Feynman paired inputs. MQT runs Hybrid and WMC with both external WASM engines; Feynman runs reduction-only as in the paper RR comparison. The full artifact is a superset of the printed table. No source angle normalization or regenerated fixtures.

200 s whole-request screening; four additional repetitions only for first finite bounds under 2 s. Single trials are not medians. Local-HTTP Chromium CI timings; not comparable as speedups to the paper hardware. BOUND means a finite certified interval, not an exact-equivalence decision. Wide intervals are flagged separately.

Coverage: 184/184 cells, 456 total trials. Interval disagreements: 0.

| Cohort / mode / engine | Cells | Finite bounds | Tight bounds (width ≤ 1e-6) | Timeout | Error | Browser crash | Unknown |
|---|---:|---:|---:|---:|---:|---:|---:|
| MQTBench / hybrid / gpmc | 37 | 26 | 24 | 2 | 9 | 0 | 0 |
| MQTBench / hybrid / ganak | 37 | 26 | 25 | 11 | 0 | 0 | 0 |
| MQTBench / wmc / gpmc | 37 | 21 | 14 | 10 | 6 | 0 | 0 |
| MQTBench / wmc / ganak | 37 | 23 | 15 | 14 | 0 | 0 | 0 |
| Feynman / reduction / dpll | 36 | 20 | 20 | 7 | 0 | 0 | 9 |
