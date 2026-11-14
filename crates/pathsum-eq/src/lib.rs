//! Equivalence checking of quantum circuits by path-sum reduction and
//! weighted model counting.
//!
//! The decision procedure follows Huang, Chareton, Chen, Chung, Hsieh,
//! Laarman, and Mei, “Equivalence Checking of Quantum Circuits via Path-Sum
//! and Weighted Model Counting” (arXiv:2604.24504).
//!
//! Rotations whose angles are multiples of π/4 are decided in the ring
//! ℚ(i, √2). Every other angle is enclosed by complex ball arithmetic, so a
//! floating-point rounding error cannot by itself produce an equivalence.

mod anf;
mod ball;
mod check;
mod circuit;
mod distance;
mod encode;
mod error;
mod exact;
mod ganak;
mod gpmc;
mod matrix;
mod pathsum;
mod phase;
mod qasm;
mod reduce;
mod wmc;
mod zpoly;

pub use ball::{ComplexBall, RealBall};
pub use check::{check, CheckRequest, CounterReport, Report};
pub use circuit::{Circuit, Gate};
pub use distance::DistanceBounds;
pub use error::EqError;
pub use exact::ExactAmp;
pub use pathsum::AmpValue;
pub use pathsum::PathSum;
pub use qasm::parse_qasm;
