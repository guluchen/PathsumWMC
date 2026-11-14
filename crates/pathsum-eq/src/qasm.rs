//! A small OpenQASM 2 parser.
//!
//! Angles are exact rationals. `0.3` is `3/10`, and `pi/3` is the rational
//! multiple `1/3` of π.

use num_bigint::{BigInt, Sign};
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

use crate::circuit::{Circuit, Gate};
use crate::error::EqError;
use crate::phase::Angle;

pub fn parse_qasm(src: &str) -> Result<Circuit, EqError> {
    let cleaned = strip_comments(src);
    let mut regs: Vec<(String, usize, usize)> = Vec::new();
    let mut next_q = 0usize;
    let mut gates = Vec::new();
    let mut saw_reg = false;

    for raw in cleaned.split(';') {
        let stmt = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }
        let lower = stmt.to_ascii_lowercase();
        if lower.starts_with("openqasm") || lower.starts_with("include") {
            continue;
        }
        if lower.starts_with("creg ") || lower.starts_with("barrier") {
            continue;
        }
        if lower.starts_with("measure ") || lower.starts_with("reset ") {
            return Err(EqError(
                "measurement and reset are not unitary operations; remove them to compare the circuit unitary".into(),
            ));
        }
        if lower.starts_with("qreg ") || lower.starts_with("qubit[") {
            let (name, n) = parse_reg(&lower)?;
            regs.push((name, next_q, n));
            next_q += n;
            saw_reg = true;
            continue;
        }
        if lower.starts_with("gate ") {
            return Err(EqError(
                "custom gate definitions are not supported; inline the gate body".into(),
            ));
        }
        if !saw_reg {
            return Err(EqError(
                "declare a quantum register with `qreg q[n];` before the gates".into(),
            ));
        }
        gates.push(parse_gate(&lower, &regs)?);
    }

    if !saw_reg {
        return Err(EqError(
            "no quantum register found; start with `qreg q[n];`".into(),
        ));
    }
    Ok(Circuit {
        qubits: next_q,
        gates,
    })
}

fn parse_reg(stmt: &str) -> Result<(String, usize), EqError> {
    if let Some(rest) = stmt.strip_prefix("qreg ") {
        let (name, n) = split_name_index(rest.trim())?;
        if n == 0 {
            return Err(EqError(
                "a quantum register needs a name and a positive size".into(),
            ));
        }
        return Ok((name, n));
    }
    if let Some(rest) = stmt.strip_prefix("qubit[") {
        let (n_txt, name) = rest
            .split_once(']')
            .ok_or_else(|| EqError("malformed qubit declaration".into()))?;
        let n: usize = n_txt
            .trim()
            .parse()
            .map_err(|_| EqError(format!("bad register size `{n_txt}`")))?;
        let name = name.trim().to_string();
        if name.is_empty() || n == 0 {
            return Err(EqError(
                "a quantum register needs a name and a positive size".into(),
            ));
        }
        return Ok((name, n));
    }
    Err(EqError(format!("unrecognized register `{stmt}`")))
}

fn split_name_index(text: &str) -> Result<(String, usize), EqError> {
    let (name, rest) = text
        .split_once('[')
        .ok_or_else(|| EqError(format!("expected name[n], found `{text}`")))?;
    let n_txt = rest.trim().trim_end_matches(']');
    let n: usize = n_txt
        .parse()
        .map_err(|_| EqError(format!("bad index `{n_txt}`")))?;
    if name.trim().is_empty() {
        return Err(EqError(format!("missing register name in `{text}`")));
    }
    Ok((name.trim().to_string(), n))
}

