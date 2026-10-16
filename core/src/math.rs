//! All transcendental math goes through libm so native and wasm builds give
//! bit-identical flowers. Do not call f64::sin/exp/etc. elsewhere.

pub fn sin(x: f64) -> f64 { libm::sin(x) }
pub fn cos(x: f64) -> f64 { libm::cos(x) }
pub fn exp(x: f64) -> f64 { libm::exp(x) }
pub fn ln(x: f64) -> f64 { libm::log(x) }
pub fn sqrt(x: f64) -> f64 { libm::sqrt(x) }
pub fn pow(x: f64, y: f64) -> f64 { libm::pow(x, y) }
pub fn atan2(y: f64, x: f64) -> f64 { libm::atan2(y, x) }
pub fn floor(x: f64) -> f64 { libm::floor(x) }
pub fn round(x: f64) -> f64 { libm::round(x) }
