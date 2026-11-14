# Original QuPRS paired comparison — diagnostic pilot

Coverage: 48/48 cells, 152 trials.

QuPRS DECIDED is its historical numerical classification, not a certified distance interval. Our BOUND is a finite inclusive interval, not automatically an exact-equivalence decision. Each cell has one screening trial or a conditional five-repeat median; timeout/UNKNOWN cells receive no speed ratio. Request times exclude Python imports/native frontend startup and include browser worker/module initialization. Whole-process times are separately retained.

| Cohort / mode / system | Cells | Decided / finite bound | Tight bounds ≤ 1e-6 | Timeout | Error / memory | Unknown | Browser crash |
|---|---:|---:|---:|---:|---:|---:|---:|
| MQTBench / hybrid / quprs-default | 9 | 7 | n/a | 1 | 1 | 0 | 0 |
| MQTBench / hybrid / quprs-proportional | 9 | 7 | n/a | 1 | 1 | 0 | 0 |
| MQTBench / hybrid / ours-native | 9 | 7 | 7 | 2 | 0 | 0 | 0 |
| MQTBench / hybrid / ours-wasm | 9 | 5 | 5 | 2 | 2 | 0 | 0 |
| MQTBench / wmc / quprs-default | 0 | 0 | n/a | 0 | 0 | 0 | 0 |
| MQTBench / wmc / quprs-proportional | 0 | 0 | n/a | 0 | 0 | 0 | 0 |
| MQTBench / wmc / ours-native | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| MQTBench / wmc / ours-wasm | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| Feynman / reduction / quprs-default | 3 | 3 | n/a | 0 | 0 | 0 | 0 |
| Feynman / reduction / quprs-proportional | 3 | 3 | n/a | 0 | 0 | 0 | 0 |
| Feynman / reduction / ours-native | 3 | 3 | 3 | 0 | 0 | 0 | 0 |
| Feynman / reduction / ours-wasm | 3 | 3 | 3 | 0 | 0 | 0 | 0 |

## Paired request-time ratios

These are descriptive ratios of per-cell request times on completed matching cells; they mix single trials and conditional five-repeat medians. The selected subset requires an original request ≥ 0.1 s and our interval width ≤ 1e-6. Different numerical guarantees and memory ceilings remain. A ratio below 1 means our request completed faster; no paper-hardware or proof-equivalence claim follows.

| Mode / original profile / our system | Selected completed pairs | Median ours/original | Range |
|---|---:|---:|---:|
| hybrid / quprs-default / ours-native | 6 | 3.492 | 0.1406–63.2 |
| hybrid / quprs-default / ours-wasm | 4 | 8.015 | 0.7049–106 |
| hybrid / quprs-proportional / ours-native | 6 | 3.189 | 0.1484–56.82 |
| hybrid / quprs-proportional / ours-wasm | 4 | 7.985 | 0.7442–86.01 |
| wmc / quprs-default / ours-native | 0 | n/a | n/a |
| wmc / quprs-default / ours-wasm | 0 | n/a | n/a |
| wmc / quprs-proportional / ours-native | 0 | n/a | n/a |
| wmc / quprs-proportional / ours-wasm | 0 | n/a | n/a |
| reduction / quprs-default / ours-native | 3 | 0.0473 | 0.02306–0.07525 |
| reduction / quprs-default / ours-wasm | 3 | 0.2225 | 0.1086–0.4161 |
| reduction / quprs-proportional / ours-native | 3 | 0.0412 | 0.03346–0.06745 |
| reduction / quprs-proportional / ours-wasm | 3 | 0.1938 | 0.1576–0.373 |

Numerical acceptance / strict-distance differences: 14 rows (including multiple modes/profiles/systems for one input). See results.json; these are retained as semantic differences, not silently counted as correctness agreement.
