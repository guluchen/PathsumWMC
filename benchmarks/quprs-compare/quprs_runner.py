"""Thin, unmodified-API wrapper around the pinned original Python QuPRS."""
import json,sys,time,traceback,math
request=json.load(sys.stdin)
start=time.perf_counter()
try:
 from QuPRS import check_equivalence
 import_ms=(time.perf_counter()-start)*1000
 tick=time.perf_counter()
 result=check_equivalence(request['circuit1'],request['circuit2'],
  method={'hybrid':'hybrid','wmc':'wmc_only','reduction':'reduction_rules'}[request['mode']],
  strategy=sys.argv[1],tool_name='gpmc',timeout=200,safe_mode=False,backend='python')
 request_ms=(time.perf_counter()-tick)*1000
 names=['equivalent','qubit_num','gate_num','gate_num2','method','strategy','verification_time',
        'pathsum_time','to_DIMACS_time','tool_time','wmc_time','expect','log_wmc','progress']
 report={name:getattr(result,name,None) for name in names}
 report={k:(str(v) if isinstance(v,float) and not math.isfinite(v) else v) for k,v in report.items()}
 # Preserve upstream diagnostics rather than interpreting rounded log_wmc as distance.
 print('PATHSUM_COMPARISON_RESULT '+json.dumps({'report':report,'request_ms':request_ms,'import_ms':import_ms},default=str,allow_nan=False),flush=True)
except Exception as e:
 print('PATHSUM_COMPARISON_RESULT '+json.dumps({'error':str(e),'traceback':traceback.format_exc()[-8000:]}),flush=True)
 sys.exit(1)
