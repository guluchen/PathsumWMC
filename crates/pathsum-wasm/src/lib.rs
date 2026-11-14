use pathsum_eq::{check, CheckRequest};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn install_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn check_equivalence(request_json: &str) -> String {
    let req: CheckRequest = match serde_json::from_str(request_json) {
        Ok(req) => req,
        Err(err) => {
            return serde_json::json!({
                "verdict": "error",
                "headline": "The request could not be read.",
                "detail": err.to_string(),
            })
            .to_string();
        }
    };
    match check(&req) {
        Ok(report) => serde_json::to_string(&report).unwrap_or_else(|err| {
            serde_json::json!({
                "verdict": "error",
                "headline": "The report could not be encoded.",
                "detail": err.to_string(),
            })
            .to_string()
        }),
        Err(err) => serde_json::json!({
            "verdict": "error",
            "headline": "The circuits could not be checked.",
            "detail": err.to_string(),
        })
        .to_string(),
    }
}

#[wasm_bindgen]
pub fn prepare_external(request_json:&str)->String {
    let result=serde_json::from_str::<CheckRequest>(request_json)
        .map_err(|e|e.to_string())
        .and_then(|r|pathsum_eq::prepare_external(&r).map_err(|e|e.to_string()));
    match result { Ok(v)=>v.to_string(),Err(e)=>serde_json::json!({"error":e}).to_string() }
}
#[wasm_bindgen]
pub fn finish_external(request_json:&str,stdout:&str,elapsed_ms:u32)->String {
    let result=serde_json::from_str::<CheckRequest>(request_json)
        .map_err(|e|e.to_string())
        .and_then(|r|pathsum_eq::finish_external(&r,stdout,elapsed_ms as u64).map_err(|e|e.to_string()));
    match result { Ok(v)=>serde_json::to_string(&v).unwrap(),Err(e)=>serde_json::json!({"error":e}).to_string() }
}
