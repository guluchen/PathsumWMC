# Original-paper artifact benchmark

This screen exercises the browser reimplementation on the paired inputs distributed with [Huang et al., TACAS 2026](https://arxiv.org/abs/2604.24504). Inputs come from [PhysicsQoo/QuPRS](https://github.com/PhysicsQoo/QuPRS/tree/60bcb250da4eb992f0b5f6727a5b6e57db8f0a84), revision `60bcb250da4eb992f0b5f6727a5b6e57db8f0a84`, the last repository commit preceding the arXiv submission. `manifest.json` records all 146 original-file hashes. No circuits were regenerated and no angles were normalized.

The artifact contains 37 MQTBench pairs and 36 Feynman pairs. It is a superset of the printed paper table; these counts should not be described as exactly the paper's printed cohort. MQTBench exercises Hybrid and WMC modes with actual GPMC and GANAK WASM counters, giving 148 cells. Feynman exercises reduction-only, matching the role of RR in the paper, giving 36 cells. Feynman rows do not measure either external counter.

See [findings and limitations](results/FINDINGS.md), [the complete status table](results/RESULTS.md), and [per-cell times](results/results.csv).

## Measurement

- Frozen tested site: [Actions build 43591913232](https://github.com/guluchen/PathsumWMC/actions/runs/43591913232), source `802195fb9b0690327931b892d713bcd07c80440f`.
- Canonical screen: [Actions run 43592259882](https://github.com/guluchen/PathsumWMC/actions/runs/43592259882), twelve shards. Each shard runs serially on one pinned logical CPU. Per-shard CPU/browser metadata and binary hashes are retained with raw results.
- Chromium loads the static site over local HTTP. Times include worker initialization, WASM work, encoding and distance computation, but not WAN download. This is not a speedup comparison against the original paper's hardware, MPFR implementation or solver versions.
- A 200-second whole-request cap and fixed 128-bit ball precision apply. Only cells whose first finite bound completes below 2 seconds receive four additional trials; those cells report a five-repeat median. All other cells are explicitly single screening trials. The Feynman paper RR reference used 80 seconds; this screen uses the common 200-second cap.
- Each WASM module has a 512 MiB linear-memory ceiling. Both C++ counters have a checked 16 MiB stack; GPMC cache is 64 MiB and GANAK cache is 128 MiB. GANAK uses deterministic exact complex-rational mode (`--mode 2 --prob 0`). GPMC uses the exact GMP complex-rational adapter. Resource clocks are worker-local elapsed wall time, not OS CPU time or RSS.
- A finite Frobenius-distance interval is `BOUND`, not automatically exact equivalence. Width greater than `1e-6` is flagged. An interval containing zero alone cannot prove exact equality. `UNKNOWN`, `TIMEOUT`, counter `ERROR` and `BROWSER_CRASH` are distinct outcomes; no unresolved row is assigned a distance or a timeout-derived speedup.
- For first GPMC trials completing in under 30 seconds and producing counter jobs, the exact same CNFs are replayed with the same rational adapter compiled natively. Native times include process startup, with a 4 GiB address-space cap and the same 16 MiB stack/cache settings. This is a portability check, not an independent algorithm or the original paper implementation. Small native times are dominated by process startup. No GANAK-native overhead claim is made.

## Reproduction and evidence

Dispatch `.github/workflows/paper-bench.yml` with `build_run=43591913232`. The frozen site artifact and exact input revision are checked before execution. The driver is `web/tests/paper-bench.cjs`. Do not combine earlier canceled or pre-fix diagnostic runs with this screen.

Download all twelve `paper-benchmark-*` artifacts from the canonical run, then run:

```sh
python3 scripts/summarize-paper-bench.py RAW_DIRECTORY benchmarks/paper-2026/results
```

The audit requires all 184 expected cells, complete conditional repetitions, no duplicate trials, overlapping intervals across repeat/engine comparisons, and agreement with every completed native exact count. It emits the summary, per-cell CSV and JSON. Raw JSONL retains exact rational interval endpoints, errors, timings and resource diagnostics; metadata records CPU, browser and frozen binaries. Five regression tests exercise the report audit's failure paths.

Complete port source bundles and rebuild instructions are permanently archived in [counter-wasm-20261114-v2](https://github.com/guluchen/PathsumWMC/releases/tag/counter-wasm-20261114-v2). The earlier v1 release and earlier timing pilots are superseded for current-port performance.
