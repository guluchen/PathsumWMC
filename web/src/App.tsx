import { useEffect, useState, type ReactNode } from "react"
import { ArrowLeftRightIcon, LoaderCircleIcon } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Separator } from "@/components/ui/separator"
import { Textarea } from "@/components/ui/textarea"
import { cancelCheck, checkEquivalence, loadChecker, type Report } from "./checker"
import { examples, starter, type Example } from "./examples"

const verdictLabel: Record<string, string> = {
  proven_equivalent: "Equivalent",
  proven_global_phase: "Global phase",
  proven_not_equivalent: "Not equivalent",
  unknown: "Undecided",
  inconclusive: "Inconclusive",
  error: "Could not check",
}


function kindLabel(kind: string): string {
  if (kind === "wmc") return "Weighted model count"
  if (kind === "reference") return "Reference"
  return kind
}

export default function App() {
  const [left, setLeft] = useState(starter.left)
  const [right, setRight] = useState(starter.right)
  const [mode, setMode] = useState(starter.mode)
  const [counter, setCounter] = useState(starter.counter)
  const [precision, setPrecision] = useState(72)
  const [maxPrecision, setMaxPrecision] = useState(256)
  const [maxNodes, setMaxNodes] = useState(200000)
  const [activeExample, setActiveExample] = useState(starter.id)
  const [wasmReady, setWasmReady] = useState(false)
  const [wasmError, setWasmError] = useState("")
  const [checking, setChecking] = useState(false)
  const [report, setReport] = useState<Report | null>(null)
  const [formError, setFormError] = useState("")

  useEffect(() => {
    loadChecker()
      .then(() => setWasmReady(true))
      .catch((err: unknown) => {
        setWasmError(err instanceof Error ? err.message : "The checker failed to load.")
      })
  }, [])

  function applyExample(example: Example) {
    setLeft(example.left)
    setRight(example.right)
    setMode(example.mode)
    setActiveExample(example.id)
    setReport(null)
    setFormError("")
  }

  async function onCheck() {
    if (!left.trim() || !right.trim()) {
      setReport(null)
      setFormError("Paste an OpenQASM 2 program into both editors. Each one needs a qreg.")
      return
    }
    const prec = clamp(precision, 32, 4096)
    const maxPrec = clamp(Math.max(maxPrecision, prec), 32, 4096)
    const nodes = clamp(maxNodes, 1, 5_000_000)
    setPrecision(prec)
    setMaxPrecision(maxPrec)
    setMaxNodes(nodes)
    setFormError("")
    setChecking(true)
    setReport(null)
      try {
        const next = await checkEquivalence({
          circuit1: left,
          circuit2: right,
          mode,
          counter,
          precision: prec,
          max_precision: maxPrec,
          max_nodes: nodes,
        })
        setReport(next)
      } catch (err) {
        setReport({
          verdict: "error",
          headline: "The checker stopped unexpectedly.",
          detail: err instanceof Error ? err.message : "Unknown failure.",
        })
      } finally {
        setChecking(false)
      }
  }

  return (
    <div className="mx-auto flex min-h-svh w-full max-w-6xl flex-col px-4 py-8 sm:px-6">
      <header className="mb-8 grid gap-4 border-b border-border pb-6 md:grid-cols-[1fr_auto] md:items-end">
        <div className="max-w-3xl">
          <p className="mb-2 text-xs font-medium tracking-[0.16em] text-muted-foreground uppercase">
            PathsumWMC · Ball arithmetic
          </p>
          <h1 className="font-heading text-4xl leading-tight font-medium tracking-tight sm:text-5xl">
            How close are these two quantum circuits?
          </h1>
          <p className="mt-3 text-sm font-medium text-foreground">
            A Cursor reimplementation of our TACAS 2026 paper, using ball arithmetic.
          </p>
          <p className="mt-3 max-w-2xl text-sm leading-6 text-muted-foreground sm:text-base">
            Compute the normalized Frobenius distance between two OpenQASM 2 circuits, ignoring global phase. The checker
            builds the path-sum of U₂U₁†, applies the [HH] and [ω] reductions,
            then evaluates the residual sum. All five counters, including GPMC and GANAK, run locally in your browser using WebAssembly.
          </p>
        </div>
        <a
          className="text-sm text-primary underline-offset-4 hover:underline"
          href="https://arxiv.org/abs/2604.24504"
          target="_blank"
          rel="noreferrer"
        >
          arXiv:2604.24504
        </a>
      </header>

      <fieldset disabled={checking} className="contents">
      <section className="mb-6" aria-label="Examples">
        <div className="mb-2 flex items-baseline justify-between gap-3">
          <h2 className="font-heading text-xl font-medium">Examples</h2>
          <p className="text-xs text-muted-foreground">Load a pair, then change it.</p>
        </div>
        <div className="flex flex-wrap gap-2">
          {examples.map((example) => (
            <Button
              key={example.id}
              type="button"
              size="sm"
              variant={activeExample === example.id ? "default" : "outline"}
              onClick={() => applyExample(example)}
            >
              {example.name}
            </Button>
          ))}
        </div>
        <p className="mt-2 text-sm text-muted-foreground">
          {examples.find((example) => example.id === activeExample)?.blurb}
        </p>
      </section>

      <section className="grid gap-4 lg:grid-cols-2">
        <CircuitEditor
          id="circuit-a"
          title="Circuit A"
          hint="U₁, the circuit that is inverted"
          value={left}
          onChange={(value) => {
            setLeft(value)
            setActiveExample("")
          }}
        />
        <CircuitEditor
          id="circuit-b"
          title="Circuit B"
          hint="U₂, applied after A†"
          value={right}
          onChange={(value) => {
            setRight(value)
            setActiveExample("")
          }}
        />
      </section>

      <Card className="mt-4">
        <CardContent className="grid gap-4 md:grid-cols-2 xl:grid-cols-6">
          <Field label="Mode" htmlFor="mode">
            <Select value={mode} onValueChange={(value) => setMode(value as Example["mode"])}>
              <SelectTrigger id="mode" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="hybrid">Hybrid</SelectItem>
                <SelectItem value="reduction">Reduction only</SelectItem>
                <SelectItem value="wmc">Model counting only</SelectItem>
              </SelectContent>
            </Select>
          </Field>
          <Field label="Engine" htmlFor="counter">
            <Select
              value={counter}
              onValueChange={(value) => setCounter(value as Example["counter"])}
            >
              <SelectTrigger id="counter" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">Built-in counters</SelectItem>
                <SelectItem value="enumerate">Enumerate</SelectItem>
                <SelectItem value="dpll">DPLL</SelectItem>
                <SelectItem value="bucket">Bucket elimination</SelectItem>
                <SelectItem value="gpmc">GPMC</SelectItem>
                <SelectItem value="ganak">GANAK</SelectItem>
              </SelectContent>
            </Select>
          </Field>
          <p className="text-xs leading-5 text-muted-foreground md:col-span-2 xl:col-span-6">
            Switch the engine between the built-in counters, GPMC, and GANAK.
            The two external engines run from the native checker.
          </p>
          <Field label="Precision (bits)" htmlFor="precision">
            <Input
              id="precision"
              type="number"
              min={32}
              max={4096}
              value={precision}
              onChange={(event) => setPrecision(Number(event.target.value))}
            />
          </Field>
          <Field label="Max precision" htmlFor="max-precision">
            <Input
              id="max-precision"
              type="number"
              min={32}
              max={4096}
              value={maxPrecision}
              onChange={(event) => setMaxPrecision(Number(event.target.value))}
            />
          </Field>
          <Field label="Node limit" htmlFor="nodes">
            <Input
              id="nodes"
              type="number"
              min={1}
              value={maxNodes}
              onChange={(event) => setMaxNodes(Number(event.target.value))}
            />
          </Field>
          <div className="flex items-end gap-2">
            <Button
              type="button"
              variant="outline"
              className="flex-1"
              onClick={() => {
                setLeft(right)
                setRight(left)
                setActiveExample("")
              }}
            >
              <ArrowLeftRightIcon />
              Swap
            </Button>
          </div>
        </CardContent>
      </Card>

      </fieldset>

      <div className="mt-4 flex flex-col gap-3 sm:flex-row sm:items-center">
        <Button type="button" size="lg" disabled={!wasmReady || checking} onClick={onCheck}>
          {checking ? <LoaderCircleIcon className="animate-spin" /> : null}
          {checking ? "Checking" : "Compute distance"}
        </Button>
        {checking ? <Button type="button" variant="outline" onClick={cancelCheck}>Stop</Button> : null}
        <p className="text-sm text-muted-foreground" role="status">
          {wasmError
            ? wasmError
            : wasmReady
              ? "The WebAssembly checker is ready."
              : "Loading the checker…"}
        </p>
      </div>

      <section className="mt-6" aria-live="polite">
        {formError ? (
          <Card>
            <CardHeader>
              <CardTitle>Both circuits are required</CardTitle>
              <CardDescription>{formError}</CardDescription>
            </CardHeader>
          </Card>
        ) : report ? (
          <ReportView report={report} />
        ) : (
          <Card>
            <CardHeader>
              <CardTitle>No result yet</CardTitle>
              <CardDescription>
                Choose an example or paste two programs, then check them. Hybrid
                mode reduces first and calls the counters only when a residual
                path-sum remains. Reduction-only mode stops at the rewrite
                rules and reports undecided when they are not enough.
              </CardDescription>
            </CardHeader>
            <CardContent className="grid gap-3 text-sm leading-6 text-muted-foreground">
              <p>
                Angles are exact rationals. <code>0.3*pi</code> is 3/10 of π,
                and <code>pi/2</code> is one half. When every weight is a
                multiple of π/4 the diagonal sum lives in ℚ(i, √2). Any other
                angle uses a complex ball, and a nonzero radius never counts as
                a proof of equivalence by itself.
              </p>
              <p>
                Qiskit’s <code>rz(θ)</code> is diag(e<sup>−iθ/2</sup>, e<sup>iθ/2</sup>).
                It differs from <code>p(θ)</code> = diag(1, e<sup>iθ</sup>) by the
                global phase e<sup>−iθ/2</sup>. That is why S and{" "}
                <code>rz(pi/2)</code> match only up to a phase, giving Frobenius distance zero.
              </p>
            </CardContent>
          </Card>
        )}
      </section>

      <Card className="mt-8" aria-label="About this reimplementation">
        <CardHeader>
          <CardTitle>About PathsumWMC</CardTitle>
          <CardDescription>
            This is a reimplementation, built with Cursor, of our paper
            “Equivalence Checking of Quantum Circuits via Path-Sum and Weighted Model Counting”.
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-3 text-sm leading-6 text-muted-foreground">
          <p>
            The implementation uses exact arithmetic for quarter-turn weights and
            complex ball arithmetic for general angles. A ball interval that
            contains the equivalence threshold remains inconclusive; increasing
            precision can separate close but unequal circuits. Exact equality
            requires symbolic or exact arithmetic evidence.
          </p>
          <p>
            GPMC uses an exact-rational numeric adapter; GANAK runs its deterministic complex-rational mode. Phase approximations are enclosed by ball arithmetic. Computations stop after 200 seconds and can be cancelled.
          </p>
          <p>
            Licensed under the <a className="text-primary underline" href="./LICENSE.txt">MIT License</a>.
            {" "}<a className="text-primary underline" href="https://github.com/guluchen/PathsumWMC">Source code</a>
            {" · "}<a className="text-primary underline" href="https://arxiv.org/abs/2604.24504">Our paper</a>
            {" · "}<a className="text-primary underline" href="./THIRD_PARTY_NOTICES.txt">Third-party notices</a>
            {" · "}<a className="text-primary underline" href="./engines/PROVENANCE.json">Counter versions and sources</a>
          </p>
        </CardContent>
      </Card>

      <footer className="mt-10 border-t border-border pt-4 text-xs leading-5 text-muted-foreground">
        Supported gates: id, h, x, y, z, s, sdg, t, tdg, cx, cz, swap, ccx, ccz,
        rx, ry, rz, p, crx, cry, crz, cp. Measurement and reset are rejected
        because they are not unitary. Huang, Chareton, Chen, Chung, Hsieh,
        Laarman, and Mei, TACAS 2026, arXiv:2604.24504.
      </footer>
    </div>
  )
}

