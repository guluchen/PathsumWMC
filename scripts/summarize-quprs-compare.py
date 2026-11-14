#!/usr/bin/env python3
"""Audit complete original-tool comparison; retain differing numerical contracts."""
import argparse,collections,csv,json,math,pathlib,statistics
from fractions import Fraction
ROOT=pathlib.Path(__file__).resolve().parents[1]
SYSTEMS=['quprs-default','quprs-proportional','ours-native','ours-wasm']
def audit(raw):
 manifest=json.loads((ROOT/'benchmarks/paper-2026/manifest.json').read_text())
 expected={(c['id'],m,s) for c in manifest['cases'] for m in (['reduction'] if c['family']=='Feynman' else ['hybrid','wmc']) for s in SYSTEMS}
 groups=collections.defaultdict(list);locations=collections.defaultdict(set)
 for f in sorted(raw.rglob('rows.jsonl')):
  for line in f.read_text().splitlines():
   if not line.strip():continue
   r=json.loads(line);key=(r['id'],r['mode'],r['system']);groups[key].append(r);locations[r['id']].add(str(f))
 missing=sorted(expected-groups.keys());unexpected=sorted(groups.keys()-expected)
 bad_trials=[];bounds=collections.defaultdict(list);cells=[]
 for key,rs in sorted(groups.items()):
  rs.sort(key=lambda r:r['trial']);first=rs[0];elapsed=first.get('process_ms') if first.get('process_ms') is not None else first.get('request_ms')
  count=5 if first['status'] in ['BOUND','DECIDED'] and elapsed is not None and elapsed<2000 else 1
  if [r['trial'] for r in rs]!=list(range(count)):bad_trials.append(key)
  ds=[r.get('report',{}).get('frobenius_distance') for r in rs];ds=[d for d in ds if d]
  for d in ds:bounds[(key[0],key[1])].append(d)
  d=ds[0] if ds else None
  median=lambda field:statistics.median(r[field] for r in rs if r.get(field) is not None) if any(r.get(field) is not None for r in rs) else None
  native_or_wasm=key[2].startswith('ours-')
  cells.append({'case':key[0],'mode':key[1],'system':key[2],'trials':len(rs),'status':first['status'],
   'stable_status':len({r['status'] for r in rs})==1,
   'stable_verdict':len({r.get('report',{}).get('verdict' if native_or_wasm else 'equivalent') for r in rs})==1,
   'verdict':first.get('report',{}).get('verdict' if native_or_wasm else 'equivalent'),
   'median_request_s':median('request_ms')/1000 if median('request_ms') is not None else None,
   'median_process_s':median('process_ms')/1000 if median('process_ms') is not None else None,
   'timing_kind':'5-repeat median' if len(rs)==5 else 'one screening trial',
   'distance_lower':d['lower'] if d else None,'distance_upper':d['upper'] if d else None,
   'lower_exact':d['lower_exact'] if d else None,'upper_exact':d['upper_exact'] if d else None,
   'tight_1e_6':Fraction(d['upper_exact'])-Fraction(d['lower_exact'])<=Fraction(1,10**6) if d else None,
   'error':first.get('error','')})
 interval_conflicts=[key for key,ds in bounds.items() if max(Fraction(d['lower_exact']) for d in ds)>min(Fraction(d['upper_exact']) for d in ds)]
 cross_mode=collections.defaultdict(list)
 for (case,mode),ds in bounds.items():cross_mode[case].extend(ds)
 cross_mode_conflicts=[case for case,ds in cross_mode.items() if max(Fraction(d['lower_exact']) for d in ds)>min(Fraction(d['upper_exact']) for d in ds)]
 unstable=[(r['case'],r['mode'],r['system']) for r in cells if not r['stable_status']]
 unstable_verdict=[(r['case'],r['mode'],r['system']) for r in cells if not r['stable_verdict']]
 by={(c['case'],c['mode'],c['system']):c for c in cells};ratios=[];verdict_differences=[]
 for c in cells:
  if not c['system'].startswith('quprs-'):continue
  for s in ['ours-native','ours-wasm']:
   own=by.get((c['case'],c['mode'],s))
   if not own:continue
   if c['status']=='DECIDED' and own['status']=='BOUND' and c['stable_status'] and own['stable_status'] and c['stable_verdict'] and own['stable_verdict']:
    # No timeout/UNKNOWN substitution and no assertion that numerical acceptance is proof.
    if c['median_request_s'] and own['median_request_s'] is not None:
     ratios.append({'case':c['case'],'mode':c['mode'],'original_profile':c['system'],'ours':s,
      'original_request_s':c['median_request_s'],'our_request_s':own['median_request_s'],
      'ratio_ours_over_original':own['median_request_s']/c['median_request_s'],
      'original_ge_100ms':c['median_request_s']>=0.1,'our_tight_1e_6':own['tight_1e_6'],
      'original_trials':c['trials'],'our_trials':own['trials']})
    accept=c['verdict'] in ['equivalent','equivalent*']
    if (accept and Fraction(own['lower_exact'])>0) or (c['verdict']=='not_equivalent' and Fraction(own['upper_exact'])==0):
     verdict_differences.append({'case':c['case'],'mode':c['mode'],'original_profile':c['system'],'original_verdict':c['verdict'],'ours':s,'our_verdict':own['verdict'],'our_lower':own['distance_lower'],'our_upper':own['distance_upper']})
 same_host_violations=sorted(case for case,files in locations.items() if len(files)>1)
 return {'complete':not(missing or unexpected or bad_trials or same_host_violations),'expected_cells':len(expected),'recorded_cells':len(groups),'trials':sum(map(len,groups.values())),
  'missing':missing,'unexpected':unexpected,'bad_trials':bad_trials,'same_host_violations':same_host_violations,'unstable_status':unstable,'unstable_verdict':unstable_verdict,
  'interval_conflicts':interval_conflicts,'cross_mode_interval_conflicts':cross_mode_conflicts,
  'cells':cells,'paired_request_ratios':ratios,'numerical_verdict_differences':verdict_differences}
