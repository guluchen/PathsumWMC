use std::fmt;

#[derive(Debug, Clone)]
pub struct EqError(pub String);

impl fmt::Display for EqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EqError {}
