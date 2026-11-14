
export type CheckRequest = {
  circuit1: string
  circuit2: string
  mode: string
  counter: string
  precision: number
  max_precision: number
  max_nodes: number
}

export type DistanceBounds = {
  lower: string
  upper: string
  lower_exact: string
  upper_exact: string
  normalization: string
  global_phase_ignored: boolean
}

export type CounterReport = {
  name: string
  kind: string
  verdict: string
  value: string
  magnitude_squared: string
  frobenius_distance: DistanceBounds | null
  elapsed_ms: number
  error: string
}

export type Report = {
  engine_timing?: {load_ms:number,solve_ms:number,heap_bytes:number|null}
  verdict: string
  headline: string
  detail: string
  phase?: string
  arithmetic?: string
  qubits?: number
  gates1?: number
  gates2?: number
  path_variables_before?: number
  path_variables_after?: number
  monomials?: number
  variables?: number
  clauses?: number
  precision_bits?: number
  frobenius_distance?: DistanceBounds | null
  reduction_steps?: string[]
  counters?: CounterReport[]
  notes?: string[]
}

let active: {worker:Worker, reject:(e:Error)=>void} | null = null

export function loadChecker(): Promise<void> {
  return new Promise((resolve,reject)=>{
    const worker=new Worker(new URL('./solver.worker.ts',import.meta.url),{type:'module'})
    worker.onmessage=({data})=>{worker.terminate();data.ready?resolve():reject(new Error(data.error))}
    worker.onerror=(e)=>{worker.terminate();reject(new Error(e.message))}
    worker.postMessage({init:true})
  })
}
export function cancelCheck() {
  if(active) { active.worker.terminate();active.reject(new Error('Computation stopped.'));active=null }
}
export function checkEquivalence(request:CheckRequest,timeoutMs=200_000):Promise<Report> {
  cancelCheck()
  return new Promise((resolve,reject)=>{
    const worker=new Worker(new URL('./solver.worker.ts',import.meta.url),{type:'module'})
    const timer=window.setTimeout(()=>finish(undefined,new Error('Time limit reached (200 seconds).')),timeoutMs)
    function finish(report?:Report,error?:Error) {
      window.clearTimeout(timer);worker.terminate()
      if(active?.worker===worker) active=null
      error?reject(error):resolve(report!)
    }
    active={worker,reject:(e)=>finish(undefined,e)}
    worker.onmessage=({data})=>finish(data.report,data.error?new Error(data.error):undefined)
    worker.onerror=(e)=>finish(undefined,new Error(e.message))
    worker.postMessage({request,engineRoot:new URL('./engines/',document.baseURI).href})
  })
}
