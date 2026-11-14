# Exact candidate: isolated diagnostic outcome

The corrected, fixed 12-case panel completed **48/48 cells and 160 trials** in [run43625065429](https://github.com/guluchen/PathsumWMC/actions/runs/43625065429). All native/original profiles run with no open benchmark browser; each WASM trial uses a fresh browser that is closed before the next sample. Previous reused-browser timing ratios are superseded and never pooled. This is a preselected diagnostic subset, not a whole-suite speed claim.

The exact candidate retains balanced input/output gate actions, immutable cached preparation and exact dyadic GMP complex arithmetic with rational fallback (functional d563342/670b985, frozen validation43613464526). Its existing validation remains 49 Rust tests,26000 arithmetic comparisons,96 weighted-CNF browser oracles,8 forced external distance cases, and Stop/mobile checks. No arithmetic or solver change was made for this remeasurement.

For the nine selected MQT Hybrid inputs, original default/proportional each returned seven numerical decisions; our native returned seven finite tight intervals and our WASM five. All three Feynman reduction inputs completed in all four profiles. There were no missing cells, inconsistent repetitions, or native/WASM/cross-mode interval conflicts. Numerical tolerance acceptance and a strict distance interval retain different meanings.

Selected paired Hybrid request-time median ratios versus original default are **3.477x native (6 pairs)** and **8.174x WASM (4 pairs)**. Selection requires original request>=0.1s, completed matching cells and our interval width<=1e-6. Cells have a conditional five-repeat median or a single screening trial; no timeout is imputed. Python import/frontend startup is excluded from these request ratios and retained separately as whole-process time.

Grover5 remains a decisive slow case: our native request13.765s / whole process13.778s, versus original default request0.239s / whole process0.979s; our WASM request25.936s. The catch-up objective is not met by this round. The exact candidate remains experimental and was not adopted into the public PathsumWMC calculation engine.

Per the PI's conditional instruction, the separately attributed [QuPRS browser fork](https://guluchen.github.io/QuPRS/) is complete. It uses current upstream Rustv0.16.0 (0b287366), whereas this diagnostic uses paper-era Python60bcb250. Those are separate baselines. The fork preserves upstream MPFR source weights and numerical classification; its output is not a certified distance interval. Its own [isolated full benchmark](https://github.com/guluchen/QuPRS/releases/tag/browser-benchmark-isolated-20261115) contains220cells/892trials with all24anchors within10%.

[Detailed table](RESULTS.md),[CSV](results.csv),[data](results.json),[provenance](PROVENANCE.json). The public PathsumWMC interval workbench remains available unchanged in its calculation engine.
