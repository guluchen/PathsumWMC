//! Benchmark-only native frontend for the same certified external API as the web.
//! This deliberately does not use the legacy floating-point CLI integration.
use pathsum_eq::{PreparedExternal, CheckRequest};
use serde_json::{json,Value};
use std::{env,fs,io::{self,Read},process::Command,time::Instant};
fn run()->Result<Value,Box<dyn std::error::Error>> {
    let binary=env::args().nth(1).ok_or("Expected the frozen GPMC adapter path")?;
    let mut input=String::new();io::stdin().read_to_string(&mut input)?;
    let req:CheckRequest=serde_json::from_str(&input)?;
    let start=Instant::now();
    let prepared=PreparedExternal::new(&req)?;
    let job=prepared.job();
    if let Some(report)=job.get("report") {return Ok(json!({"report":report,"request_ms":start.elapsed().as_secs_f64()*1000.0}));}
    let dir=env::temp_dir().join(format!("pathsum-native-compare-{}",std::process::id()));
    fs::create_dir(&dir)?;
    let result=(||->Result<Value,Box<dyn std::error::Error>> {
        let mut outputs=Vec::new();let mut solve_ms=0.0;
        for (i,cnf) in job["cnfs"].as_array().ok_or("Missing CNFs")?.iter().enumerate() {
            let file=dir.join(format!("{i}.cnf"));fs::write(&file,cnf.as_str().ok_or("Invalid CNF")?)?;
            let tick=Instant::now();let output=Command::new(&binary).args(["-mode=1","-cs=64"]).arg(&file).output()?;
            solve_ms+=tick.elapsed().as_secs_f64()*1000.0;
            if !output.status.success(){return Err(format!("Native counter failed {}: {}",output.status,String::from_utf8_lossy(&output.stderr)).into());}
            outputs.push(String::from_utf8(output.stdout)?);
        }
        let report=prepared.finish(&serde_json::to_string(&outputs)?,solve_ms.ceil() as u64)?;
        Ok(json!({"report":report,"request_ms":start.elapsed().as_secs_f64()*1000.0,"counter_ms":solve_ms}))
    })();
    let _=fs::remove_dir_all(&dir);result
}
fn main(){
    match run(){Ok(value)=>println!("{}",value),Err(e)=>{println!("{}",json!({"error":e.to_string()}));std::process::exit(1);}}
}
