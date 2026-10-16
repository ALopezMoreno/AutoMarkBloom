//! All transcendental math goes through libm so native and wasm builds give
//! bit-identical flowers. Do not call f64::sin/exp/etc. elsewhere.
//!
//! sqrt is the exception: IEEE 754 requires it correctly rounded, so the
//! hardware instruction gives the same bits everywhere. libm::sqrt already
//! uses it natively but falls back to a much slower software routine on wasm.

pub fn sin(x: f64) -> f64 { libm::sin(x) }
pub fn cos(x: f64) -> f64 { libm::cos(x) }
pub fn exp(x: f64) -> f64 { libm::exp(x) }
pub fn ln(x: f64) -> f64 { libm::log(x) }
pub fn sqrt(x: f64) -> f64 { x.sqrt() }
pub fn pow(x: f64, y: f64) -> f64 { libm::pow(x, y) }
pub fn atan2(y: f64, x: f64) -> f64 { libm::atan2(y, x) }
pub fn floor(x: f64) -> f64 { libm::floor(x) }
pub fn round(x: f64) -> f64 { libm::round(x) }
pub fn erf(x: f64) -> f64 { libm::erf(x) }
