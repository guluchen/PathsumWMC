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

pub(crate) fn cnf(enc: &Encoded, prec: u32, engine: &str) -> (Vec<String>, BigRational) {
    let one = BigRational::one();
    let two = BigRational::from_integer(2.into());
    let mut errors = Vec::new();
    let mut total_error = BigRational::zero();
    let mut weights=vec![(one.clone(),BigRational::zero());enc.nvars as usize+1];
    let mut odd=Vec::new();
    for var in 1..=enc.nvars {
        if let Some(angle)=&enc.angles[var as usize] {
            if let Some(k)=angle.quarter_index() {
                let w=crate::ExactAmp::exp_i_pi_quarters(&k);
                if !w.re1.is_zero() || !w.im1.is_zero() {
                    weights[var as usize]=(w.re1,w.im1);odd.push(var as i32);
                } else {weights[var as usize]=(w.re0,w.im0)}
            } else {
                let w=weight_ball(angle,prec);
                let (rl,rh)=w.re.rational_bounds();let (il,ih)=w.im.rational_bounds();
                let eps=(&rh-&rl+&ih-&il)/&two;
                total_error+=&eps;errors.push(eps);
                weights[var as usize]=((rl+rh)/&two,(il+ih)/&two);
            }
        }
    }
    let mut clauses=enc.clauses.clone();
    // Each odd pi/4 weight is alpha*sqrt(2). A deterministic parity/carry
    // chain factors sqrt(2)^k into 2^floor(k/2) and one final sqrt(2).
    // Two rational counts recover the exact Q(i,sqrt(2)) midpoint sum.
    let mut parity=odd.first().copied();
    for &v in odd.iter().skip(1) {
        let p=parity.unwrap();
        let carry=weights.len() as i32;weights.push((two.clone(),BigRational::zero()));
        clauses.extend([vec![-carry,p],vec![-carry,v],vec![carry,-p,-v]]);
        let next=weights.len() as i32;weights.push((one.clone(),BigRational::zero()));
        clauses.extend([vec![-p,-v,-next],vec![-p,v,next],vec![p,-v,next],vec![p,v,-next]]);
        parity=Some(next);
    }
    let nvars=weights.len()-1;
    let mut body=String::new();
    for (var,(re,im)) in weights.iter().enumerate().skip(1) {
        if engine=="gpmc" {body.push_str(&format!("c p weight {var} {re} {im} 0\nc p weight -{var} 1 0 0\n"))}
        else {let sign=if im<&BigRational::zero() {""} else {"+"};body.push_str(&format!("c p weight {var} {re}{sign}{im}i 0\nc p weight -{var} 1+0i 0\n"))}
    }
    for c in &clauses {for l in c {body.push_str(&format!("{l} "));}body.push_str("0\n")}
    let jobs=if let Some(p)=parity {
        [false,true].into_iter().map(|odd|format!("c t wmc\np cnf {nvars} {}\n{body}{} 0\n",clauses.len()+1,if odd {p} else {-p})).collect()
    } else {vec![format!("c t wmc\np cnf {nvars} {}\n{body}",clauses.len())]};
    // product(1+eps)-1 <= sum(eps)/(1-sum(eps)), when sum(eps)<1.
    let error=if total_error<one {&total_error/(&one-&total_error)}
        else {errors.into_iter().fold(one.clone(),|p,e|p*(&one+e))-one};
    let radius=error*(BigInt::one() << (enc.inputs+enc.path_vars) as usize);
    (jobs,radius)
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
        for src in ["p(1/7) q[0];", "h q[0]; p(1/7) q[0]; h q[0];", "h q[0]; s q[0]; cx q[0],q[1]; ry(1/6) q[1];"] {
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

    fn enumerate_job(text:&str)->Q {
        use crate::wmc::Ring;
        let mut p=crate::wmc::Problem {nvars:0,clauses:Vec::new(),w0:Vec::new(),w1:Vec::new()};
        for line in text.lines() {
            let t:Vec<_>=line.split_whitespace().collect();
            if t.first()==Some(&"p") {p.nvars=t[2].parse().unwrap();p.w0=vec![Q::one();p.nvars as usize+1];p.w1=p.w0.clone()}
            else if t.starts_with(&["c","p","weight"]) {
                let v:i32=t[3].parse().unwrap();let w=Q(rational(t[4]).unwrap(),rational(t[5]).unwrap());
                if v>0 {p.w1[v as usize]=w} else {p.w0[(-v) as usize]=w}
            } else if t.first()!=Some(&"c") {p.clauses.push(t[..t.len()-1].iter().map(|x|x.parse().unwrap()).collect())}
        }
        crate::wmc::dpll(&p,10_000_000).unwrap()
    }
    #[test] fn exact_sqrt2_extension_ladder() {
        for n in 0..=5 {
            let a=crate::parse_qasm(&format!("OPENQASM 2.0; qreg q[1]; {}", "h q[0]; t q[0];".repeat(n))).unwrap();
            let b=crate::parse_qasm("OPENQASM 2.0; qreg q[1];").unwrap();
            let ps=crate::PathSum::compose(&a,&b,false).unwrap();
            let enc=crate::encode::encode(&ps).unwrap();
            let (jobs,r)=cnf(&enc,64,"gpmc");assert!(r.is_zero());
            let c:Vec<_>=jobs.iter().map(|s|enumerate_job(s)).collect();
            let actual=crate::ExactAmp {re0:c[0].0.clone(),im0:c[0].1.clone(),re1:c.get(1).map(|z|z.0.clone()).unwrap_or_default(),im1:c.get(1).map(|z|z.1.clone()).unwrap_or_default()};
            let expected=crate::wmc::dpll(&crate::wmc::problem_exact(&enc).unwrap(),10_000_000).unwrap();
            assert_eq!(actual,expected,"sqrt2 reconstruction failed at ladder {n}");
        }
    }

}