function CircuitEditor({
  id,
  title,
  hint,
  value,
  onChange,
}: {
  id: string
  title: string
  hint: string
  value: string
  onChange: (value: string) => void
}) {
  return (
    <div className="grid gap-2">
      <div className="flex items-baseline justify-between gap-3">
        <Label htmlFor={id} className="font-heading text-lg font-medium">
          {title}
        </Label>
        <span className="text-xs text-muted-foreground">{hint}</span>
      </div>
      <Textarea
        id={id}
        className="qasm"
        spellCheck={false}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
    </div>
  )
}

function Field({
  label,
  htmlFor,
  children,
}: {
  label: string
  htmlFor: string
  children: ReactNode
}) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={htmlFor}>{label}</Label>
      {children}
    </div>
  )
}

function ReportView({ report }: { report: Report }) {
  const steps = report.reduction_steps ?? []
  const counters = report.counters ?? []
  const notes = report.notes ?? []
  return (
    <Card>
      <CardHeader>
        <div className="flex flex-wrap items-center gap-2">
          {report.arithmetic ? (
            <Badge variant="secondary">{report.arithmetic} arithmetic</Badge>
          ) : null}
        </div>
        <CardTitle className="font-heading text-2xl">Frobenius distance</CardTitle>
        <CardDescription>Normalized by √(2ⁿ), with global phase ignored. Inclusive lower and upper bounds.</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-5">
        <output aria-label="Frobenius distance" className="break-words font-mono text-2xl font-semibold sm:text-3xl">
          {report.frobenius_distance
            ? report.frobenius_distance.lower === report.frobenius_distance.upper
              ? report.frobenius_distance.lower
              : `[${report.frobenius_distance.lower}, ${report.frobenius_distance.upper}]`
            : "Unavailable"}
        </output>
        <p className="text-sm leading-6 text-muted-foreground">
          Closer to 0 means the two circuits are closer. A distance of 0 means
          they are identical up to a global phase. For an interval, the upper
          bound tells you how close they are guaranteed to be; a lower bound
          of 0 alone does not establish exact equality.
        </p>
        {!report.frobenius_distance ? <p className="text-sm text-muted-foreground">{report.headline} {report.detail}</p> : null}
        <details className="grid gap-4">
          <summary className="cursor-pointer text-sm font-medium">Computation details</summary>
          <p className="mt-3 text-sm text-muted-foreground">{report.headline} {report.detail}</p>
          <div className="mt-4 grid gap-5">
        <dl className="grid grid-cols-2 gap-3 text-sm sm:grid-cols-4">
          <Stat label="Phase" value={report.phase || "—"} />
          <Stat label="Qubits" value={text(report.qubits)} />
          <Stat label="Gates" value={`${text(report.gates1)} / ${text(report.gates2)}`} />
          <Stat
            label="Path variables"
            value={`${text(report.path_variables_before)} → ${text(report.path_variables_after)}`}
          />
          <Stat label="Monomials" value={text(report.monomials)} />
          <Stat label="CNF variables" value={text(report.variables)} />
          <Stat label="Clauses" value={text(report.clauses)} />
          <Stat label="Precision" value={report.precision_bits ? `${report.precision_bits} bits` : "—"} />
        </dl>
        <Separator />
        <div>
          <h3 className="font-heading mb-2 text-lg font-medium">Reduction</h3>
          {steps.length === 0 ? (
            <p className="text-sm text-muted-foreground">No rewrite fired.</p>
          ) : (
            <ol className="grid gap-1 text-sm">
              {steps.map((step, index) => (
                <li key={`${index}-${step}`} className="font-mono text-xs leading-5">
                  {index + 1}. {step}
                </li>
              ))}
            </ol>
          )}
        </div>
        <div>
          <h3 className="font-heading mb-2 text-lg font-medium">Procedures</h3>
          {counters.length === 0 ? (
            <p className="text-sm text-muted-foreground">{emptyCounters(report)}</p>
          ) : (
            <>
              <div className="hidden overflow-x-auto md:block">
                <table className="w-full text-left text-sm">
                  <thead className="text-xs text-muted-foreground">
                    <tr>
                      <th className="py-2 pr-3 font-medium">Procedure</th>
                      <th className="py-2 pr-3 font-medium">Kind</th>
                      <th className="py-2 pr-3 font-medium">Verdict</th>
                      <th className="py-2 pr-3 font-medium">Normalized sum</th>
                      <th className="py-2 pr-3 font-medium">|sum|²</th>
                      <th className="py-2 font-medium">Time</th>
                    </tr>
                  </thead>
                  <tbody>
                    {counters.map((counter) => (
                      <tr key={`${counter.kind}-${counter.name}`} className="border-t border-border align-top">
                        <td className="py-2 pr-3 font-medium">{counter.name}</td>
                        <td className="py-2 pr-3">{kindLabel(counter.kind)}</td>
                        <td className="py-2 pr-3">
                          {verdictLabel[counter.verdict] ?? counter.verdict}
                          {counter.error ? (
                            <span className="mt-1 block text-xs text-destructive">{counter.error}</span>
                          ) : null}
                        </td>
                        <td className="py-2 pr-3 font-mono text-xs">{counter.value || "—"}</td>
                        <td className="py-2 pr-3 font-mono text-xs">{counter.magnitude_squared || "—"}</td>
                        <td className="py-2">{counter.elapsed_ms} ms</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <div className="grid gap-3 md:hidden">
                {counters.map((counter) => (
                  <div
                    key={`${counter.kind}-${counter.name}`}
                    className="rounded-lg border border-border p-3"
                  >
                    <div className="flex items-center justify-between gap-2">
                      <p className="font-medium">{counter.name}</p>
                      <Badge variant="outline">
                        {verdictLabel[counter.verdict] ?? counter.verdict}
                      </Badge>
                    </div>
                    <p className="mt-1 text-xs text-muted-foreground">{kindLabel(counter.kind)}</p>
                    <p className="mt-2 font-mono text-xs">{counter.value || "—"}</p>
                    <p className="font-mono text-xs text-muted-foreground">
                      |sum|² {counter.magnitude_squared || "—"}
                    </p>
                    {counter.error ? (
                      <p className="mt-2 text-xs text-destructive">{counter.error}</p>
                    ) : null}
                    <p className="mt-2 text-xs text-muted-foreground">{counter.elapsed_ms} ms</p>
                  </div>
                ))}
              </div>
            </>
          )}
        </div>
        {notes.length > 0 ? (
          <ul className="grid gap-2 text-sm leading-6 text-muted-foreground">
            {notes.map((note) => (
              <li key={note}>{note}</li>
            ))}
          </ul>
        ) : null}
          </div>
        </details>
      </CardContent>
    </Card>
  )
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs text-muted-foreground">{label}</dt>
      <dd className="mt-1 break-words font-medium">{value}</dd>
    </div>
  )
}

function emptyCounters(report: Report): string {
  if (report.verdict === "unknown") {
    return "No counter ran. Reduction-only mode stops when the path-sum is not yet an identity."
  }
  if (report.verdict === "error") {
    return "The check stopped before a counter ran."
  }
  return "No counter ran. The reductions already proved the result, so weighted model counting was not required."
}

function text(value: number | undefined): string {
  return value === undefined ? "—" : String(value)
}

function clamp(value: number, lo: number, hi: number): number {
  if (!Number.isFinite(value)) return lo
  return Math.min(hi, Math.max(lo, Math.round(value)))
}
