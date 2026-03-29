// Custom random number generation for WASM portability.
// xorshift64 PRNG + Box-Muller transform for normal variates.
// No rand crate dependency.

use std::f64::consts::TAU; // TAU = 2*pi

// xorshift64: a fast PRNG that's just three XOR-shift operations on a 64-bit state.
// Not cryptographically secure, but random enough for Monte Carlo.
pub struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    // Create a new PRNG with a given seed. Seed must not be zero.
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    // Generate the next random u64 — the core xorshift algorithm.
    // Three shifts, three XORs. That's the whole thing.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    // Convert to a uniform f64 in (0, 1) — divide by max u64 value.
    // Excludes exactly 0.0 because Box-Muller needs ln(U1), and ln(0) = -infinity.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() as f64) / (u64::MAX as f64)
    }

    // Box-Muller transform: turns two uniform randoms into two standard normals.
    // Returns a pair (z1, z2) — both are independent N(0,1) samples.
    pub fn next_normal_pair(&mut self) -> (f64, f64) {
        let u1 = self.next_f64();
        let u2 = self.next_f64();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = TAU * u2;
        (r * theta.cos(), r * theta.sin())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Same seed should always produce the same sequence — reproducibility.
    #[test]
    fn deterministic() {
        let mut a = Xorshift64::new(42);
        let mut b = Xorshift64::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    // next_f64 should stay within (0, 1)
    #[test]
    fn f64_in_range() {
        let mut rng = Xorshift64::new(12345);
        for _ in 0..10_000 {
            let x = rng.next_f64();
            assert!(x > 0.0 && x < 1.0);
        }
    }

    // Box-Muller normals should average roughly zero over many samples.
    #[test]
    fn normals_mean_near_zero() {
        let mut rng = Xorshift64::new(99);
        let mut sum = 0.0;
        let n = 100_000;
        for _ in 0..n {
            let (z1, z2) = rng.next_normal_pair();
            sum += z1 + z2;
        }
        let mean = sum / (2 * n) as f64;
        assert!(mean.abs() < 0.01);
    }

    // Zero seed should not panic or produce all zeros.
    #[test]
    fn zero_seed_handled() {
        let mut rng = Xorshift64::new(0);
        assert!(rng.next_u64() != 0);
    }
}