def write(result,out):
 out.mkdir(parents=True,exist_ok=True);(out/'results.json').write_text(json.dumps(result,indent=2)+'\n')
 with (out/'results.csv').open('w') as f:
  cells=result['cells'];w=csv.DictWriter(f,fieldnames=list(cells[0]) if cells else ['case']);w.writeheader();w.writerows(cells)
 lines=['# Original QuPRS paired comparison','',f"Coverage: {result['recorded_cells']}/{result['expected_cells']} cells, {result['trials']} trials.",'',
 'QuPRS DECIDED is its historical numerical classification, not a certified distance interval. Our BOUND is a finite inclusive interval, not automatically an exact-equivalence decision. Each cell has one screening trial or a conditional five-repeat median; timeout/UNKNOWN cells receive no speed ratio. Request times exclude Python imports/native frontend startup and include browser worker/module initialization. Whole-process times are separately retained.','',
 '| Cohort / mode / system | Cells | Decided / finite bound | Tight bounds ≤ 1e-6 | Timeout | Error / memory | Unknown | Browser crash |','|---|---:|---:|---:|---:|---:|---:|---:|']
 for family,modes in [('MQTBench',['hybrid','wmc']),('Feynman',['reduction'])]:
  for mode in modes:
   for system in SYSTEMS:
    cs=[r for r in result['cells'] if r['case'].startswith(family+'/') and r['mode']==mode and r['system']==system]
    counts=collections.Counter(r['status'] for r in cs)
    lines.append(f"| {family} / {mode} / {system} | {len(cs)} | {counts['DECIDED']+counts['BOUND']} | {sum(r['status']=='BOUND' and r['tight_1e_6'] for r in cs) if system.startswith('ours-') else 'n/a'} | {counts['TIMEOUT']} | {counts['ERROR']+counts['MEMORY_LIMIT']} | {counts['UNKNOWN']+counts['INCONCLUSIVE']} | {counts['BROWSER_CRASH']} |")
 lines+=['','## Paired request-time ratios','','These are descriptive ratios of per-cell request times on completed matching cells; they mix single trials and conditional five-repeat medians. The selected subset requires an original request ≥ 0.1 s and our interval width ≤ 1e-6. Different numerical guarantees and memory ceilings remain. A ratio below 1 means our request completed faster; no paper-hardware or proof-equivalence claim follows.','',
 '| Mode / original profile / our system | Selected completed pairs | Median ours/original | Range |','|---|---:|---:|---:|']
 for mode in ['hybrid','wmc','reduction']:
  for profile in SYSTEMS[:2]:
   for ours in SYSTEMS[2:]:
    rs=[r['ratio_ours_over_original'] for r in result['paired_request_ratios'] if r['mode']==mode and r['original_profile']==profile and r['ours']==ours and r['original_ge_100ms'] and r['our_tight_1e_6']]
    lines.append(f"| {mode} / {profile} / {ours} | {len(rs)} | {statistics.median(rs):.4g} | {min(rs):.4g}–{max(rs):.4g} |" if rs else f"| {mode} / {profile} / {ours} | 0 | n/a | n/a |")
 lines+=['',f"Numerical acceptance / strict-distance differences: {len(result['numerical_verdict_differences'])} rows (including multiple modes/profiles/systems for one input). See results.json; these are retained as semantic differences, not silently counted as correctness agreement."]
 (out/'RESULTS.md').write_text('\n'.join(lines)+'\n');print('\n'.join(lines))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('raw',type=pathlib.Path);p.add_argument('out',type=pathlib.Path);a=p.parse_args();r=audit(a.raw);write(r,a.out)
 if not r['complete'] or r['interval_conflicts'] or r['cross_mode_interval_conflicts']:raise SystemExit(1)
