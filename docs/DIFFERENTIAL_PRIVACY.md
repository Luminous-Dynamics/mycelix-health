# Differential Privacy in Mycelix-Health

This document describes the formal differential privacy (DP) implementation in Mycelix-Health, which provides mathematically provable privacy guarantees for patient data.

## Overview

Differential privacy is a mathematical framework that provides strong privacy guarantees when analyzing sensitive data. The key insight is that an algorithm is differentially private if its output doesn't change significantly when any single individual's data is added or removed from the dataset.

**Formal Definition**: A randomized algorithm M is (ε, δ)-differentially private if for any two datasets D and D' differing in at most one record, and for any subset S of outputs:

```
Pr[M(D) ∈ S] ≤ e^ε × Pr[M(D') ∈ S] + δ
```

Where:
- **ε (epsilon)**: Privacy loss parameter. Smaller values = stronger privacy.
- **δ (delta)**: Probability of privacy breach. Should be cryptographically small (< 1/n).

## Why Differential Privacy?

Traditional anonymization techniques (k-anonymity, data masking) have been repeatedly broken. Differential privacy provides:

1. **Mathematical Guarantees**: Provable bounds on privacy loss
2. **Composition**: Privacy degrades gracefully over multiple queries
3. **Post-Processing Immunity**: Any computation on DP output is also DP
4. **Future-Proof**: Resistant to auxiliary information attacks

## Implementation Components

### 1. Cryptographic Random Number Generator (`dp_core::rng`)

The foundation of DP is secure randomness. We use the `getrandom` crate which provides cryptographically secure random numbers.

```rust
use mycelix_health_shared::dp_core::rng::SecureRng;

// Generate random bytes
let mut bytes = [0u8; 32];
SecureRng::fill_bytes(&mut bytes)?;

// Generate uniform random in [0, 1)
let u: f64 = SecureRng::random_f64_uniform()?;

// Generate centered random in [-0.5, 0.5)
let c: f64 = SecureRng::random_f64_centered()?;
```

### 2. Laplace Mechanism (`dp_core::laplace`)

The Laplace mechanism adds noise from the Laplace distribution to achieve pure ε-differential privacy.

**When to use**: Numeric queries where you want pure ε-DP (no δ).

```rust
use mycelix_health_shared::dp_core::laplace::LaplaceMechanism;

// Query: "How many patients have diabetes?"
let true_count = 150.0;
let sensitivity = 1.0;  // Adding/removing one patient changes count by at most 1
let epsilon = 0.1;      // Privacy parameter

let noisy_count = LaplaceMechanism::add_noise(true_count, sensitivity, epsilon)?;
// noisy_count might be 147.3 or 152.8, etc.
```

**Mathematical Properties**:
- Scale parameter: `b = Δf / ε` where Δf is the sensitivity
- Variance: `2b²`
- Expected error: `b`

### 3. Gaussian Mechanism (`dp_core::gaussian`)

The Gaussian mechanism adds normally distributed noise for (ε, δ)-DP, which can be more efficient for some applications.

**When to use**: When you can tolerate small δ and want tighter confidence intervals.

```rust
use mycelix_health_shared::dp_core::gaussian::GaussianMechanism;

// Query with Gaussian noise
let true_value = 75.5;
let sensitivity = 2.0;
let epsilon = 0.5;
let delta = 1e-6;  // Very small failure probability

let noisy_value = GaussianMechanism::add_noise(true_value, sensitivity, epsilon, delta)?;
```

**Mathematical Properties**:
- Standard deviation: `σ = Δf × √(2 ln(1.25/δ)) / ε`
- Provides (ε, δ)-DP guarantee

### 4. Privacy Budget Accounting (`dp_core::budget`)

Each DP query consumes privacy budget. Without tracking, an adversary could combine many queries to defeat privacy.

```rust
use mycelix_health_shared::dp_core::budget::{BudgetAccount, CompositionTheorem};

// Create budget account with ε=1.0 total budget
let mut budget = BudgetAccount::new(1.0);

// Each query consumes some budget
budget.check_and_consume(0.1)?;  // Query 1: ε=0.1
budget.check_and_consume(0.2)?;  // Query 2: ε=0.2
budget.check_and_consume(0.3)?;  // Query 3: ε=0.3

// Check remaining budget
println!("Remaining: {}", budget.remaining_epsilon());  // 0.4

// This will fail - insufficient budget
let result = budget.check_and_consume(0.5);
assert!(result.is_err());  // BudgetError::Exhausted
```

#### Advanced Composition

For many queries, advanced composition provides tighter bounds than simple addition:

```rust
// Create budget with advanced composition
let mut budget = BudgetAccount::new_with_delta(
    5.0,    // total epsilon
    1e-5,   // total delta
    1e-6,   // delta' for composition
);

// Can answer more queries with same privacy guarantee
for _ in 0..50 {
    budget.check_and_consume(0.1)?;
}

// Under basic composition: 50 × 0.1 = 5.0 (exhausted)
// Under advanced composition: ~3.5 (still has budget remaining!)
```

