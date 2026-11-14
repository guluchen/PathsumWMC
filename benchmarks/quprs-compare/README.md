# Original QuPRS versus PathsumWMC

This comparison runs the original tool rather than only its benchmark inputs or our counter adapter. The original is [PhysicsQoo/QuPRS](https://github.com/PhysicsQoo/QuPRS/tree/60bcb250da4eb992f0b5f6727a5b6e57db8f0a84), revision `60bcb250da4eb992f0b5f6727a5b6e57db8f0a84` (2026-03-19), the last repository revision preceding the paper's arXiv submission. Its GPMC submodule is `a1bda3489e959d3695b6798d327c5a1fc1bed305`; it is compiled from that source with its original MPFR arithmetic. This is the paper-era Python implementation, **not the newer QuPRS Rust backend**. No original verification source is patched.

Inputs and their 146 file hashes are the same [73 original-paper artifact pairs](../paper-2026/manifest.json) as our earlier browser screen. Neither circuit angles nor gate order are normalized. The source/transpiled direction remains as listed in the manifest. This artifact cohort is a superset of the printed table; the experiment does not reconstruct every unpublished per-case paper setting.

The complete same-host screen is [run 43608139119](https://github.com/guluchen/PathsumWMC/actions/runs/43608139119): 440/440 cells, 1,416 trials. See [findings and limitations](results/FINDINGS.md), [mode/profile tables](results/RESULTS.md), and [per-cell times](results/results.csv).

## Systems and settings

Each MQTBench input runs Hybrid and WMC; each Feynman input runs reduction-only. There are 110 case/mode combinations, each tested with four separately reported configurations, for **440 expected cells**:

1. `quprs-default`: original Python `check_equivalence`, `backend='python'`, `strategy='Difference'` (the historical public API default), `tool_name='gpmc'`, `safe_mode=False`, default `TOLERANCE=1e-10`.
2. `quprs-proportional`: the same API with `strategy='Proportional'`, also used in upstream benchmark tests. This is a separate strategy sensitivity panel, not a per-case tuned best-of claim. Upstream tests also use other strategies and sometimes reverse input direction; this experiment does not silently optimize those settings.
3. `ours-native`: the same exact-rational external API as our browser, exposed by the benchmark-only `compare_exact` example. It invokes the native build of our pinned GPMC rational adapter. It does **not** use our legacy floating subprocess CLI. Fixed 128-bit ball precision, GPMC cache 64 MiB.
4. `ours-wasm`: the frozen tested browser site from [build 43591913232](https://github.com/guluchen/PathsumWMC/actions/runs/43591913232), source `802195fb9b0690327931b892d713bcd07c80440f`, using GPMC. Existing static website assets are unchanged. GANAK is outside this direct three-system comparison; its earlier WASM results remain separately documented.

The GPMC adapter retains upstream search/counting with exact GMP complex-rational weights; this differs from the original tool's MPFR numerical path. Engine revisions and binary hashes are retained. The experiment compares complete implementations and their numerical contracts, not solely frontend language or search algorithm speed.

## Timing and resource policy

All four configurations of one input run serially on the **same pinned logical CPU and host**, in rotated configuration order. GitHub-hosted runner CPU models differ between shards, so only paired same-input ratios should be used. Native/original subprocesses inherit CPU affinity and use a 4 GiB address-space cap and 16 MiB stack. WASM modules retain the deployed 512 MiB/module ceiling and checked C++ 16 MiB stack. Capacity outcomes under these different limits are reported explicitly, not described as equal-memory comparisons. Each request has a 200-second outer wall limit, with subprocess-group termination including child counters on timeout or exit. Per-trial temporary directories are removed.

Two timings are retained for subprocess tools: whole-process wall time (including Python import/process startup), and request/API time after import/request decoding. Browser request time includes fresh worker/module initialization, encoding, solving and distance finishing, with the browser already open and assets served over local HTTP. It excludes WAN download and Chromium startup. API times avoid presenting Python startup alone as an algorithmic speedup; the whole-process column remains available for end-user cost. No timer is treated as OS CPU time.

Each cell gets one screening trial. Only a first completed DECIDED/BOUND trial below two seconds in whole-process/browser-request time gets four additional trials. Longer cells are explicitly single trials, not five-seed medians. Unstable repeated statuses are flagged and excluded from speed ratios. No timeout or UNKNOWN result is assigned a synthetic finite solve time or distance. Selected speed summaries additionally require original API time at least 100 ms and our interval width at most `1e-6`; per-cell data and broader completed-pair ratios remain available.

The original's `equivalent` / `equivalent*` verdict is a historical numerical acceptance, **not a certified exact equivalence or a rigorous Frobenius interval**. Its reported `log_wmc` is rounded by the original code and cannot be converted back into a certified distance. Our finite interval may remain inconclusive; accepting zero overlap as exact equality is prohibited. Original acceptance with a strictly positive lower distance bound is recorded as a numerical/source-semantic difference, not silently treated as agreement or a speed win on identical correctness guarantees.

## Reproduction

Workflow: `.github/workflows/quprs-compare.yml`; driver: `web/tests/quprs-compare.cjs`; original wrapper: `quprs_runner.py`. The prepare job freezes binaries and the full resolved Python environment, validates real weighted counting and a near-identity diagnostic, and only then releases timing jobs. Python's `PYTHONHASHSEED=0`; BLAS/OpenMP threads are limited to one. The original source and both original GPMC and our binaries have recorded revisions/hashes. The comparison cannot mutate the public website runtime.

Download all 24 `quprs-comparison-N` artifacts and run:

```sh
python3 scripts/summarize-quprs-compare.py RAW_DIRECTORY OUTPUT_DIRECTORY
```

The audit requires 440 distinct expected cells and complete conditional repetitions, checks that each input stays on one shard, and rejects conflicting native/WASM/repeat/mode intervals. Eight report-audit regressions cover missing/duplicate cells, incomplete repetitions, unresolved-speed exclusion, retained numerical verdict differences, conflicting intervals and unstable status/verdict visibility. Setup failures and canceled setup attempts contain no accepted timing evidence and are excluded.
