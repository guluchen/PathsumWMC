import init, {check_equivalence, ExternalJob} from './pkg/pathsum_wasm.js'

const ready=init()
let sequence=0
const pending=new Map<number,{resolve:(data:any)=>void,reject:(error:Error)=>void}>()
function count(engine:string,cnf:string):Promise<any> {
  return new Promise((resolve,reject)=>{
    const id=++sequence;pending.set(id,{resolve,reject})
    self.postMessage({count_request:{id,engine,cnf}})
  })
}
self.onmessage=async ({data})=>{
  if(data.count_result) {
    const response=data.count_result,waiter=pending.get(response.id)
    if(waiter){pending.delete(response.id);response.error?waiter.reject(new Error(response.error)):waiter.resolve(response)}
    return
  }
  try {
    await ready
    if(data.init) { self.postMessage({ready:true}); return }
    const request={...data.request}
    if(!['gpmc','ganak'].includes(request.counter)) {
      self.postMessage({report:JSON.parse(check_equivalence(JSON.stringify(request)))}); return
    }
    let prec=Math.min(4096,Math.max(32,request.precision))
    const maxPrec=Math.min(4096,Math.max(prec,request.max_precision))
    for(;;) {
      request.precision=prec
      const prepared=new ExternalJob(JSON.stringify(request))
      try {
      const job=JSON.parse(prepared.job())
      if(job.error) throw new Error(job.error)
      if(job.report) { self.postMessage({report:job.report}); return }
      const outputs:string[]=[]
      const result={elapsed_ms:0,load_ms:0,heap_bytes:0}
      for(const cnf of job.cnfs) {
      const part=await count(job.engine,cnf)
      outputs.push(part.stdout);result.elapsed_ms+=part.elapsed_ms;result.load_ms+=part.load_ms
      result.heap_bytes=Math.max(result.heap_bytes,part.heap_bytes||0)
      }
      const report=JSON.parse(prepared.finish(JSON.stringify(outputs),Math.ceil(result.elapsed_ms)))
      if(report.error) throw new Error(report.error)
      report.engine_timing={load_ms:result.load_ms,solve_ms:result.elapsed_ms,heap_bytes:result.heap_bytes}
      if(report.verdict==='inconclusive' && prec<maxPrec) {
        prec=Math.min(maxPrec,prec*2);continue
      }
      self.postMessage({report,...(data.diagnostics?{diagnostics:{cnfs:job.cnfs,stdout:outputs}}:{})});return
      } finally { prepared.free() }
    }
  } catch(e) { self.postMessage({error:e instanceof Error?e.message:String(e)}) }
}
