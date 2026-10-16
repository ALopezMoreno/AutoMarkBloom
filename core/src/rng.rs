//! A small, fully specified PRNG (PCG32 seeded through SplitMix64).
//!
//! We avoid any library or platform RNG so a seed means the same random
//! stream in every build, engine and language port.

use crate::math;

pub fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Derive an independent seed for a named sub-stream.
pub fn derive(seed: u64, key: u64) -> u64 {
    splitmix64(seed ^ splitmix64(key.wrapping_add(0x5EED)))
}

#[derive(Clone, Debug)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    pub fn new(seed: u64, stream: u64) -> Self {
        let mut r = Pcg32 { state: 0, inc: (stream << 1) | 1 };
        r.next_u32();
        r.state = r.state.wrapping_add(seed);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(self.inc);
        let xs = (((old >> 18) ^ old) >> 27) as u32;
        xs.rotate_right((old >> 59) as u32)
    }

    /// Uniform in [0, 1) with 53 bits.
    pub fn f64(&mut self) -> f64 {
        let a = (self.next_u32() >> 5) as u64;
        let b = (self.next_u32() >> 6) as u64;
        ((a << 26) | b) as f64 / (1u64 << 53) as f64
    }

    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f64()
    }

    /// Uniform integer in [lo, hi] inclusive.
    pub fn int(&mut self, lo: i64, hi: i64) -> i64 {
        let n = (hi - lo + 1) as f64;
        (lo + (self.f64() * n) as i64).min(hi)
    }

    pub fn normal(&mut self) -> f64 {
        let u1 = self.f64().max(1e-300);
        let u2 = self.f64();
        math::sqrt(-2.0 * math::ln(u1)) * math::cos(std::f64::consts::TAU * u2)
    }

    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.int(0, i as i64) as usize;
            v.swap(i, j);
        }
    }
}
