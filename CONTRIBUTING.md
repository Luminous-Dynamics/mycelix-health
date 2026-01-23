# Contributing to Mycelix-Health

Thank you for your interest in contributing to Mycelix-Health! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Submitting Changes](#submitting-changes)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Documentation](#documentation)

## Code of Conduct

This project adheres to a code of conduct that all contributors are expected to follow. Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- **Rust** (stable, latest version recommended)
- **Holochain** development environment (0.4.x)
- **Node.js** 18+ (for SDK development)
- **npm** or **yarn**

### Project Structure

```
mycelix-health/
├── zomes/                  # Holochain zomes
│   ├── patient/           # Patient identity & demographics
│   ├── consent/           # Access authorization
│   ├── commons/           # Privacy-preserving data pools (DP)
│   ├── trials/            # Clinical research
│   └── shared/            # Shared types and utilities
├── sdk/                   # TypeScript SDK
├── dna/                   # DNA manifest
├── tests/                 # Integration tests
└── docs/                  # Documentation
```

## Development Setup

### 1. Clone the Repository

```bash
git clone https://github.com/Luminous-Dynamics/mycelix-health.git
cd mycelix-health
```

### 2. Rust/Holochain Development

```bash
# Build all zomes (native, for tests)
cargo build

# Run tests
cargo test --workspace

# Build WASM zomes (requires wasm32-unknown-unknown target)
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
```

### 3. SDK Development

```bash
cd sdk
npm install
npm run build
npm test
```

## Making Changes

### Branching Strategy

- `main` - Production-ready code
- `feature/*` - New features
- `fix/*` - Bug fixes
- `docs/*` - Documentation updates

### Workflow

1. Fork the repository
2. Create a feature branch from `main`
3. Make your changes
4. Write/update tests
5. Ensure all tests pass
6. Submit a pull request

## Submitting Changes

### Commit Messages

Follow conventional commits format:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `test`: Test additions/changes
- `refactor`: Code refactoring
- `chore`: Maintenance tasks

Examples:
```
feat(commons): add Gaussian mechanism for DP queries
fix(consent): correct expiration check for time-bound consents
docs(sdk): update privacy budget examples
```

### Pull Request Guidelines

1. **Title**: Clear, descriptive title
2. **Description**: Explain what changes were made and why
3. **Tests**: Include relevant tests
4. **Documentation**: Update docs if needed
5. **Privacy**: Note any changes affecting differential privacy

## Coding Standards

### Rust

- Follow Rust standard formatting (`cargo fmt`)
- Use `clippy` for linting (`cargo clippy`)
- Write doc comments for public APIs
- Prefer explicit types for clarity in complex code

```rust
/// Creates a new data pool with the specified parameters.
///
/// # Arguments
/// * `name` - Pool name
/// * `epsilon` - Default privacy budget per query
///
/// # Returns
/// The action hash of the created pool entry.
#[hdk_extern]
pub fn create_pool(input: CreatePoolInput) -> ExternResult<ActionHash> {
    // Implementation
}
```

### TypeScript (SDK)

- Use TypeScript strict mode
- Follow existing code style
- Write JSDoc comments for public APIs
- Include type exports for all public types

```typescript
/**
 * Creates a new patient record.
 *
 * @param input - Patient demographics and information
 * @returns The created patient with action hash
 * @throws {HealthSdkError} If creation fails
 */
async createPatient(input: CreatePatientInput): Promise<PatientRecord> {
  // Implementation
}
```

## Testing

### Rust Tests

```bash
# Run all tests
cargo test --workspace

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### SDK Tests

```bash
cd sdk
npm test              # Run all tests
npm run test:watch    # Watch mode
npm run typecheck     # Type checking only
```

### Writing Tests

- Unit tests for individual functions
- Integration tests for zome interactions
- Property-based tests for differential privacy mechanisms

## Documentation

### Code Documentation

- Document all public APIs
- Include examples in doc comments
- Explain privacy implications where relevant

### Updating README

If your changes affect:
- Features/capabilities
- Installation/setup
- API usage
- Privacy guarantees

Please update the relevant README files.

## Security and Privacy

### Differential Privacy

If modifying the `commons` zome or any DP-related code:

1. **Mathematical Correctness**: Ensure DP guarantees are preserved
2. **Budget Tracking**: Verify budget consumption is accurate
3. **Testing**: Add property-based tests for statistical properties
4. **Documentation**: Document privacy guarantees

### Security Considerations

- Never log sensitive patient data
- Validate all inputs
- Use cryptographic randomness for DP mechanisms
- Follow HIPAA-aligned practices

## Getting Help

- **Questions**: Open a [Question issue](https://github.com/Luminous-Dynamics/mycelix-health/issues/new?template=question.md)
- **Bugs**: Open a [Bug Report](https://github.com/Luminous-Dynamics/mycelix-health/issues/new?template=bug_report.md)
- **Features**: Open a [Feature Request](https://github.com/Luminous-Dynamics/mycelix-health/issues/new?template=feature_request.md)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to decentralized healthcare!
