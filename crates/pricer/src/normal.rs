// Standard normal PDF and CDF from scratch.
// CDF uses the Abramowitz and Stegun approximation.
    
/// Standard normal probability density function.
/// n(x) = (1/sqrt(2*pi)) * exp(-x^2/2)
#[inline]
pub fn norm_pdf(x: f64) -> f64 {
    const INV_SQRT_2PI: f64 = 0.398_942_280_401_432_7;
    INV_SQRT_2PI * (-0.5 * x * x).exp()
}

/// Standard normal cumulative distribution function.
/// Uses Abramowitz and Stegun approximation 26.2.17 — accurate to ~1e-7.
#[inline]
pub fn norm_cdf(x: f64) -> f64 {
    let a1 = 0.319_381_530;
    let a2 = -0.356_563_782;
    let a3 = 1.781_477_937;
    let a4 = -1.821_255_978;
    let a5 = 1.330_274_429;
    let p = 0.231_641_900;

    let abs_x = x.abs();
    let t = 1.0 / (1.0 + p * abs_x);
    // Horner's method for the polynomial — more numerically stable
    let poly = t * (a1 + t * (a2 + t * (a3 + t * (a4 + t * a5))));
    let y = 1.0 - poly * norm_pdf(abs_x);

    if x >= 0.0 { y } else { 1.0 - y }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_at_zero() {
        assert!((norm_pdf(0.0) - 0.398_942_280_401_432_7).abs() < 1e-10);
    }

    #[test]
    fn pdf_symmetry() {
        assert!((norm_pdf(1.0) - norm_pdf(-1.0)).abs() < 1e-15);
    }

    #[test]
    fn cdf_at_zero() {
        assert!((norm_cdf(0.0) - 0.5).abs() < 1e-7);
    }

    #[test]
    fn cdf_symmetry() {
        assert!((norm_cdf(2.0) + norm_cdf(-2.0) - 1.0).abs() < 1e-7);
    }

    #[test]
    fn cdf_known_values() {
        assert!((norm_cdf(1.0) - 0.841_344_746).abs() < 1e-6);
        assert!((norm_cdf(-1.0) - 0.158_655_254).abs() < 1e-6);
        assert!((norm_cdf(2.0) - 0.977_249_868).abs() < 1e-6);
    }

    #[test]
    fn cdf_extremes() {
        assert!(norm_cdf(10.0) > 0.999_999);
        assert!(norm_cdf(-10.0) < 0.000_001);
    }
}