fn parse_gate(stmt: &str, regs: &[(String, usize, usize)]) -> Result<Gate, EqError> {
    let (name, angle, rest) = if let Some(open) = stmt.find('(') {
        let close = stmt
            .find(')')
            .ok_or_else(|| EqError(format!("missing `)` in `{stmt}`")))?;
        let name = stmt[..open].trim().to_string();
        let angle = parse_angle(stmt[open + 1..close].trim())?;
        let rest = stmt[close + 1..].trim().to_string();
        (name, Some(angle), rest)
    } else {
        let mut parts = stmt.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or("").trim().to_string();
        let rest = parts.next().unwrap_or("").trim().to_string();
        (name, None, rest)
    };
    let qubits = parse_qubits(&rest, regs)?;
    let q = |i: usize| -> Result<usize, EqError> {
        qubits
            .get(i)
            .copied()
            .ok_or_else(|| EqError(format!("`{name}` expects more qubit arguments")))
    };
    let ang = || -> Result<Angle, EqError> {
        angle
            .clone()
            .ok_or_else(|| EqError(format!("gate `{name}` needs an angle")))
    };
    match name.as_str() {
        "id" | "iden" => Ok(Gate::P(q(0)?, Angle::zero())),
        "h" => Ok(Gate::H(q(0)?)),
        "x" => Ok(Gate::X(q(0)?)),
        "y" => Ok(Gate::Y(q(0)?)),
        "z" => Ok(Gate::Z(q(0)?)),
        "s" => Ok(Gate::S(q(0)?)),
        "sdg" => Ok(Gate::Sdg(q(0)?)),
        "t" => Ok(Gate::T(q(0)?)),
        "tdg" => Ok(Gate::Tdg(q(0)?)),
        "cx" | "cnot" => Ok(Gate::Cx(q(0)?, q(1)?)),
        "cz" => Ok(Gate::Cz(q(0)?, q(1)?)),
        "swap" => Ok(Gate::Swap(q(0)?, q(1)?)),
        "ccx" | "toffoli" => Ok(Gate::Ccx(q(0)?, q(1)?, q(2)?)),
        "ccz" => Ok(Gate::Ccz(q(0)?, q(1)?, q(2)?)),
        "rz" => Ok(Gate::Rz(q(0)?, ang()?)),
        "rx" => Ok(Gate::Rx(q(0)?, ang()?)),
        "ry" => Ok(Gate::Ry(q(0)?, ang()?)),
        "p" | "u1" | "phase" => Ok(Gate::P(q(0)?, ang()?)),
        "crz" => Ok(Gate::Crz(q(0)?, q(1)?, ang()?)),
        "crx" => Ok(Gate::Crx(q(0)?, q(1)?, ang()?)),
        "cry" => Ok(Gate::Cry(q(0)?, q(1)?, ang()?)),
        "cp" | "cphase" | "cu1" => Ok(Gate::Cp(q(0)?, q(1)?, ang()?)),
        other => Err(EqError(format!(
            "unsupported gate `{other}`; supported gates are h, x, y, z, s, sdg, t, tdg, cx, cz, swap, ccx, ccz, rx, ry, rz, p, crx, cry, crz, cp"
        ))),
    }
}

fn parse_qubits(rest: &str, regs: &[(String, usize, usize)]) -> Result<Vec<usize>, EqError> {
    if rest.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for part in rest.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (name, idx) = split_name_index(part)?;
        let (_, base, len) = regs
            .iter()
            .find(|(n, _, _)| n == &name)
            .ok_or_else(|| EqError(format!("unknown register `{name}`")))?;
        if idx >= *len {
            return Err(EqError(format!(
                "qubit {name}[{idx}] is outside a register of length {len}"
            )));
        }
        out.push(base + idx);
    }
    Ok(out)
}

pub fn parse_angle(expr: &str) -> Result<Angle, EqError> {
    let tokens = lex_angle(expr)?;
    let mut parser = AngleParser { tokens, i: 0 };
    let angle = parser.parse_expr()?;
    if parser.i != parser.tokens.len() {
        return Err(EqError(format!("trailing tokens in angle `{expr}`")));
    }
    // Do not reduce mod 2π here. Rz(θ+2π) = −Rz(θ), so π and −π are different gates.
    Ok(angle)
}

