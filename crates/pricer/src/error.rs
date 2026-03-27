use thiserror::Error;

#[derive(Debug, Error)]
pub enum PricerError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Convergence failure: solver did not converge within max iterations")]
    ConvergenceFailure,

    #[error("Numerical instability encountered")]
    NumericalInstability,
}
