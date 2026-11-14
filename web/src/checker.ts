import init, { check_equivalence } from "./pkg/pathsum_wasm.js"

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

let ready: Promise<void> | null = null

export function loadChecker(): Promise<void> {
  ready ??= init().then(() => undefined)
  return ready
}

export function checkEquivalence(request: CheckRequest): Report {
  return JSON.parse(check_equivalence(JSON.stringify(request))) as Report
}
