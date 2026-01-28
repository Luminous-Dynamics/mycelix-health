# {ZOME_NAME} Zome

> {ONE_LINE_DESCRIPTION}

## Overview

{DETAILED_DESCRIPTION}

## Architecture

```
{zome_name}/
├── integrity/           # Entry type definitions & validation
│   └── src/
│       └── lib.rs
└── coordinator/         # Business logic & extern functions
    └── src/
        └── lib.rs
```

## Entry Types

| Entry Type | Description | Links |
|------------|-------------|-------|
| {EntryType1} | {Description} | {LinkTypes} |
| {EntryType2} | {Description} | {LinkTypes} |

## Extern Functions

### Create Operations

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `create_{entity}` | `Create{Entity}Input` | `Record` | {Description} |

### Read Operations

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `get_{entity}` | `ActionHash` | `Option<{Entity}>` | {Description} |
| `get_{entity}_by_{field}` | `String` | `Option<Record>` | {Description} |
| `list_{entities}` | `{ListInput}` | `Vec<Record>` | {Description} |

### Update Operations

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `update_{entity}` | `Update{Entity}Input` | `Record` | {Description} |

### Delete Operations

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `delete_{entity}` | `ActionHash` | `ActionHash` | {Description} |

## Input Types

```rust
// Example input structures
#[derive(Serialize, Deserialize, Debug)]
pub struct Create{Entity}Input {
    pub field1: String,
    pub field2: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Update{Entity}Input {
    pub original_hash: ActionHash,
    pub updates: {Entity}Updates,
}
```

## Validation Rules

### Entry Validation

- **{Rule1}**: {Description}
- **{Rule2}**: {Description}

### Link Validation

- **{LinkType}**: {Validation rules}

## Usage Examples

### SDK (TypeScript)

```typescript
import { {Zome}Client } from '@mycelix/health-sdk';

// Create client
const client = new {Zome}Client(appClient, 'health');

// Create entity
const record = await client.create{Entity}({
  field1: 'value1',
  field2: 42,
});

// Get entity
const entity = await client.get{Entity}(record.hash);

// List entities
const entities = await client.list{Entities}({ limit: 10 });
```

### Direct Zome Calls

```typescript
const result = await appClient.callZome({
  cap_secret: null,
  role_name: 'health',
  zome_name: '{zome_name}',
  fn_name: 'create_{entity}',
  payload: {
    field1: 'value1',
    field2: 42,
  },
});
```

## Dependencies

### Internal Dependencies

- `mycelix-health-shared`: Common types, validation, error handling

### External Dependencies

- `hdi ^0.7.0`: Holochain Deterministic Integrity
- `hdk ^0.6.0`: Holochain Development Kit

## Error Handling

This zome uses typed errors from `mycelix-health-shared`:

```rust
use mycelix_health_shared::validation::{ValidationResult, ValidationError};

// Validation errors are returned as WasmError
validation_result.into_result()?;
```

Common error types:
- `ValidationError::Required`: Required field is missing
- `ValidationError::InvalidFormat`: Field format is invalid
- `ValidationError::OutOfRange`: Value is outside allowed range

## Testing

### Unit Tests

```bash
# Run unit tests (native target for non-WASM deps)
cargo test -p {zome_name}_integrity --target x86_64-unknown-linux-gnu
cargo test -p {zome_name}_coordinator --target x86_64-unknown-linux-gnu
```

### Integration Tests

Integration tests are located in `tests/` at the project root and use Sweettest.

## Privacy & Consent

{DESCRIBE_PRIVACY_MODEL}

- **Data Sensitivity**: {High/Medium/Low}
- **Consent Required**: {Yes/No}
- **Encryption**: {None/At-rest/End-to-end}

## FHIR Mapping

{IF_APPLICABLE}

| FHIR Resource | Entry Type | Notes |
|---------------|------------|-------|
| {Resource} | {EntryType} | {Notes} |

## Changelog

### v{VERSION} ({DATE})

- Initial implementation
- {Feature 1}
- {Feature 2}

## Related Zomes

- `{related_zome}`: {Relationship description}
- `{related_zome}`: {Relationship description}

## Contributing

1. Follow the validation patterns in `mycelix-health-shared`
2. Add unit tests for all validation rules
3. Update this README when adding new extern functions
4. Ensure FHIR compatibility where applicable
