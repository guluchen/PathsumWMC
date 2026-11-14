# Findings from the corrected browser ports

The full screen completed: **184/184 cells, 456 trials**, including **68 five-repeat cells**. All repeated statuses were stable. There were **0 interval conflicts** in 46 same-mode engine comparisons and in the additional joint Hybrid/WMC/engine comparison across 26 cases. All **40 native exact-count replays matched**. These checks support portability and internal consistency; they are not an independent validation against the original paper implementation.

## Coverage and usefulness

See [the full status table](RESULTS.md), [per-cell timings](results.csv) and [machine-readable audit](results.json). Of 184 cells, 116 returned finite intervals: 98 had width at most `1e-6`, and 18 were wider. There were 44 timeouts, 15 counter errors, 9 reduction-only UNKNOWN results and no browser crashes. A finite interval is not automatically an equivalence proof. Across the finite results, verdicts were 19 exact equivalence, 13 global-phase equivalence, 31 non-equivalence and 53 inconclusive; these counts include repeated modes/engines for the same input pair, not unique circuits.

Hybrid GPMC and GANAK each returned 26/37 finite bounds, with 24 and 25 respectively narrower than `1e-6`. Three successful Hybrid cells per engine finished entirely by reduction, without invoking that counter. Pure WMC returned 21/37 finite bounds for GPMC and 23/37 for GANAK, but only 14 and 15 respectively were narrower than `1e-6`. Thus a fast response alone overstates useful coverage: e.g. pure-WMC graphstate32/64 at 128 bits returned an essentially full-range distance interval. Increasing precision and tightening the global error bound remain separate performance/precision work.

## Actual portability overhead

For the **20 paired first trials with native GPMC process time ≥ 100 ms**, the median of WASM counter time / native process time was **4.83×**, with range **2.86–7.56×**. Every pair uses identical generated CNFs and the same exact-rational GPMC adapter, cache and stack settings. Native time includes process startup; WASM counter time excludes the separately recorded module load and Rust preparation/finishing time. This is a selected single-trial portability diagnostic, not a five-repeat speedup estimate, a GANAK result or a comparison with the paper's MPFR program. All 40 matched pairs, including those below the 100 ms cutoff, are in [performance.json](performance.json).

For example, Hybrid qpeinexact20 took 20.421 s for the complete browser request: 15.002 s in the WASM counter, versus 3.310 s for native counting. The counter path therefore has substantial browser overhead, in addition to encoding and ball arithmetic. This screen does not separate rational-arithmetic cost from search and runtime costs. On the paper hardware or a different browser, these numbers can change. Shards used several GitHub-hosted CPU models; comparisons between engines/modes of one input were kept on the same host, and per-shard metadata is preserved.

## Limits and remaining failures

All 15 errors came from GPMC. **14 occurred at the 512 MiB linear-memory ceiling**, mainly QFT20/22/24, QNN8/12, QPE24 and VQE16. GNU MP allocation failures are explicitly present in most of those logs; reaching the ceiling alone does not identify every abort's exact cause. The remaining Grover5 Hybrid abort occurred at 450,887,680 allocated heap bytes and remains unclassified; it must not be called a confirmed out-of-memory error. No incomplete case is assigned a distance, native speedup or fabricated timeout duration.

GANAK had no counter errors in this screen, but timed out in 11/37 Hybrid and 14/37 pure-WMC cells. It is not uniformly faster: on Hybrid qft16 it took 25.492 s versus GPMC's 39.407 s, while on Hybrid vqe8 it took 68.875 s versus GPMC's 36.361 s. These longer cells are single screening trials. The pure-WMC and Hybrid comparisons also include different encoding/reduction work, so they should not be read as standalone counter rankings.

The ports are functional and passed their correctness/cleanup tests, but **the large-paper workload does not have uniformly satisfactory browser efficiency or capacity**. The evidence motivates tighter rational arithmetic/error accounting and investigation of GPMC's remaining abort; the current result does not justify claiming native-like performance or complete reproduction of the paper's solved set.

## Permanent evidence

Canonical run: [43592259882](https://github.com/guluchen/PathsumWMC/actions/runs/43592259882). Frozen validated site: [43591913232](https://github.com/guluchen/PathsumWMC/actions/runs/43591913232). Complete raw JSONL, metadata and summaries are archived in [paper-benchmark-20261114](https://github.com/guluchen/PathsumWMC/releases/tag/paper-benchmark-20261114). The current deployed runtime is byte-identical to this frozen build; only permanent-source provenance was subsequently updated, and all 35 deployed runtime/source asset hashes were verified.

Source provenance, resource settings and reproduction instructions are in [the benchmark README](../README.md) and [manifest](../manifest.json). Earlier canceled or pre-fix timing runs are excluded from every table here.
