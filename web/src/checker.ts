
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

type Host={result:Promise<{report:Report}>,cancel:()=>void}
let active:Host|null=null
let hostModule:any=null
const hostUrl=()=>new URL('./engines/host.js?v=20261114-clock-stack-v2',document.baseURI).href
export async function loadChecker():Promise<void> {
  const wasm=new Promise<void>((resolve,reject)=>{
    const worker=new Worker(new URL('./solver.worker.ts',import.meta.url),{type:'module'})
    const timer=window.setTimeout(()=>{worker.terminate();reject(new Error('The checker did not load within 30 seconds.'))},30000)
    worker.onmessage=({data})=>{window.clearTimeout(timer);worker.terminate();data.ready?resolve():reject(new Error(data.error))}
    worker.onerror=e=>{window.clearTimeout(timer);worker.terminate();reject(new Error(e.message))}
    worker.postMessage({init:true})
  })
  const module=import(/* @vite-ignore */ hostUrl())
  const [host]=await Promise.all([module,wasm]);hostModule=host
}
export function cancelCheck(){active?.cancel()}
export function checkEquivalence(request:CheckRequest,timeoutMs=200000):Promise<Report> {
  cancelCheck()
  if(!hostModule)return Promise.reject(new Error('The checker is not ready.'))
  const worker=new Worker(new URL('./solver.worker.ts',import.meta.url),{type:'module'})
  const host:Host=hostModule.createWorkerHost(worker,new URL('./engines/',document.baseURI).href,timeoutMs)
  active=host;worker.postMessage({request})
  return host.result.then(data=>data.report).finally(()=>{if(active===host)active=null})
}
