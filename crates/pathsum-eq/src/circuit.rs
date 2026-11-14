use crate::phase::Angle;

#[derive(Clone, Debug)]
pub struct Circuit {
    pub qubits: usize,
    pub gates: Vec<Gate>,
}

#[derive(Clone, Debug)]
pub enum Gate {
    H(usize),
    X(usize),
    Y(usize),
    Z(usize),
    S(usize),
    Sdg(usize),
    T(usize),
    Tdg(usize),
    Cx(usize, usize),
    Cz(usize, usize),
    Swap(usize, usize),
    Ccx(usize, usize, usize),
    Ccz(usize, usize, usize),
    /// Qiskit `rz`: `diag(e^{-iθ/2}, e^{iθ/2})`.
    Rz(usize, Angle),
    Rx(usize, Angle),
    Ry(usize, Angle),
    /// Qiskit `p` / `u1`: `diag(1, e^{iθ})`.
    P(usize, Angle),
    Crz(usize, usize, Angle),
    Crx(usize, usize, Angle),
    Cry(usize, usize, Angle),
    /// Qiskit `cp`: phase `θ` on `|11⟩`.
    Cp(usize, usize, Angle),
}

impl Circuit {
    pub fn new(qubits: usize) -> Self {
        Self {
            qubits,
            gates: Vec::new(),
        }
    }

    pub fn push(&mut self, gate: Gate) {
        self.gates.push(gate);
    }

    pub fn dagger(&self) -> Self {
        let mut gates = Vec::with_capacity(self.gates.len());
        for gate in self.gates.iter().rev() {
            gates.push(gate.dagger());
        }
        Self {
            qubits: self.qubits,
            gates,
        }
    }

    pub fn len(&self) -> usize {
        self.gates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.gates.is_empty()
    }
}

impl Gate {
    pub fn dagger(&self) -> Self {
        match self {
            Gate::H(q) => Gate::H(*q),
            Gate::X(q) => Gate::X(*q),
            Gate::Y(q) => Gate::Y(*q),
            Gate::Z(q) => Gate::Z(*q),
            Gate::S(q) => Gate::Sdg(*q),
            Gate::Sdg(q) => Gate::S(*q),
            Gate::T(q) => Gate::Tdg(*q),
            Gate::Tdg(q) => Gate::T(*q),
            Gate::Cx(c, t) => Gate::Cx(*c, *t),
            Gate::Cz(c, t) => Gate::Cz(*c, *t),
            Gate::Swap(a, b) => Gate::Swap(*a, *b),
            Gate::Ccx(a, b, c) => Gate::Ccx(*a, *b, *c),
            Gate::Ccz(a, b, c) => Gate::Ccz(*a, *b, *c),
            Gate::Rz(q, a) => Gate::Rz(*q, a.neg()),
            Gate::Rx(q, a) => Gate::Rx(*q, a.neg()),
            Gate::Ry(q, a) => Gate::Ry(*q, a.neg()),
            Gate::P(q, a) => Gate::P(*q, a.neg()),
            Gate::Crz(c, t, a) => Gate::Crz(*c, *t, a.neg()),
            Gate::Crx(c, t, a) => Gate::Crx(*c, *t, a.neg()),
            Gate::Cry(c, t, a) => Gate::Cry(*c, *t, a.neg()),
            Gate::Cp(c, t, a) => Gate::Cp(*c, *t, a.neg()),
        }
    }

    pub fn qubits(&self) -> Vec<usize> {
        match self {
            Gate::H(q)
            | Gate::X(q)
            | Gate::Y(q)
            | Gate::Z(q)
            | Gate::S(q)
            | Gate::Sdg(q)
            | Gate::T(q)
            | Gate::Tdg(q)
            | Gate::Rz(q, _)
            | Gate::Rx(q, _)
            | Gate::Ry(q, _)
            | Gate::P(q, _) => vec![*q],
            Gate::Cx(a, b)
            | Gate::Cz(a, b)
            | Gate::Swap(a, b)
            | Gate::Crz(a, b, _)
            | Gate::Crx(a, b, _)
            | Gate::Cry(a, b, _)
            | Gate::Cp(a, b, _) => vec![*a, *b],
            Gate::Ccx(a, b, c) | Gate::Ccz(a, b, c) => vec![*a, *b, *c],
        }
    }
}
