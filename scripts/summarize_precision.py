import json,sys
from pathlib import Path
rows=[json.loads(l) for l in Path(sys.argv[1]).read_text().splitlines()]
assert len(rows)==45 and len({(r["case"],r["fixed_precision_bits"]) for r in rows})==45
bits=[32,64,128,256,512];cases=list(dict.fromkeys(r["case"] for r in rows))
labels={"proven_equivalent":"Equivalent","proven_global_phase":"Global phase","proven_not_equivalent":"Not equivalent","inconclusive":"Inconclusive"}
lines=["# Fixed-precision experiment", "", "All three built-in counters; fixed initial and maximum precision per cell. General-angle interval overlap is inconclusive. Exact and symbolic proofs retain positive verdicts.", "", "| Case | Known relation | 32 bits | 64 bits | 128 bits | 256 bits | 512 bits |", "|---|---|---|---|---|---|---|"]
for case in cases:
 data={r["fixed_precision_bits"]:r for r in rows if r["case"]==case};assert set(data)==set(bits)
 lines.append("| "+" | ".join([case,data[32]["ground_truth"],*[labels.get(data[b]["report"]["verdict"],data[b]["report"]["verdict"]) for b in bits]])+" |")
lines += ["", "Intervals and all per-counter/reference reports are preserved in precision.jsonl. This is a small correctness/precision study, not a performance benchmark."]
lines += ["", "## Frobenius distance bounds", "", "Normalized by sqrt(2^n),global phase minimized. Display bounds are rounded outward;exact rational endpoints remain in JSONL.", "", "| Case | 32 bits | 64 bits | 128 bits | 256 bits | 512 bits |", "|---|---|---|---|---|---|"]
for case in cases:
    data={r["fixed_precision_bits"]:r for r in rows if r["case"]==case}
    values=[]
    for b in bits:
        distance=data[b]["report"].get("frobenius_distance")
        values.append("Unavailable" if distance is None else "["+distance["lower"]+", "+distance["upper"]+"]")
    lines.append("| "+" | ".join([case,*values])+" |")
Path(sys.argv[2]).write_text("\n".join(lines)+"\n")