#[derive(Clone, Debug)]
enum Tok {
    Num(BigRational),
    Pi,
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

struct AngleParser {
    tokens: Vec<Tok>,
    i: usize,
}

impl AngleParser {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.i)
    }

    fn bump(&mut self) -> Option<Tok> {
        let t = self.tokens.get(self.i).cloned();
        if t.is_some() {
            self.i += 1;
        }
        t
    }

    fn parse_expr(&mut self) -> Result<Angle, EqError> {
        let mut acc = self.parse_term()?;
        while matches!(self.peek(), Some(Tok::Plus | Tok::Minus)) {
            let minus = matches!(self.bump(), Some(Tok::Minus));
            let rhs = self.parse_term()?;
            acc = if minus {
                acc.add(&rhs.neg())
            } else {
                acc.add(&rhs)
            };
        }
        Ok(acc)
    }

    fn parse_term(&mut self) -> Result<Angle, EqError> {
        let mut acc = self.parse_factor()?;
        while matches!(self.peek(), Some(Tok::Star | Tok::Slash)) {
            let div = matches!(self.bump(), Some(Tok::Slash));
            let rhs = self.parse_factor()?;
            acc = combine_product(&acc, &rhs, div)?;
        }
        Ok(acc)
    }

    fn parse_factor(&mut self) -> Result<Angle, EqError> {
        match self.bump() {
            Some(Tok::Plus) => self.parse_factor(),
            Some(Tok::Minus) => Ok(self.parse_factor()?.neg()),
            Some(Tok::Num(n)) => Ok(Angle::from_rad(n)),
            Some(Tok::Pi) => Ok(Angle::from_pi(BigRational::one())),
            Some(Tok::LParen) => {
                let v = self.parse_expr()?;
                match self.bump() {
                    Some(Tok::RParen) => Ok(v),
                    _ => Err(EqError("missing `)` in an angle".into())),
                }
            }
            _ => Err(EqError(
                "expected a number, `pi`, or `(` in an angle".into(),
            )),
        }
    }
}

fn pure_rational(angle: &Angle) -> Option<BigRational> {
    if angle.pi.is_zero() {
        Some(angle.rad.clone())
    } else {
        None
    }
}

fn combine_product(a: &Angle, b: &Angle, div: bool) -> Result<Angle, EqError> {
    if div {
        let k = pure_rational(b).ok_or_else(|| {
            EqError("only a rational number can divide an angle; write `pi/2`".into())
        })?;
        if k.is_zero() {
            return Err(EqError("division by zero in an angle".into()));
        }
        return Ok(a.scale(&k.recip()));
    }
    if let Some(k) = pure_rational(b) {
        return Ok(a.scale(&k));
    }
    if let Some(k) = pure_rational(a) {
        return Ok(b.scale(&k));
    }
    Err(EqError(
        "angle products may only scale by a rational; use `pi/2` or `0.3*pi`".into(),
    ))
}

fn lex_angle(expr: &str) -> Result<Vec<Tok>, EqError> {
    let b = expr.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i] as char;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '+' => out.push(Tok::Plus),
            '-' => out.push(Tok::Minus),
            '*' => out.push(Tok::Star),
            '/' => out.push(Tok::Slash),
            '(' => out.push(Tok::LParen),
            ')' => out.push(Tok::RParen),
            _ if c.is_ascii_digit() || c == '.' => {
                let start = i;
                i += 1;
                while i < b.len() && ((b[i] as char).is_ascii_digit() || b[i] == b'.') {
                    i += 1;
                }
                if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
                    let mut j = i + 1;
                    if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
                        j += 1;
                    }
                    let digits = j;
                    while j < b.len() && (b[j] as char).is_ascii_digit() {
                        j += 1;
                    }
                    if j > digits {
                        i = j;
                    }
                }
                out.push(Tok::Num(parse_decimal(&expr[start..i])?));
                continue;
            }
            _ if c.is_ascii_alphabetic() => {
                let start = i;
                i += 1;
                while i < b.len() && (b[i] as char).is_ascii_alphabetic() {
                    i += 1;
                }
                let word = expr[start..i].to_ascii_lowercase();
                if word == "pi" {
                    out.push(Tok::Pi);
                } else {
                    return Err(EqError(format!(
                        "unknown name `{word}` in an angle; use numbers and `pi`"
                    )));
                }
                continue;
            }
            _ => return Err(EqError(format!("unexpected character `{c}` in an angle"))),
        }
        i += 1;
    }
    Ok(out)
}

