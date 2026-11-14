//! Certified adapter for exact rational complex model counters.
//!
//! A midpoint approximation m_j to each unit-modulus weight w_j has
//! |m_j-w_j| <= eps_j. Telescoping products give an error no larger than
//! product(1+eps_j)-1 per model. The CNF's Tseitin variables are uniquely
//! determined by the input and path bits, so there are at most 2^(n+p)
//! models. No assumption about cancellation or the counter's rounding is used:
//! the external counter must return an EXACT rational complex sum.
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};
use crate::{ball::ComplexBall, encode::Encoded, error::EqError, wmc::weight_ball};

pub(crate) fn cnf(enc: &Encoded, prec: u32, engine: &str) -> (String, BigRational) {
    let one = BigRational::one();
    let two = BigRational::from_integer(2.into());
    let mut errors = Vec::new();
    let mut total_error = BigRational::zero();
    let mut text = format!("c t wmc\np cnf {} {}\n",enc.nvars,enc.clauses.len());
    for var in 1..=enc.nvars {
        let (re,im) = match &enc.angles[var as usize] {
            Some(angle) => {
                let w = weight_ball(angle,prec);
                let (rl,rh)=w.re.rational_bounds();
                let (il,ih)=w.im.rational_bounds();
                let eps=(&rh-&rl+&ih-&il)/&two;
                total_error += &eps;
                errors.push(eps);
                ((rl+rh)/&two,(il+ih)/&two)
            },
            None => (one.clone(),BigRational::zero()),
        };
        if engine=="gpmc" {
            text.push_str(&format!("c p weight {var} {re} {im} 0\nc p weight -{var} 1 0 0\n"));
        } else {
            let sign=if im < BigRational::zero() {""} else {"+"};
            text.push_str(&format!("c p weight {var} {re}{sign}{im}i 0\nc p weight -{var} 1+0i 0\n"));
        }
    }
    for c in &enc.clauses {
        for l in c { text.push_str(&format!("{l} ")); }
        text.push_str("0\n");
    }
    // For sum(eps)<1, product(1+eps)-1 <= sum(eps)/(1-sum(eps)).
    // This avoids enormous rational denominators on large circuits.
    let error=if total_error<one {
        &total_error/(&one-&total_error)
    } else {
        errors.into_iter().fold(one.clone(),|p,e|p*(&one+e))-one
    };
    let radius=error*(BigInt::one() << (enc.inputs+enc.path_vars) as usize);
    (text,radius)
}
pub(crate) fn count(text:&str)->Result<(BigRational,BigRational),EqError> {
    let mut values=text.lines().filter_map(|s|s.trim().strip_prefix("c s exact arb frac "));
    let value=values.next().ok_or_else(||EqError("The counter did not return an exact rational count.".into()))?;
    if values.next().is_some() { return Err(EqError("Ambiguous multiple counter results.".into())); }
    let compact:String=value.chars().filter(|c|!c.is_whitespace()).collect();
    let Some(body)=compact.strip_suffix('i') else { return Ok((rational(&compact)?,BigRational::zero())); };
    let bytes=body.as_bytes();
    let split=(1..bytes.len()).find(|&i|(bytes[i]==b'+' || bytes[i]==b'-') && bytes[i-1]!=b'+' && bytes[i-1]!=b'-')
        .ok_or_else(||EqError("Malformed exact complex count.".into()))?;
    let im=body[split..].strip_prefix('+').unwrap_or(&body[split..]);
    Ok((rational(&body[..split])?,rational(im)?))
}
fn rational(s:&str)->Result<BigRational,EqError> {
    // Fraction-only parsing: scientific/floating estimates must never be
    // accepted as the counter's exact-arithmetic result.
    if s.len()>2_000_000 { return Err(EqError("Counter result is too large.".into())); }
    let (n,d)=s.split_once('/').unwrap_or((s,"1"));
    let n:BigInt=n.parse().map_err(|_|EqError("Invalid rational numerator.".into()))?;
    let d:BigInt=d.parse().map_err(|_|EqError("Invalid rational denominator.".into()))?;
    if d<=BigInt::zero() { return Err(EqError("Nonpositive rational denominator.".into())); }
    Ok(BigRational::new(n,d))
}
pub(crate) fn ball(re:&BigRational,im:&BigRational,r:&BigRational,p:u32)->ComplexBall {
    ComplexBall {
        re:crate::RealBall::span_rationals(&(re-r),&(re+r),p),
        im:crate::RealBall::span_rationals(&(im-r),&(im+r),p),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn exact_only_parser() {
        let (r,i)=count("c s exact arb frac -3/7 + -2/9i").unwrap();
        assert_eq!(r,BigRational::new((-3).into(),7.into()));
        assert_eq!(i,BigRational::new((-2).into(),9.into()));
        for s in ["c s exact quadruple float 1+0i","c s exact arb frac 1e0+0i","c s exact arb frac 1/0+0i","c s exact arb frac 1+0i\nc s exact arb frac 2+0i"] { assert!(count(s).is_err(),"{s}"); }
    }
    #[test] fn unique_tseitin_extension_and_bound() {
        // Exhaustively check uniqueness on a packed-parity + conjunction CNF,
        // and compare the bound to a direct rigorous weighted enumeration.
        for source in ["h q[0]; t q[0]; cx q[0],q[1]; h q[1];", "h q[0]; p(1/7) q[0]; ccx q[0],q[1],q[2];"] {
            let q=if source.contains("ccx") {3} else {2};
            let left=crate::parse_qasm(&format!("OPENQASM 2.0; qreg q[{q}]; {source}")).unwrap();
            let right=crate::parse_qasm(&format!("OPENQASM 2.0; qreg q[{q}];")).unwrap();
            let ps=crate::PathSum::compose(&left,&right,false).unwrap();
            let enc=crate::encode::encode(&ps).unwrap();
            assert!(enc.nvars<24);
            let free=enc.inputs+enc.path_vars;
            let mut extensions=vec![0;1usize<<free];
            for bits in 0usize..1usize<<enc.nvars {
                if enc.clauses.iter().all(|c|c.iter().any(|&l| ((bits>>(l.unsigned_abs()-1))&1==1)==(l>0))) {
                    extensions[bits&((1usize<<free)-1)]+=1;
                }
            }
            assert!(extensions.iter().all(|&n|n<=1));
            let (_,r)=cnf(&enc,64,"gpmc"); assert!(r>=BigRational::zero());
            let (_,r2)=cnf(&enc,128,"ganak"); assert!(r2<=r);
        }
    }
    #[derive(Clone)]
    struct Q(BigRational,BigRational);
    impl crate::wmc::Ring for Q {
        fn zero()->Self {Self(BigRational::zero(),BigRational::zero())}
        fn one()->Self {Self(BigRational::one(),BigRational::zero())}
        fn add(&self,b:&Self)->Self {Self(&self.0+&b.0,&self.1+&b.1)}
        fn mul(&self,b:&Self)->Self {Self(&self.0*&b.0-&self.1*&b.1,&self.0*&b.1+&self.1*&b.0)}
    }
    #[test] fn certified_error_bound_contains_high_precision_reference() {
        use crate::wmc::Ring;
        for src in ["p(1/7) q[0];", "h q[0]; p(1/7) q[0]; h q[0];", "h q[0]; t q[0]; cx q[0],q[1]; ry(1/6) q[1];"] {
            let a=crate::parse_qasm(&format!("OPENQASM 2.0; qreg q[2]; {src}")).unwrap();
            let b=crate::parse_qasm("OPENQASM 2.0; qreg q[2];").unwrap();
            let ps=crate::PathSum::compose(&a,&b,false).unwrap();
            let enc=crate::encode::encode(&ps).unwrap();
            let (_,r)=cnf(&enc,64,"gpmc");
            let mut p=crate::wmc::Problem {nvars:enc.nvars,clauses:enc.clauses.clone(),w0:vec![Q::one();enc.nvars as usize+1],w1:vec![Q::one();enc.nvars as usize+1]};
            for (v,a) in enc.angles.iter().enumerate().skip(1) {
                if let Some(a)=a {
                    let w=weight_ball(a,64);
                    let (l,h)=w.re.rational_bounds();let re=(l+h)/BigInt::from(2);
                    let (l,h)=w.im.rational_bounds();let im=(l+h)/BigInt::from(2);
                    p.w1[v]=Q(re,im);
                }
            }
            let mid=crate::wmc::enumerate(&p,1_000_000).unwrap();
            let enclosure=ball(&mid.0,&mid.1,&r,64);
            let reference=crate::wmc::enumerate(&crate::wmc::problem_ball(&enc,256),1_000_000).unwrap();
            for (outer,inner) in [(&enclosure.re,&reference.re),(&enclosure.im,&reference.im)] {
                let (l,h)=outer.rational_bounds();let (rl,rh)=inner.rational_bounds();
                assert!(l<=rl && h>=rh,"insufficient weight-error enclosure");
            }
        }
    }

}
