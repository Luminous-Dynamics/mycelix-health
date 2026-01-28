# HDC Genetics Sweettest Integration Tests

Integration tests for the HDC Genetics Holochain zome using sweettest framework.

## Prerequisites

1. **Holochain Development Environment**
   ```bash
   nix develop
   ```

2. **Built WASM Zomes**
   ```bash
   # From project root
   nix develop
   cd zomes
   cargo build --target wasm32-unknown-unknown --release
   ```

3. **Built DNA File**
   ```bash
   # Requires hc tool from Holochain
   hc dna pack workdir/health.dna.yaml
   ```

## Running Tests

### Quick Start (Without Conductor)

The utility tests can run without a Holochain conductor:

```bash
cargo test -p hdc-genetics-sweettest test_action_hash_creation
cargo test -p hdc-genetics-sweettest test_metadata_serialization
cargo test -p hdc-genetics-sweettest test_coverage_summary
```

### Full Integration Tests

Full integration tests require a Holochain conductor and are marked `#[ignore]`:

```bash
# Run all ignored tests (requires conductor)
cargo test -p hdc-genetics-sweettest -- --ignored

# Run specific test
cargo test -p hdc-genetics-sweettest test_encode_dna_sequence -- --ignored
```

## Test Coverage

### DNA Sequence Encoding
- `test_encode_dna_sequence` - Encodes a DNA sequence as hypervector
- Tests k-mer configuration and metadata handling

### HLA Typing
- `test_encode_hla_typing` - Encodes HLA alleles for transplant matching
- `test_hla_matching_transplant_scenario` - Validates perfect vs partial match scoring

### SNP Panel Encoding
- `test_encode_snp_panel` - Encodes pharmacogenomic SNPs

### Similarity Computation
- `test_calculate_similarity_identical_sequences` - Verifies identical sequences score ~1.0
- `test_calculate_similarity_different_sequences` - Verifies different sequences score < 0.9

### Privacy Properties
- `test_hypervector_privacy_properties` - Validates non-invertibility of encoding

## Architecture

```
tests/sweettest/
├── Cargo.toml          # Test crate dependencies
├── README.md           # This file
└── tests/
    └── hdc_genetics.rs # Integration tests
```

## Troubleshooting

### "DNA file not found"
Build the DNA file first:
```bash
nix develop
hc dna pack workdir/health.dna.yaml
```

### "Conductor connection failed"
Ensure you're in the nix develop environment which provides Holochain:
```bash
nix develop
```

### Dependency Issues
The sweettest framework requires compatible versions of Holochain dependencies.
Check `Cargo.toml` versions match your Holochain version.

## Key Concepts

### Hyperdimensional Computing (HDC)
- Genetic data encoded as 10,000-dimensional binary vectors
- Similarity computed via Hamming/Cosine distance
- Non-invertible encoding provides privacy

### Test Scenarios
1. **DNA Barcode Matching** - Species identification via COI sequences
2. **HLA Transplant Matching** - Donor-recipient compatibility scoring
3. **Pharmacogenomics** - Drug response prediction via SNP panels

## Contributing

Add new tests following the existing pattern:
1. Define input/output structs matching zome types
2. Use `#[ignore]` for tests requiring conductor
3. Include assertions for expected behavior
4. Document the test purpose in comments