fn parse_decimal(text: &str) -> Result<BigRational, EqError> {
    if text.is_empty() || text == "." {
        return Err(EqError(format!("bad number `{text}`")));
    }
    let (mantissa, exponent) = match text.find(['e', 'E']) {
        Some(pos) => {
            let exp: i32 = text[pos + 1..]
                .parse()
                .map_err(|_| EqError(format!("bad number `{text}`")))?;
            (&text[..pos], exp)
        }
        None => (text, 0),
    };
    let mut value = parse_plain_decimal(mantissa)?;
    if exponent > 0 {
        value *= BigRational::from_integer(BigInt::from(10).pow(exponent as u32));
    } else if exponent < 0 {
        value /= BigRational::from_integer(BigInt::from(10).pow((-exponent) as u32));
    }
    Ok(value)
}

fn parse_plain_decimal(text: &str) -> Result<BigRational, EqError> {
    if text.is_empty() || text == "." {
        return Err(EqError(format!("bad number `{text}`")));
    }
    if let Some((whole, frac)) = text.split_once('.') {
        if frac.contains('.') {
            return Err(EqError(format!("bad number `{text}`")));
        }
        let whole_int = if whole.is_empty() {
            BigInt::zero()
        } else {
            whole
                .parse::<BigInt>()
                .map_err(|_| EqError(format!("bad number `{text}`")))?
        };
        let frac_val = if frac.is_empty() {
            BigInt::zero()
        } else {
            frac.parse::<BigInt>()
                .map_err(|_| EqError(format!("bad number `{text}`")))?
        };
        let den = BigInt::from(10).pow(frac.len() as u32);
        let num = whole_int.abs() * &den + frac_val;
        let num = if whole_int.sign() == Sign::Minus {
            -num
        } else {
            num
        };
        Ok(BigRational::new(num, den))
    } else {
        let n = text
            .parse::<BigInt>()
            .map_err(|_| EqError(format!("bad number `{text}`")))?;
        Ok(BigRational::from_integer(n))
    }
}

fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'/') {
            for d in chars.by_ref() {
                if d == '\n' {
                    out.push('\n');
                    break;
                }
            }
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(d) = chars.next() {
                if d == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pi_is_not_folded_to_minus_pi() {
        let a = parse_angle("pi").unwrap();
        assert_eq!(a.pi, BigRational::one());
        let b = parse_angle("-pi").unwrap();
        assert_eq!(b.pi, -BigRational::one());
    }

    #[test]
    fn parses_pi_over_two() {
        let a = parse_angle("pi/2").unwrap();
        assert!(a.rad.is_zero());
        assert_eq!(a.pi, BigRational::new(BigInt::one(), BigInt::from(2)));
    }

    #[test]
    fn parses_scientific_notation() {
        let a = parse_angle("-3.5e-6").unwrap();
        assert!(a.pi.is_zero());
        assert_eq!(
            a.rad,
            BigRational::new(BigInt::from(-35), BigInt::from(10_000_000))
        );
    }

    #[test]
    fn parses_decimal_times_pi() {
        let a = parse_angle("0.3*pi").unwrap();
        assert!(a.rad.is_zero());
        assert_eq!(a.pi, BigRational::new(BigInt::from(3), BigInt::from(10)));
    }
}
