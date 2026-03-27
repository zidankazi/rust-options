// Custom random number generation for WASM portability.
// xorshift64 PRNG + Box-Muller transform for normal variates.
// No rand crate dependency.

// TODO: implement Xorshift64 struct with next_u64() and next_f64()
// TODO: implement box_muller(rng) -> (f64, f64) returning two standard normals
