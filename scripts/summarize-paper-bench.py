#!/usr/bin/env python3
"""Audit complete screening coverage and compare rigorously bounded results."""
import argparse,collections,csv,json,pathlib,statistics
from fractions import Fraction
p=argparse.ArgumentParser();p.add_argument('raw',type=pathlib.Path);p.add_argument('out',type=pathlib.Path);a=p.parse_args()
manifest=json.loads((pathlib.Path(__file__).resolve().parents[1]/'benchmarks/paper-2026/manifest.json').read_text())
rows=[]
for f in sorted(a.raw.rglob('rows.jsonl')):
 rows.extend(json.loads(s) for s in f.read_text().splitlines() if s.strip())
groups=collections.defaultdict(list)
for r in rows:groups[(r['id'],r['mode'],r['counter'])].append(r)
expected=set()
for c in manifest['cases']:
 cells=[('reduction','dpll')] if c['family']=='Feynman' else [(m,e) for m in ['hybrid','wmc'] for e in ['gpmc','ganak']]
 expected.update((c['id'],m,e) for m,e in cells)
missing=sorted(expected-groups.keys());unexpected=sorted(groups.keys()-expected)
summary=[];first={};duplicates=[]
for key,rr in sorted(groups.items()):
 rr.sort(key=lambda r:r['trial']);trials=[r['trial'] for r in rr]
 if len(trials)!=len(set(trials)) or not trials or trials[0]!=0:duplicates.append(key)
 r=rr[0];first[key]=r
 statuses=collections.Counter(v['status'] for v in rr)
 d=r.get('report',{}).get('frobenius_distance')
 summary.append({'case':key[0],'mode':key[1],'counter':key[2],'trials':len(rr),'status':r['status'],
  'stable_status':len(statuses)==1,'verdict':r.get('report',{}).get('verdict'),
  'whole_request_median_s':statistics.median(v['wall_ms'] for v in rr)/1000,
  'timing_kind':'5-repeat median' if len(rr)==5 else 'one screening trial',
  'distance_lower':d['lower'] if d else None,'distance_upper':d['upper'] if d else None,
  'width_gt_1e_6':Fraction(d['upper_exact'])-Fraction(d['lower_exact'])>Fraction(1,10**6) if d else None,
  'actual_counter_invoked':bool(r.get('report',{}).get('counters')),
  'counter_median_s':statistics.median(v.get('report',{}).get('engine_timing',{}).get('solve_ms',0) for v in rr)/1000,
  'load_median_s':statistics.median(v.get('report',{}).get('engine_timing',{}).get('load_ms',0) for v in rr)/1000,
  'max_wasm_heap_bytes':max(v.get('report',{}).get('engine_timing',{}).get('heap_bytes',0) or 0 for v in rr),
  'error':r.get('error','')})
disagreements=[];overlaps=0
for c in manifest['cases']:
 if c['family']!='MQTBench':continue
 for mode in ['hybrid','wmc']:
  pair=[first.get((c['id'],mode,e),{}).get('report',{}).get('frobenius_distance') for e in ['gpmc','ganak']]
  if all(pair):
   lo=max(Fraction(d['lower_exact']) for d in pair);hi=min(Fraction(d['upper_exact']) for d in pair)
   if lo>hi:disagreements.append([c['id'],mode])
   else:overlaps+=1
result={'complete':not(missing or unexpected or duplicates),'expected_cells':len(expected),'recorded_cells':len(groups),'total_trials':len(rows),
 'missing':missing,'unexpected':unexpected,'duplicate_trials':duplicates,'interval_pairs_compared':overlaps+len(disagreements),'interval_disagreements':disagreements,
 'scope':'Original paper-era artifact snapshot; 37 MQT paired inputs and 36 Feynman paired inputs. MQT runs Hybrid and WMC with both external WASM engines; Feynman runs reduction-only as in the paper RR comparison. The full artifact is a superset of the printed table. No source angle normalization or regenerated fixtures.',
 'timing_policy':'200 s whole-request screening; four additional repetitions only for first finite bounds under 2 s. Single trials are not medians. Local-HTTP Chromium CI timings; not comparable as speedups to the paper hardware. BOUND means a finite certified interval, not an exact-equivalence decision. Wide intervals are flagged separately.',
 'status_counts':dict(collections.Counter(r['status'] for r in summary)),'cells':summary}
a.out.mkdir(parents=True,exist_ok=True)
(a.out/'results.json').write_text(json.dumps(result,indent=2)+'\n')
with (a.out/'results.csv').open('w') as f:
 w=csv.DictWriter(f,fieldnames=list(summary[0]) if summary else ['case']);w.writeheader();w.writerows(summary)
lines=['# Paper-artifact browser benchmark','',result['scope'],'',result['timing_policy'],'',f"Coverage: {len(groups)}/{len(expected)} cells, {len(rows)} total trials. Interval disagreements: {len(disagreements)}.",'', '| Cohort / mode / engine | Cells | Finite bounds | Tight bounds (width ≤ 1e-6) | Timeout | Error |','|---|---:|---:|---:|---:|---:|']
for family,mode,engine in [('MQTBench',m,e) for m in ['hybrid','wmc'] for e in ['gpmc','ganak']]+[('Feynman','reduction','dpll')]:
 rr=[r for r in summary if r['case'].startswith(family+'/') and r['mode']==mode and r['counter']==engine]
 lines.append(f"| {family} / {mode} / {engine} | {len(rr)} | {sum(r['status']=='BOUND' for r in rr)} | {sum(r['status']=='BOUND' and not r['width_gt_1e_6'] for r in rr)} | {sum(r['status']=='TIMEOUT' for r in rr)} | {sum(r['status']=='ERROR' for r in rr)} |")
(a.out/'RESULTS.md').write_text('\n'.join(lines)+'\n')
print('\n'.join(lines))
if not result['complete'] or disagreements:raise SystemExit(1)
