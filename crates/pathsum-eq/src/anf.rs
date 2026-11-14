//! Algebraic normal form over GF(2): an XOR of AND-monomials.

use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mono(pub Vec<u16>);

impl Mono {
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    pub fn var(v: u16) -> Self {
        Self(vec![v])
    }

    pub fn contains(&self, v: u16) -> bool {
        self.0.binary_search(&v).is_ok()
    }

    pub fn without(&self, v: u16) -> Self {
        let mut xs = self.0.clone();
        if let Ok(i) = xs.binary_search(&v) {
            xs.remove(i);
        }
        Self(xs)
    }

    pub fn union(&self, other: &Self) -> Self {
        let mut out = Vec::with_capacity(self.0.len() + other.0.len());
        let mut i = 0;
        let mut j = 0;
        while i < self.0.len() && j < other.0.len() {
            match self.0[i].cmp(&other.0[j]) {
                std::cmp::Ordering::Less => {
                    out.push(self.0[i]);
                    i += 1;
                }
                std::cmp::Ordering::Greater => {
                    out.push(other.0[j]);
                    j += 1;
                }
                std::cmp::Ordering::Equal => {
                    out.push(self.0[i]);
                    i += 1;
                    j += 1;
                }
            }
        }
        out.extend_from_slice(&self.0[i..]);
        out.extend_from_slice(&other.0[j..]);
        Self(out)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn degree(&self) -> usize {
        self.0.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Anf {
    pub terms: BTreeSet<Mono>,
}

impl Anf {
    pub fn zero() -> Self {
        Self {
            terms: BTreeSet::new(),
        }
    }

    pub fn one() -> Self {
        let mut terms = BTreeSet::new();
        terms.insert(Mono::empty());
        Self { terms }
    }

    pub fn var(v: u16) -> Self {
        let mut terms = BTreeSet::new();
        terms.insert(Mono::var(v));
        Self { terms }
    }

    pub fn contains_var(&self, v: u16) -> bool {
        self.terms.iter().any(|m| m.contains(v))
    }

    pub fn xor(&self, other: &Self) -> Self {
        let mut terms = self.terms.clone();
        for t in &other.terms {
            if !terms.remove(t) {
                terms.insert(t.clone());
            }
        }
        Self { terms }
    }

    pub fn and(&self, other: &Self) -> Self {
        let mut acc = Self::zero();
        for a in &self.terms {
            for b in &other.terms {
                acc = acc.xor(&Self::from_mono(a.union(b)));
            }
        }
        acc
    }

    pub fn not(&self) -> Self {
        self.xor(&Self::one())
    }

    pub fn from_mono(m: Mono) -> Self {
        let mut terms = BTreeSet::new();
        terms.insert(m);
        Self { terms }
    }

    pub fn xor_mono(&mut self, m: Mono) {
        if !self.terms.remove(&m) {
            self.terms.insert(m);
        }
    }

    /// Replace `var` by `repl` over GF(2).
    pub fn substitute(&self, var: u16, repl: &Self) -> Self {
        let mut acc = Self::zero();
        for term in &self.terms {
            if term.contains(var) {
                let mut piece = repl.clone();
                for v in &term.without(var).0 {
                    piece = piece.and(&Self::var(*v));
                }
                acc = acc.xor(&piece);
            } else {
                acc = acc.xor(&Self::from_mono(term.clone()));
            }
        }
        acc
    }

    pub fn eval(&self, env: &dyn Fn(u16) -> bool) -> bool {
        let mut acc = false;
        for term in &self.terms {
            let mut v = true;
            for x in &term.0 {
                if !env(*x) {
                    v = false;
                    break;
                }
            }
            if v {
                acc ^= true;
            }
        }
        acc
    }

    pub fn linear_path_pivot(&self, path_vars: &[u16]) -> Option<u16> {
        self.terms.iter().find_map(|m| {
            if m.degree() == 1 {
                let v = m.0[0];
                if path_vars.contains(&v) {
                    Some(v)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    pub fn without_linear(&self, v: u16) -> Self {
        let mut t = self.clone();
        t.xor_mono(Mono::var(v));
        t
    }
}
