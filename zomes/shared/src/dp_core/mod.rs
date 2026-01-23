//! Formal Differential Privacy (DP) Core Module
//!
//! Provides mathematically rigorous differential privacy primitives:
//! - Cryptographically secure random number generation
//! - Laplace mechanism for (ε, 0)-DP
//! - Gaussian mechanism for (ε, δ)-DP
//! - Privacy budget accounting with composition theorems
//! - Input validation for DP parameters
//!
//! # Mathematical Guarantees
//!
//! Differential Privacy ensures that for any two neighboring datasets D and D'
//! (differing by at most one record), and any output S:
//!
//! P[M(D) ∈ S] ≤ e^ε · P[M(D') ∈ S] + δ
//!
//! Where:
//! - ε (epsilon): Privacy loss parameter (lower = more private)
//! - δ (delta): Probability of privacy failure (should be negligible, e.g., 10^-6)
//!
//! # Example
//!
//! ```ignore
//! use mycelix_health_shared::dp_core::{laplace, budget::BudgetAccount, validation};
//!
//! // Validate DP parameters
//! validation::validate_epsilon(0.1)?;
//! validation::validate_sensitivity(1.0)?;
//!
//! // Create budget tracker
//! let mut budget = BudgetAccount::new(1.0); // Total ε = 1.0
//!
//! // Add Laplace noise to a count query (sensitivity = 1)
//! let true_count = 42.0;
//! let epsilon = 0.1;
//! let sensitivity = 1.0;
//!
//! // Check and consume budget
//! budget.check_and_consume(epsilon)?;
//!
//! // Apply mechanism
//! let noisy_count = laplace::add_noise(true_count, sensitivity, epsilon)?;
//! ```

pub mod budget;
pub mod gaussian;
pub mod laplace;
pub mod rng;
pub mod validation;

// Re-export commonly used items
pub use budget::{BudgetAccount, BudgetError, CompositionTheorem};
pub use gaussian::GaussianMechanism;
pub use laplace::LaplaceMechanism;
pub use rng::SecureRng;
pub use validation::{validate_delta, validate_epsilon, validate_sensitivity, DpValidationError};
