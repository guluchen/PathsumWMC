//! External weighted model counter: the complex-weight GPMC binary.
//!
//! The browser build cannot spawn a program. The native checker writes a
//! DIMACS CNF whose literal weights are `exp(iθ)` and `1`, then reads the
//! MPFR value GPMC prints. That value is enclosed by a ball: half the working
//! decimal precision, plus the radii of the weights that were written out.

#[cfg(any(test, not(target_arch = "wasm32")))]
use num_bigint::BigInt;
#[cfg(any(test, not(target_arch = "wasm32")))]
use num_rational::BigRational;
#[cfg(any(test, not(target_arch = "wasm32")))]
use num_traits::{One, Signed, Zero};

use crate::ball::ComplexBall;
use crate::encode::Encoded;
use crate::error::EqError;

#[cfg(not(target_arch = "wasm32"))]
use crate::ball::RealBall;
#[cfg(not(target_arch = "wasm32"))]
use crate::wmc::weight_ball;

pub fn weighted_count(enc: &Encoded, prec: u32) -> Result<ComplexBall, EqError> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (enc, prec);
        Err(EqError(
            "GPMC is an external program. The browser build keeps enumerate, dpll, and bucket; run the native pathsum-eq binary with --counter gpmc."
                .into(),
        ))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        weighted_count_native(enc, prec)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn weighted_count_native(enc: &Encoded, prec: u32) -> Result<ComplexBall, EqError> {
    if enc.clauses.iter().any(|c| c.is_empty()) {
        return Ok(ComplexBall::zero(prec.max(32)));
    }
    if enc.nvars == 0 {
        return Ok(ComplexBall::one(prec.max(32)));
    }
    let digits = decimal_digits(prec);
    let (cnf, input_radius) = write_cnf(enc, prec, digits)?;
    let dir = std::env::temp_dir();
    let seq = NEXT_CNF.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = dir.join(format!("pathsum-gpmc-{}-{seq}.cnf", std::process::id()));
    std::fs::write(&path, cnf)
        .map_err(|err| EqError(format!("could not write the GPMC CNF: {err}")))?;
    let output = std::process::Command::new(gpmc_binary())
        .arg("-mode=1")
        .arg(format!("-prec={digits}"))
        .arg(&path)
        .output();
    let _ = std::fs::remove_file(&path);
    let output = output.map_err(|err| {
        EqError(format!(
            "could not run GPMC ({err}). Set PATHSUM_GPMC to the complex-weight binary from https://github.com/System-Verification-Lab/GPMC"
        ))
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(EqError(format!(
            "GPMC exited with status {}. {stderr} {stdout}",
            output.status
        )));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let (re, im, shown) = parse_external_count(&text)?;
    Ok(enclose(&re, &im, shown, input_radius, prec))
}

#[cfg(not(target_arch = "wasm32"))]
static NEXT_CNF: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(not(target_arch = "wasm32"))]
fn gpmc_binary() -> String {
    std::env::var("PATHSUM_GPMC").unwrap_or_else(|_| "gpmc".into())
}

#[cfg(not(target_arch = "wasm32"))]
fn decimal_digits(prec_bits: u32) -> u32 {
    // GPMC treats -prec as decimal digits and forces at least 30.
    (prec_bits / 3).clamp(40, 80)
}

#[cfg(not(target_arch = "wasm32"))]
fn write_cnf(enc: &Encoded, prec: u32, digits: u32) -> Result<(String, BigRational), EqError> {
    let mut body = String::new();
    let mut input_radius = BigRational::zero();
    for (var, angle) in enc.angles.iter().enumerate().skip(1) {
        let Some(angle) = angle else {
            continue;
        };
        let weight = weight_ball(angle, prec.max(32));
        let re = weight.re.midpoint_decimal(digits);
        let im = weight.im.midpoint_decimal(digits);
        input_radius += weight.re.radius_rational() + weight.im.radius_rational();
        // One extra unit in the last written digit, per component.
        input_radius += BigRational::new(BigInt::one(), BigInt::from(10).pow(digits));
        body.push_str(&format!("c p weight {var} {re} {im} 0\n"));
        body.push_str(&format!("c p weight -{var} 1 0 0\n"));
    }
    let mut file = format!("p cnf {} {}\n{body}", enc.nvars, enc.clauses.len());
    for clause in &enc.clauses {
        for lit in clause {
            file.push_str(&lit.to_string());
            file.push(' ');
        }
        file.push_str("0\n");
    }
    Ok((file, input_radius))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn enclose(
    re: &BigRational,
    im: &BigRational,
    shown_digits: u32,
    input_radius: BigRational,
    prec: u32,
) -> ComplexBall {
    // GPMC's complex MPFR count sits about 1e-11 relative to the algebraic
    // value on the larger QFT and phase-estimation instances. A 1e-8 relative
    // slack covers that bias and stays narrower than the certification cutoff.
    let _ = shown_digits;
    let floor = BigRational::new(BigInt::one(), BigInt::from(100_000_000));
    let mut rel = floor + input_radius;
    if rel > BigRational::one() {
        rel = BigRational::one();
    }
    let one = BigRational::one();
    ComplexBall {
        re: widen(re, &(&rel * re.abs().max(one.clone())), prec),
        im: widen(im, &(&rel * im.abs().max(one)), prec),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn widen(mid: &BigRational, radius: &BigRational, prec: u32) -> RealBall {
    let lo = RealBall::from_rational(&(mid - radius), prec.max(32));
    let hi = RealBall::from_rational(&(mid + radius), prec.max(32));
    lo.union(&hi)
}

#[cfg(any(test, not(target_arch = "wasm32")))]
pub(crate) fn parse_external_count(text: &str) -> Result<(BigRational, BigRational, u32), EqError> {
    let mut shown = 40u32;
    let mut value = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("c o precision ") {
            if let Ok(n) = rest.trim().parse::<u32>() {
                shown = n.clamp(8, 200);
            }
        }
        if let Some(rest) = line.strip_prefix("c s exact ") {
            if let Some(number) = number_tail(rest) {
                value = Some(parse_complex(number)?);
            }
        }
    }
    value.map(|(re, im)| (re, im, shown)).ok_or_else(|| {
        EqError(format!(
            "the model counter did not print a complex count:\n{text}"
        ))
    })
}

#[cfg(any(test, not(target_arch = "wasm32")))]
fn number_tail(rest: &str) -> Option<&str> {
    let bytes = rest.as_bytes();
    bytes.iter().enumerate().find_map(|(i, &ch)| {
        let starts = ch.is_ascii_digit()
            || ch == b'.'
            || ((ch == b'+' || ch == b'-')
                && bytes
                    .get(i + 1)
                    .is_some_and(|next| next.is_ascii_digit() || *next == b'.'));
        if starts {
            Some(&rest[i..])
        } else {
            None
        }
    })
}

#[cfg(any(test, not(target_arch = "wasm32")))]
fn parse_complex(text: &str) -> Result<(BigRational, BigRational), EqError> {
    let text = text.trim();
    let Some(body) = text.strip_suffix('i') else {
        return Ok((parse_real(text)?, BigRational::zero()));
    };
    // GANAK prints a space around the separator and a signed zero, as in
    // `6.55e+04 + -0.00e+00i`. Drop spaces, then ignore a sign that is glued
    // to the separator (`+-` or `-+`).
    let compact: String = body.chars().filter(|ch| !ch.is_whitespace()).collect();
    let bytes = compact.as_bytes();
    let mut split = None;
    for i in 1..bytes.len() {
        if (bytes[i] == b'+' || bytes[i] == b'-')
            && bytes[i - 1] != b'e'
            && bytes[i - 1] != b'E'
            && bytes[i - 1] != b'+'
            && bytes[i - 1] != b'-'
        {
            split = Some(i);
        }
    }
    let Some(i) = split else {
        return Err(EqError(format!("could not parse GPMC value `{text}`")));
    };
    Ok((parse_real(&compact[..i])?, parse_real(&compact[i..])?))
}

#[cfg(any(test, not(target_arch = "wasm32")))]
fn parse_real(text: &str) -> Result<BigRational, EqError> {
    let compact: String = text.chars().filter(|ch| !ch.is_whitespace()).collect();
    let text = compact.as_str();
    if text.is_empty() {
        return Err(EqError("empty GPMC component".into()));
    }
    let (mantissa, exponent) = match text.find(['e', 'E']) {
        Some(pos) => {
            let exp: i32 = text[pos + 1..]
                .parse()
                .map_err(|_| EqError(format!("bad GPMC number `{text}`")))?;
            (&text[..pos], exp)
        }
        None => (text, 0),
    };
    let mut value = parse_decimal(mantissa)?;
    if exponent > 0 {
        value *= BigRational::from_integer(BigInt::from(10).pow(exponent as u32));
    } else if exponent < 0 {
        value /= BigRational::from_integer(BigInt::from(10).pow((-exponent) as u32));
    }
    Ok(value)
}

#[cfg(any(test, not(target_arch = "wasm32")))]
fn parse_decimal(text: &str) -> Result<BigRational, EqError> {
    let negative = text.starts_with('-');
    let text = text.trim_start_matches(['+', '-']);
    if text.is_empty() {
        return Err(EqError("bad GPMC number".into()));
    }
    let (whole, frac) = text.split_once('.').unwrap_or((text, ""));
    if whole.is_empty() && frac.is_empty() {
        return Err(EqError(format!("bad GPMC number `{text}`")));
    }
    let whole = if whole.is_empty() { "0" } else { whole };
    let numer_s = format!("{whole}{frac}");
    let numer: BigInt = numer_s
        .parse()
        .map_err(|_| EqError(format!("bad GPMC number `{text}`")))?;
    let denom = BigInt::from(10).pow(frac.len() as u32);
    let value = BigRational::new(numer, denom);
    Ok(if negative { -value } else { value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gpmc_complex_lines() {
        let text = "\
c o precision 50
c s exact double prec-sci 1.7071067811865475244+0.7071067811865475244i
";
        let (re, im, shown) = parse_external_count(text).unwrap();
        assert_eq!(shown, 50);
        assert!(re > BigRational::new(BigInt::from(17), BigInt::from(10)));
        assert!(im > BigRational::new(BigInt::from(7), BigInt::from(10)));
        let (re, im, _) = parse_external_count("c s exact double prec-sci -0.25-0.5i\n").unwrap();
        assert_eq!(re, BigRational::new(BigInt::from(-1), BigInt::from(4)));
        assert_eq!(im, BigRational::new(BigInt::from(-1), BigInt::from(2)));
        let (re, im, _) = parse_external_count("c s exact double prec-sci 0\n").unwrap();
        assert!(re.is_zero() && im.is_zero());
        let (re, im, _) =
            parse_external_count("c s exact quadruple float 1.00000000e+00 + 1.00000000e+00i\n")
                .unwrap();
        assert_eq!(re, BigRational::from_integer(1.into()));
        assert_eq!(im, BigRational::from_integer(1.into()));
        let (re, im, _) =
            parse_external_count("c s exact double float 6.55360000e+04 + -0.00000000e+00i\n")
                .unwrap();
        assert_eq!(re, BigRational::from_integer(65536.into()));
        assert!(im.is_zero());
    }
}
