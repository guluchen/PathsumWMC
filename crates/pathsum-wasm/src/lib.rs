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