### 5. Parameter Validation (`dp_core::validation`)

Always validate DP parameters before use:

```rust
use mycelix_health_shared::dp_core::validation::*;

// Validate individual parameters
validate_epsilon(0.1)?;      // Must be positive and finite
validate_delta(1e-6)?;       // Must be in [0, 1) and typically < 0.01
validate_sensitivity(1.0)?;  // Must be positive and finite

// Validate all parameters at once
validate_dp_parameters(0.1, 1e-6, 1.0)?;

// Get recommended parameters for a dataset size
let params = recommended_dp_parameters(10000);  // 10,000 records
println!("Recommended ε: {}", params.epsilon);
println!("Recommended δ: {}", params.delta);
```

## Usage in Commons Zome

The Commons zome uses these primitives for privacy-preserving analytics:

```rust
// In coordinator/src/lib.rs

#[hdk_extern]
pub fn query_pool_stats(input: PoolStatsQuery) -> ExternResult<DpPoolStats> {
    // Get or create budget for this patient-pool pair
    let mut budget = get_or_create_budget(input.patient_hash, input.pool_hash)?;

    // Check if we have sufficient budget
    check_budget_available(&budget, input.epsilon)?;

    // Compute the statistic
    let true_value = compute_pool_statistic(&input)?;

    // Add calibrated noise
    let noisy_value = compute_dp_result_with_budget(
        true_value,
        input.sensitivity,
        input.epsilon,
        &mut budget,
    )?;

    // Persist the updated budget
    update_privacy_budget(&budget)?;

    Ok(DpPoolStats {
        value: noisy_value,
        epsilon_consumed: input.epsilon,
        remaining_budget: budget.remaining_epsilon(),
    })
}
```

## Best Practices

### 1. Choose Appropriate Epsilon

| Use Case | Recommended ε | Privacy Level |
|----------|--------------|---------------|
| Highly sensitive (HIV status) | 0.01 - 0.1 | Very strong |
| Medical records | 0.1 - 1.0 | Strong |
| Aggregate statistics | 1.0 - 5.0 | Moderate |
| Non-sensitive analytics | 5.0 - 10.0 | Weak |

### 2. Calculate Sensitivity Correctly

Sensitivity (Δf) is the maximum change in query output when one record changes:

- **Count queries**: Δf = 1
- **Sum queries**: Δf = max possible value
- **Average queries**: Δf = range / n (use with caution)
- **Histogram queries**: Δf = 1 per bin

### 3. Budget Management

- Set per-user/per-dataset budgets based on data sensitivity
- Consider time-based budget renewal (e.g., weekly reset)
- Log all budget consumption for audit trails
- Fail closed: reject queries when budget is exhausted

### 4. Error Handling

```rust
match budget.check_and_consume(epsilon) {
    Ok(()) => {
        // Proceed with query
    }
    Err(BudgetError::Exhausted { required, remaining }) => {
        // Log the denial
        warn!("Budget exhausted: needed {}, had {}", required, remaining);
        return Err(ExternError::guest("Privacy budget exhausted"));
    }
    Err(BudgetError::InvalidParameter(msg)) => {
        // Invalid input
        return Err(ExternError::guest(format!("Invalid parameter: {}", msg)));
    }
}
```

## Testing

The implementation includes comprehensive tests:

1. **Unit tests**: Verify individual mechanism correctness
2. **Property-based tests (proptest)**:
   - Budget monotonicity (never increases)
   - Budget non-negativity
   - Composition theorem correctness
   - Serialization roundtrip
3. **Statistical tests**: Verify distribution properties (mean, variance)

Run tests:
```bash
cargo test -p mycelix-health-shared --target x86_64-unknown-linux-gnu
```

## References

- Dwork, C., & Roth, A. (2014). The Algorithmic Foundations of Differential Privacy.
- Dwork, C., et al. (2010). Boosting and Differential Privacy.
- Apple Differential Privacy Technical Overview
- Google's RAPPOR: Randomized Aggregatable Privacy-Preserving Ordinal Response

## Security Considerations

1. **RNG Quality**: Uses `getrandom` which provides OS-level cryptographic randomness
2. **Budget Persistence**: Budgets are stored in Holochain entries with tamper-evident links
3. **Validation**: All parameters are validated before use
4. **Composition**: Proper tracking prevents budget overflow attacks
5. **Audit Trail**: All queries and budget changes are logged

## Limitations

1. **Not a Silver Bullet**: DP protects against certain attacks but doesn't prevent all privacy breaches
2. **Accuracy Trade-off**: Stronger privacy = more noise = less accurate results
3. **Budget Exhaustion**: Finite privacy budget means limited queries
4. **Correlation Attacks**: Multiple correlated queries can leak information
5. **Side Channels**: Timing and other side channels are not addressed by DP alone
