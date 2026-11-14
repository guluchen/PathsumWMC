import json,pathlib,subprocess,sys,tempfile,unittest
ROOT=pathlib.Path(__file__).resolve().parents[2]
MANIFEST=json.loads((ROOT/'benchmarks/paper-2026/manifest.json').read_text())
class ReportAudit(unittest.TestCase):
 def rows(self):
  result=[]
  for c in MANIFEST['cases']:
   cells=[('reduction','dpll')] if c['family']=='Feynman' else [(m,e) for m in ['hybrid','wmc'] for e in ['gpmc','ganak']]
   for mode,engine in cells:
    for trial in range(5):
     result.append({'id':c['id'],'mode':mode,'counter':engine,'trial':trial,'status':'BOUND','wall_ms':100,
      'report':{'verdict':'proven_equivalent','frobenius_distance':{'lower':'0','upper':'0','lower_exact':'0','upper_exact':'0'}}})
  return result
 def run_report(self,rows):
  with tempfile.TemporaryDirectory(prefix='pathsum-report-test-') as temp:
   p=pathlib.Path(temp);(p/'rows.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in rows))
   proc=subprocess.run([sys.executable,str(ROOT/'scripts/summarize-paper-bench.py'),str(p),str(p/'out')],capture_output=True,text=True)
   self.assertTrue((p/'out/results.json').exists(),proc.stderr)
   return proc.returncode,json.loads((p/'out/results.json').read_text())
 def test_full_coverage_and_zero_overlap(self):
  code,r=self.run_report(self.rows());self.assertEqual(code,0);self.assertTrue(r['complete']);self.assertEqual(r['expected_cells'],184);self.assertEqual(r['interval_disagreements'],[])
 def test_incomplete_repeat_is_not_complete(self):
  code,r=self.run_report(self.rows()[:-1]);self.assertNotEqual(code,0);self.assertFalse(r['complete']);self.assertTrue(r['incomplete_repetitions'])
 def test_engine_disagreement_is_failure(self):
  rows=self.rows()
  for r in rows:
   if r['counter']=='ganak' and r['id']==MANIFEST['cases'][0]['id']:
    r['report']['frobenius_distance']={'lower':'1','upper':'1','lower_exact':'1','upper_exact':'1'}
  code,r=self.run_report(rows);self.assertNotEqual(code,0);self.assertTrue(r['interval_disagreements'])
if __name__=='__main__':unittest.main()
