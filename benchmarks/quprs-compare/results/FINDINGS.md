> TIMING QUALIFICATION — 2026-11-15: a later real-browser audit found that short-lived asynchronous Chromium teardown can overlap a following sample when the same browser is reused. This historical screen used that protocol, so its numerical/interval evidence and raw records remain useful, but its speed ratios must not be treated as controlled performance claims. A fixed diagnostic subset is being remeasured with fresh-browser isolation in [run43625065429](https://github.com/guluchen/PathsumWMC/actions/runs/43625065429); this does not replace the entire historical corpus.

# Direct comparison with the original QuPRS

**The current Hybrid reimplementation has a substantial performance problem compared with the paper-era original.** This is not solely a WebAssembly problem: the certified native path is also slower on many Hybrid inputs, and both our implementations complete fewer cases. Reduction-only is faster on the selected common completed cases, but has lower completion coverage. Pure WMC native timing is competitive on a small selected subset; that does not compensate for its missing cases or imply equivalent numerical guarantees.

## Complete experiment

The [canonical same-host run](https://github.com/guluchen/PathsumWMC/actions/runs/43608139119) completed **440/440 cells and 1,416 trials**, including 244 conditional five-repeat cells and 196 single-trial cells. All 24 timing shards succeeded as measurements; this does not mean every solver request succeeded. There were no missing/duplicate/incomplete trials, no unstable statuses or verdicts, no conflicting native/WASM/repeat/mode distance intervals and no browser crashes. Frozen source/binary hashes agree across all 24 shards. The preparation gate passed 45 Rust tests and 24 paired API/browser smoke cells; eight reporting regressions also passed.

Original QuPRS is the **Python backend at `60bcb250` (2026-03-19)**, with its original MPFR GPMC submodule `a1bda348`. This is the last upstream revision before the paper's arXiv submission, not current QuPRS's newer Rust backend. Its default Difference and alternative Proportional strategies are separate fixed profiles. Input direction is unchanged; no per-case strategy/direction tuning is claimed. Our native frontend uses the certified exact-rational external API, not the old floating diagnostic CLI. Our GPMC WASM site remains the unchanged frozen build `802195f`.

All original/native subprocesses have a 4 GiB address-space cap and 16 MiB stack; WASM keeps the deployed 512 MiB/module ceiling. Requests have a 200-second outer cap. Configurations of one input run serially on the same pinned CPU, with rotated order. CPU models vary between shards. API/request time excludes Python imports and native frontend startup; browser requests include fresh worker/module initialization but exclude Chromium startup and WAN download. Whole-process time is retained separately. Native request timing is measured inside the example before printing its JSON result. The numerical contracts and memory ceilings differ, so these are implementation comparisons rather than equal-guarantee, equal-memory algorithm rankings.

## Coverage

The 110 input/mode combinations consist of 37 MQTBench pairs in each of Hybrid and WMC, and 36 Feynman pairs in reduction-only. Four fixed configurations give 440 cells.

| Configuration | Numerical decision / finite distance | Narrow distance interval (width ≤ 1e-6) | Timeout | Error / reported memory limit | Unknown |
|---|---:|---:|---:|---:|---:|
| QuPRS Difference default | 90/110 numerical decisions | n/a | 10 | 10 | 0 |
| QuPRS Proportional | 91/110 numerical decisions | n/a | 11 | 8 | 0 |
| Ours native | 71/110 finite intervals | 62/110 | 30 | 0 | 9 |
| Ours WASM | 66/110 finite intervals | 58/110 | 20 | 15 | 9 |

A finite interval is not an equivalence proof. Of our native finite results, 43 have a strict decision (19 exact equivalence, 7 global-phase equivalence, 17 non-equivalence) and 28 remain inconclusive. For WASM these counts are 41 strict decisions (19/7/15) and 25 inconclusive. Results from multiple modes on one input are counted separately. The [mode-specific table](RESULTS.md), [all per-cell times and statuses](results.csv) and [full exact intervals](results.json) retain the distinctions.

## Paired API/request times

Ratios below are ours / QuPRS **default Difference**, selected only where both finished with a numerical decision/finite bound, the original API time is at least 0.1 s, and our interval width is at most `1e-6`. They are medians of paired per-cell ratios; they mix single screening trials and conditional five-repeat medians, and are not uniform five-seed speedup estimates. Timeout/UNKNOWN/error cases are never imputed. Selection favors completed cases and must be read together with the coverage table.

| Mode | Native pairs | Native median ratio | WASM pairs | WASM median ratio |
|---|---:|---:|---:|---:|
| Hybrid | 22 | 3.35× | 19 | 7.11× |
| Pure WMC | 14 | 0.75× | 13 | 3.22× |
| Reduction-only | 12 | 0.052× | 12 | 0.353× |

Against the separate Proportional profile, the Hybrid ratios are 4.14× native (21 pairs) and 7.93× WASM (18 pairs). Thus the main Hybrid finding is not an artifact of choosing only the original default strategy. Proportional can have large per-case effects; [RESULTS.md](RESULTS.md) retains all modes/profiles, sample sizes and ranges. No selected median substitutes for cases that only one implementation completes.

Representative Hybrid API/request times in seconds:

| Input | QuPRS default | Ours native | Ours WASM |
|---|---:|---:|---:|
| QFT16 | 1.261 | 7.819 | 29.353 |
| QNN8 | 5.489 | 91.790 | Error at 512 MiB |
| QPE-inexact24 | 0.801 | 106.004 | Error at 512 MiB |

QFT16 and QNN8 are single screening trials for these three configurations. QuPRS's QPE-inexact24 value is a five-repeat median; our native value is one screening trial. This table is illustrative, not a statistically uniform timing panel.

The slow native path contains substantial counter time: QNN8 used 83.711 s in native counter subprocesses out of 91.790 s total, and QPE-inexact24 used 98.019 s out of 106.004 s. On VQE8, native counter subprocesses used 5.275 s out of 18.644 s total, leaving substantial non-counter work too. These observations locate costs, but do not isolate their causes. Encoding/search differences, exact rational arithmetic, reduction strategy and the repeated setup in the prepare/finish API all need profiling before assigning blame or designing an optimization. A faster browser loader alone cannot address these native gaps.

## Numerical and capacity caveats

All 90/91 completed original classifications accepted equivalence or global-phase equivalence under its historical numerical treatment. On **17 distinct MQTBench inputs**, original numerical acceptance coexists with our strictly positive lower distance bound (64 rows across modes/profiles/our systems). This is explicitly preserved in `numerical_verdict_differences` in results.json, not counted as correctness agreement. Qiskit's floating input interpretation, historical tolerance-based reduction and rounded weighted-count classification can differ from strict source-literal ball arithmetic. This screen does not attribute every difference to a specific cause or claim a universal original-tool false-positive rate.

The separate near-phase smoke diagnostic makes the contract difference concrete: an original QASM `u1(0.000001)` compared with identity is accepted by both original profiles, while both our implementations enclose a strictly positive normalized projective Frobenius distance of about `5e-7`. These smoke inputs are not pooled with the 440 paper-artifact cells.

Our WASM had 15 GPMC counter errors, including the known memory-ceiling cases and the still-unclassified Grover5 Hybrid abort. Native had no counter errors but 30 outer timeouts. Original errors/reported memory limits are also retained: several fatal exits report `cannot allocate memory for thread-local data: ABORT` under the 4 GiB process cap. These are resource/error outcomes, not negative equivalence answers. This experiment does not reproduce the paper's 64 GiB server capacity or its hardware timings.

## Evidence and reproduction

- [Methodology and reproduction instructions](../README.md)
- [Full mode/profile tables](RESULTS.md), [CSV](results.csv), [JSON audit and intervals](results.json), [source/binary/environment provenance](PROVENANCE.json)
- [Permanent raw JSONL, metadata, smoke diagnostics and checksums](https://github.com/guluchen/PathsumWMC/releases/tag/quprs-comparison-20261114)
- [Original paired-input manifest and SHA256 hashes](../../paper-2026/manifest.json)

This is a fresh direct comparison, not a pooling of previous own-only measurements. The source/worker/WASM assets on the public website were not changed by this experiment. No external communication or modification to the original verification source was made.
