> TIMING QUALIFICATION — 2026-11-15: a later real-browser audit found that short-lived asynchronous Chromium teardown can overlap a following sample when the same browser is reused. This historical screen used that protocol, so its numerical/interval evidence and raw records remain useful, but its speed ratios must not be treated as controlled performance claims. The fixed diagnostic subset was remeasured with fresh-browser isolation in [run43625065429](https://github.com/guluchen/PathsumWMC/actions/runs/43625065429); its [complete corrected results](https://github.com/guluchen/PathsumWMC/releases/tag/exact-isolated-pilot-20261115) do not replace the entire historical corpus.

# Original QuPRS paired comparison

Coverage: 440/440 cells, 1416 trials.

QuPRS DECIDED is its historical numerical classification, not a certified distance interval. Our BOUND is a finite inclusive interval, not automatically an exact-equivalence decision. Each cell has one screening trial or a conditional five-repeat median; timeout/UNKNOWN cells receive no speed ratio. Request times exclude Python imports/native frontend startup and include browser worker/module initialization. Whole-process times are separately retained.

| Cohort / mode / system | Cells | Decided / finite bound | Tight bounds ≤ 1e-6 | Timeout | Error / memory | Unknown | Browser crash |
|---|---:|---:|---:|---:|---:|---:|---:|
| MQTBench / hybrid / quprs-default | 37 | 31 | n/a | 4 | 2 | 0 | 0 |
| MQTBench / hybrid / quprs-proportional | 37 | 31 | n/a | 3 | 3 | 0 | 0 |
| MQTBench / hybrid / ours-native | 37 | 29 | 27 | 8 | 0 | 0 | 0 |
| MQTBench / hybrid / ours-wasm | 37 | 25 | 24 | 3 | 9 | 0 | 0 |
| MQTBench / wmc / quprs-default | 37 | 29 | n/a | 5 | 3 | 0 | 0 |
| MQTBench / wmc / quprs-proportional | 37 | 29 | n/a | 6 | 2 | 0 | 0 |
| MQTBench / wmc / ours-native | 37 | 22 | 15 | 15 | 0 | 0 | 0 |
| MQTBench / wmc / ours-wasm | 37 | 21 | 14 | 10 | 6 | 0 | 0 |
| Feynman / reduction / quprs-default | 36 | 30 | n/a | 1 | 5 | 0 | 0 |
| Feynman / reduction / quprs-proportional | 36 | 31 | n/a | 2 | 3 | 0 | 0 |
| Feynman / reduction / ours-native | 36 | 20 | 20 | 7 | 0 | 9 | 0 |
| Feynman / reduction / ours-wasm | 36 | 20 | 20 | 7 | 0 | 9 | 0 |

## Paired request-time ratios

These are descriptive ratios of per-cell request times on completed matching cells; they mix single trials and conditional five-repeat medians. The selected subset requires an original request ≥ 0.1 s and our interval width ≤ 1e-6. Different numerical guarantees and memory ceilings remain. A ratio below 1 means our request completed faster; no paper-hardware or proof-equivalence claim follows.

| Mode / original profile / our system | Selected completed pairs | Median ours/original | Range |
|---|---:|---:|---:|
| hybrid / quprs-default / ours-native | 22 | 3.349 | 0.001754–132.4 |
| hybrid / quprs-default / ours-wasm | 19 | 7.111 | 0.1233–98.48 |
| hybrid / quprs-proportional / ours-native | 21 | 4.14 | 0.001888–108.9 |
| hybrid / quprs-proportional / ours-wasm | 18 | 7.933 | 0.1327–63.88 |
| wmc / quprs-default / ours-native | 14 | 0.7489 | 0.02508–9.693 |
| wmc / quprs-default / ours-wasm | 13 | 3.217 | 0.3966–41.82 |
| wmc / quprs-proportional / ours-native | 12 | 0.9555 | 0.02588–10.47 |
| wmc / quprs-proportional / ours-wasm | 11 | 3.902 | 0.432–45.18 |
| reduction / quprs-default / ours-native | 12 | 0.05163 | 0.03284–0.2481 |
| reduction / quprs-default / ours-wasm | 12 | 0.3531 | 0.1362–0.6066 |
| reduction / quprs-proportional / ours-native | 11 | 0.0693 | 0.03213–1.031 |
| reduction / quprs-proportional / ours-wasm | 11 | 0.3341 | 0.2593–1.609 |

Numerical acceptance / strict-distance differences: 64 rows (including multiple modes/profiles/systems for one input). See results.json; these are retained as semantic differences, not silently counted as correctness agreement.
