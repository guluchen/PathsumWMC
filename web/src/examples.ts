export type Example = {
  id: string
  name: string
  blurb: string
  mode: "hybrid" | "reduction" | "wmc"
  counter: "all" | "enumerate" | "dpll" | "bucket" | "gpmc" | "ganak"
  left: string
  right: string
}

function program(qubits: number, body: string): string {
  return `OPENQASM 2.0;\nqreg q[${qubits}];\n${body.trim()}\n`
}

export const examples: Example[] = [
  {
    id: "cz",
    name: "CZ = H CX H",
    blurb: "The paper’s running example. Reduction reaches the identity.",
    mode: "hybrid",
    counter: "all",
    left: program(2, "cz q[0],q[1];"),
    right: program(2, "h q[1];\ncx q[0],q[1];\nh q[1];"),
  },
  {
    id: "hh",
    name: "HH = I",
    blurb: "Two Hadamards cancel by the [HH] rule.",
    mode: "hybrid",
    counter: "all",
    left: program(1, "h q[0];\nh q[0];"),
    right: program(1, "id q[0];"),
  },
  {
    id: "hzh",
    name: "HZH = X",
    blurb: "A Hadamard sandwich turns Z into X.",
    mode: "reduction",
    counter: "all",
    left: program(1, "h q[0];\nz q[0];\nh q[0];"),
    right: program(1, "x q[0];"),
  },
  {
    id: "t4",
    name: "TTTT = Z",
    blurb: "Four T gates are exactly a Z.",
    mode: "hybrid",
    counter: "all",
    left: program(1, "t q[0];\nt q[0];\nt q[0];\nt q[0];"),
    right: program(1, "z q[0];"),
  },
  {
    id: "phase",
    name: "S vs rz(π/2)",
    blurb: "Qiskit’s rz differs from S by a global phase.",
    mode: "wmc",
    counter: "all",
    left: program(1, "s q[0];"),
    right: program(1, "rz(pi/2) q[0];"),
  },
  {
    id: "bell",
    name: "Bell circuit",
    blurb: "A circuit compared with itself.",
    mode: "hybrid",
    counter: "all",
    left: program(2, "h q[0];\ncx q[0],q[1];"),
    right: program(2, "h q[0];\ncx q[0],q[1];"),
  },
  {
    id: "cancel",
    name: "rx(1/5) cancels",
    blurb: "Opposite rational rotations cancel exactly.",
    mode: "hybrid",
    counter: "all",
    left: program(1, "rx(1/5) q[0];\nrx(-1/5) q[0];"),
    right: program(1, "id q[0];"),
  },
  {
    id: "separate",
    name: "rx(1/5) ≠ rx(1/6)",
    blurb: "Ball arithmetic proves these rotations differ.",
    mode: "wmc",
    counter: "all",
    left: program(1, "rx(1/5) q[0];"),
    right: program(1, "rx(1/6) q[0];"),
  },
  {
    id: "not-id",
    name: "H ≠ I",
    blurb: "Hadamard is not the identity.",
    mode: "wmc",
    counter: "all",
    left: program(1, "h q[0];"),
    right: program(1, "id q[0];"),
  },
]

export const starter = examples[0]
