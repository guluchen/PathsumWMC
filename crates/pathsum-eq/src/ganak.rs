//! External weighted model counter: GANAK in complex MPFR mode.
//!
//! GANAK 2.x counts over the complex floats with `--mode 6`. The native
//! checker writes both literal polarities, disables probabilistic caching,
//! and encloses the printed value in the same ball used for GPMC.

use crate::ball::ComplexBall;
use crate::encode::Encoded;
use crate::error::EqError;

#[cfg(not(target_arch = "wasm32"))]
use crate::gpmc::{enclose, parse_external_count};
#[cfg(not(target_arch = "wasm32"))]
use crate::wmc::weight_ball;
#[cfg(not(target_arch = "wasm32"))]
use num_bigint::BigInt;
#[cfg(not(target_arch = "wasm32"))]
use num_rational::BigRational;
#[cfg(not(target_arch = "wasm32"))]
use num_traits::{One, Zero};

pub fn weighted_count(enc: &Encoded, prec: u32) -> Result<ComplexBall, EqError> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (enc, prec);
        Err(EqError(
            "GANAK is an external program. The browser build keeps the built-in counters; run the native pathsum-eq binary with --counter ganak."
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
    let digits = (prec / 3).clamp(40, 80);
    let (cnf, input_radius) = write_cnf(enc, prec, digits)?;
    let seq = NEXT_CNF.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("pathsum-ganak-{}-{seq}.cnf", std::process::id()));
    std::fs::write(&path, cnf)
        .map_err(|err| EqError(format!("could not write the GANAK CNF: {err}")))?;
    let bits = prec.clamp(64, 256);
    let output = std::process::Command::new(ganak_binary())
        .arg("--verb")
        .arg("0")
        .arg("--mode")
        .arg("6")
        .arg("--prob")
        .arg("0")
        .arg("--mpfrprec")
        .arg(bits.to_string())
        .arg(&path)
        .output();
    let _ = std::fs::remove_file(&path);
    let output = output.map_err(|err| {
        EqError(format!(
            "could not run GANAK ({err}). Set PATHSUM_GANAK to the ganak binary from https://github.com/meelgroup/ganak"
        ))
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(EqError(format!(
            "GANAK exited with status {}. {stderr} {stdout}",
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
fn ganak_binary() -> String {
    std::env::var("PATHSUM_GANAK").unwrap_or_else(|_| "ganak".into())
}

#[cfg(not(target_arch = "wasm32"))]
fn write_cnf(enc: &Encoded, prec: u32, digits: u32) -> Result<(String, BigRational), EqError> {
    let mut body = String::new();
    let mut input_radius = BigRational::zero();
    for var in 1..=enc.nvars {
        let angle = enc.angles.get(var as usize).and_then(|a| a.as_ref());
        let (re, im, radius) = match angle {
            Some(angle) => {
                let weight = weight_ball(angle, prec.max(32));
                let radius = weight.re.radius_rational()
                    + weight.im.radius_rational()
                    + BigRational::new(BigInt::one(), BigInt::from(10).pow(digits));
                (
                    weight.re.midpoint_decimal(digits),
                    weight.im.midpoint_decimal(digits),
                    radius,
                )
            }
            None => ("1".to_string(), "0".to_string(), BigRational::zero()),
        };
        input_radius += radius;
        let token = complex_token(&re, &im);
        body.push_str(&format!("c p weight {var} {token} 0\n"));
        body.push_str(&format!("c p weight -{var} 1+0i 0\n"));
    }
    let mut file = format!("c t wmc\np cnf {} {}\n{body}", enc.nvars, enc.clauses.len());
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
fn complex_token(re: &str, im: &str) -> String {
    if im.starts_with('-') {
        format!("{re}{im}i")
    } else {
        format!("{re}+{im}i")
    }
}
