import init, {check_equivalence, prepare_external, finish_external} from './pkg/pathsum_wasm.js'

const ready=init()
self.onmessage=async ({data})=>{
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
      const job=JSON.parse(prepare_external(JSON.stringify(request)))
      if(job.error) throw new Error(job.error)
      if(job.report) { self.postMessage({report:job.report}); return }
      const result=await new Promise<any>((resolve,reject)=>{
        const worker=new Worker(new URL('runner.js',data.engineRoot))
        worker.onerror=(e)=>{worker.terminate();reject(new Error(e.message))}
        worker.onmessage=({data:response})=>{if(response.progress)return;worker.terminate(); response.error?reject(new Error(response.error)):resolve(response)}
        worker.postMessage(job)
      })
      const report=JSON.parse(finish_external(JSON.stringify(request),result.stdout,Math.ceil(result.elapsed_ms)))
      if(report.error) throw new Error(report.error)
      report.engine_timing={load_ms:result.load_ms,solve_ms:result.elapsed_ms,heap_bytes:result.heap_bytes}
      if(report.verdict==='inconclusive' && prec<maxPrec) {
        prec=Math.min(maxPrec,prec*2);continue
      }
      self.postMessage({report});return
    }
  } catch(e) { self.postMessage({error:e instanceof Error?e.message:String(e)}) }
}
