import copy,importlib.util,json,pathlib,tempfile,unittest
ROOT=pathlib.Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('report',ROOT/'scripts/summarize-quprs-compare.py');report=importlib.util.module_from_spec(spec);spec.loader.exec_module(report)
MANIFEST=json.loads((ROOT/'benchmarks/paper-2026/manifest.json').read_text())
class ComparisonAudit(unittest.TestCase):
 def rows(self):
  rows=[]
  for c in MANIFEST['cases']:
   for mode in ['reduction'] if c['family']=='Feynman' else ['hybrid','wmc']:
    for system in report.SYSTEMS:
     original=system.startswith('quprs-')
     rows.append({'id':c['id'],'mode':mode,'system':system,'trial':0,'status':'DECIDED' if original else 'BOUND','request_ms':3000,'process_ms':3100 if original else None,
      'report':{'equivalent':'equivalent'} if original else {'verdict':'proven_equivalent','frobenius_distance':{'lower':'0','upper':'0','lower_exact':'0','upper_exact':'0'}}})
  return rows
 def audit(self,rows):
  with tempfile.TemporaryDirectory(prefix='pathsum-quprs-report-') as temp:
   p=pathlib.Path(temp);(p/'rows.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in rows));return report.audit(p)
 def test_complete_profiles(self):
  r=self.audit(self.rows());self.assertTrue(r['complete']);self.assertEqual(r['recorded_cells'],440);self.assertFalse(r['interval_conflicts']);self.assertEqual(len(r['paired_request_ratios']),440)
 def test_missing_and_duplicate(self):
  self.assertFalse(self.audit(self.rows()[:-1])['complete']);self.assertFalse(self.audit(self.rows()+self.rows()[:1])['complete'])
 def test_missing_conditional_repeats(self):
  rows=self.rows();rows[0]['process_ms']=100;self.assertFalse(self.audit(rows)['complete'])
 def test_timeout_not_given_speed_ratio(self):
  rows=self.rows();rows[0]['status']='TIMEOUT';rows[0]['report']={};r=self.audit(rows);self.assertTrue(r['complete']);self.assertEqual(len(r['paired_request_ratios']),438)
 def test_strict_difference_is_retained(self):
  rows=self.rows()
  for r in rows[:4]:
   if r['system'].startswith('ours-'):r['report']['frobenius_distance']={'lower':'0.1','upper':'0.1','lower_exact':'1/10','upper_exact':'1/10'}
  result=self.audit(rows);self.assertTrue(result['complete']);self.assertEqual(len(result['numerical_verdict_differences']),4)
 def test_interval_conflict_detected(self):
  rows=self.rows();rows[2]['report']['frobenius_distance']={'lower':'1','upper':'1','lower_exact':'1','upper_exact':'1'};r=self.audit(rows);self.assertTrue(r['interval_conflicts'])
 def test_status_instability_visible(self):
  rows=self.rows();first=rows[0];first['process_ms']=100
  for trial in range(1,5):
   r=copy.deepcopy(first);r['trial']=trial
   if trial==4:r['status']='TIMEOUT';r['report']={}
   rows.append(r)
  result=self.audit(rows);self.assertTrue(result['complete']);self.assertTrue(result['unstable_status']);self.assertEqual(len(result['paired_request_ratios']),438)
 def test_verdict_instability_is_not_a_speed_win(self):
  rows=self.rows();first=rows[0];first['process_ms']=100
  for trial in range(1,5):
   r=copy.deepcopy(first);r['trial']=trial
   if trial==4:r['report']['equivalent']='not_equivalent'
   rows.append(r)
  result=self.audit(rows);self.assertTrue(result['complete']);self.assertFalse(result['unstable_status']);self.assertTrue(result['unstable_verdict']);self.assertEqual(len(result['paired_request_ratios']),438)
if __name__=='__main__':unittest.main()
