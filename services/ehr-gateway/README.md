# EHR Gateway Service

TypeScript service for integrating Mycelix-Health with external Electronic Health Record (EHR) systems via FHIR R4 and SMART on FHIR.

## Features

- **SMART on FHIR Authentication**: OAuth2 with PKCE for secure EHR access
- **EHR Adapters**: Pre-built adapters for Epic, Cerner, and generic FHIR R4 servers
- **Bidirectional Sync**: Pull data from EHRs and push updates back
- **Conflict Resolution**: Multiple strategies for handling data conflicts

## Installation

```bash
npm install
```

## Usage

### SMART on FHIR Authentication

```typescript
import { SmartOnFhirAuth } from './auth/smart-on-fhir';

const auth = new SmartOnFhirAuth({
  clientId: 'your-client-id',
  redirectUri: 'http://localhost:3000/callback',
  scope: ['patient/*.read', 'launch/patient'],
});

// Discover endpoints
await auth.discover('https://fhir.epic.com/interconnect-fhir-oauth/api/FHIR/R4');

// Get authorization URL
const authUrl = await auth.getAuthorizationUrl();

// Exchange code for token
const tokenInfo = await auth.exchangeCodeForToken(code, codeVerifier);
```

### EHR Adapters

```typescript
import { EpicAdapter } from './adapters/epic';
import { CernerAdapter } from './adapters/cerner';

// Epic
const epic = new EpicAdapter({
  baseUrl: 'https://fhir.epic.com/...',
  clientId: '...',
});

// Cerner
const cerner = new CernerAdapter({
  baseUrl: 'https://fhir.cerner.com/...',
  clientId: '...',
});

// Fetch patient data
const patient = await epic.getPatient(accessToken, patientId);
const observations = await epic.getObservations(accessToken, patientId);
```

### Conflict Resolution

```typescript
import { ConflictResolver } from './sync/conflict-resolver';

const resolver = new ConflictResolver({
  defaultStrategy: 'most_recent',
  autoResolveThreshold: 0.1,
  mergeRules: {
    preferLocalFields: ['phone'],
    preferRemoteFields: ['email'],
  },
});

// Detect conflicts
const conflict = resolver.detectConflict(
  'Patient', 'patient-123',
  localData, remoteData,
  localVersion, remoteVersion
);

// Resolve
if (conflict) {
  const id = resolver.registerConflict(conflict);
  const resolution = await resolver.resolveConflict(id, 'merge', 'user-id');
}
```

## Architecture

```
src/
├── auth/
│   ├── smart-on-fhir.ts    # SMART on FHIR OAuth2 + PKCE
│   └── token-manager.ts    # Token lifecycle management
├── adapters/
│   ├── base.ts             # Abstract base adapter
│   ├── epic.ts             # Epic MyChart adapter
│   ├── cerner.ts           # Cerner Millennium adapter
│   └── generic-fhir.ts     # Generic FHIR R4 adapter
├── sync/
│   ├── pull-service.ts     # Pull data from EHRs
│   ├── push-service.ts     # Push updates to EHRs
│   └── conflict-resolver.ts # Handle sync conflicts
├── types.ts                # TypeScript type definitions
└── index.ts                # Main exports
```

## Resolution Strategies

| Strategy | Description |
|----------|-------------|
| `local_wins` | Always use local data |
| `remote_wins` | Always use remote data |
| `most_recent` | Use data with latest timestamp |
| `merge` | Combine fields from both sources |
| `manual` | Require manual resolution |

## Running as HTTP Server

The gateway can run as a standalone HTTP server for receiving FHIR webhooks and serving the API.

### Quick Start

```bash
# Build
npm run build

# Start (connects to Holochain at ws://localhost:8888)
npm run start

# Or with custom configuration
PORT=3000 HOLOCHAIN_URL=ws://localhost:8888 npm run start
```

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| GET | `/ready` | Readiness check (requires Holochain) |
| POST | `/fhir/Bundle` | Ingest FHIR Bundle |
| GET | `/fhir/Bundle/:patientHash` | Export patient as FHIR Bundle |
| GET | `/fhir/metadata` | FHIR CapabilityStatement |
| GET | `/auth/smart/launch` | SMART on FHIR launch |
| GET | `/auth/smart/callback` | OAuth callback |
| POST | `/sync/patient/:id` | Trigger patient sync |
| GET | `/sync/status/:syncId` | Check sync status |

---

## Local Development with Holochain

### Prerequisites

Ensure you have the Holochain tools installed and patched for your system:

```bash
# Check versions (should be 0.6.0)
~/.local/bin/hc --version      # hc 0.6.0
~/.local/bin/holochain --version  # holochain 0.6.0
```

If not installed, see the project root's setup instructions.

### Step 1: Build WASM Zomes

From the project root (`mycelix-health/`):

```bash
cargo build --release --target wasm32-unknown-unknown
```

### Step 2: Pack DNA and hApp

```bash
~/.local/bin/hc dna pack dna -o bundles/health.dna
~/.local/bin/hc app pack . -o bundles/mycelix-health.happ
```

### Step 3: Start Holochain Sandbox

```bash
# Create sandbox directory
SANDBOX_DIR=$(mktemp -d)
echo "Sandbox: $SANDBOX_DIR"

# Generate sandbox with the hApp
~/.local/bin/hc sandbox generate --root "$SANDBOX_DIR" bundles

# Run the sandbox (this will prompt for a passphrase)
~/.local/bin/hc sandbox run --root "$SANDBOX_DIR" 0 -p 8888
```

The conductor will be available at `ws://localhost:8888`.

### Step 4: Start EHR Gateway

In a separate terminal:

```bash
cd services/ehr-gateway
HOLOCHAIN_URL=ws://localhost:8888 npm run start
```

### Step 5: Test FHIR Ingestion

```bash
# Health check
curl http://localhost:3000/health

# Ingest test bundle
curl -X POST http://localhost:3000/fhir/Bundle \
  -H "Content-Type: application/json" \
  -H "X-Source-System: test-ehr" \
  -d @../../tests/fixtures/patient-bundle.json

# Expected response:
# {
#   "status": "success",
#   "source_system": "test-ehr",
#   "report": {
#     "total_processed": 7,
#     "patients_created": 1,
#     "conditions_created": 1,
#     "observations_created": 1,
#     ...
#   }
# }
```

### Automated Test Script

For non-interactive testing with piped passphrase:

```bash
./scripts/test-e2e.sh
```

**Known Limitation:** The hc CLI has a 60-second websocket timeout that may not
be sufficient for installing large hApps (the mycelix-health.happ is 11MB). If
app installation times out, the app may still be installing in the background.
Wait 1-2 minutes and check if the gateway can connect.

### Manual Testing (Recommended for Large hApps)

If the automated script times out, run manually in separate terminals:

**Terminal 1 - Conductor:**
```bash
export PATH="$HOME/.local/bin:$PATH"
echo "test-passphrase" | hc sandbox --piped -f 8877 create --in-process-lair
echo "test-passphrase" | hc sandbox --piped -f 8877 run 0 -p 8878
# Wait for "Conductor ready"
```

**Terminal 2 - Install App (after conductor is ready):**
```bash
export PATH="$HOME/.local/bin:$PATH"
# This may take 1-2 minutes for large hApps
echo "test-passphrase" | hc sandbox --piped call --running 8877 install-app bundles/mycelix-health.happ
```

**Terminal 3 - Gateway:**
```bash
cd services/ehr-gateway
HOLOCHAIN_URL=ws://localhost:8878 npm run start
```

**Terminal 4 - Test:**
```bash
curl http://localhost:3000/health
curl -X POST http://localhost:3000/fhir/Bundle \
  -H "Content-Type: application/json" \
  -H "X-Source-System: test-ehr" \
  -d @tests/fixtures/patient-bundle.json
```

### Using start-local-env.sh

For interactive development (prompts for passphrase):

```bash
./scripts/start-local-env.sh
```

This will:
1. Build WASM zomes (if needed)
2. Pack DNA and hApp
3. Start the Holochain sandbox
4. Start the EHR Gateway

---

## Testing

```bash
npm test
```

### Test Fixtures

The `tests/fixtures/` directory contains sample FHIR bundles:

- `patient-bundle.json` - Complete patient with all 7 resource types

---

## Holochain Compatibility

This gateway is compatible with **Holochain 0.6.0**:

| Component | Version |
|-----------|---------|
| @holochain/client | 0.20.0 |
| holochain conductor | 0.6.0 |
| hc CLI | 0.6.0 |
| hdk | 0.6.0 |
| hdi | 0.7.0 |

---

## Environment Variables

### Server Configuration

```bash
# HTTP Server
PORT=3000                           # Server port (default: 3000)
EHR_GATEWAY_HOST=0.0.0.0           # Bind address (default: 0.0.0.0)

# Holochain Connection
HOLOCHAIN_URL=ws://localhost:8888   # Conductor WebSocket URL
HOLOCHAIN_APP_ID=mycelix-health     # App ID in conductor

# Timeouts & Retries
DEFAULT_TIMEOUT=30000               # Operation timeout in ms
MAX_RETRIES=3                       # Max retry attempts

# Logging
LOG_LEVEL=info                      # debug, info, warn, error
ENABLE_REQUEST_LOGGING=true         # Log all requests

# CORS
CORS_ORIGINS=*                      # Comma-separated origins

# OAuth (for SMART on FHIR redirects)
OAUTH_CALLBACK_BASE_URL=http://localhost:3000
```

### EHR Connection Configuration

Configure multiple EHR connections using prefixed environment variables:

```bash
# Epic Connection
EHR_CONNECTION_EPIC_SYSTEM=epic
EHR_CONNECTION_EPIC_BASE_URL=https://fhir.epic.com/interconnect-fhir-oauth/api/FHIR/R4
EHR_CONNECTION_EPIC_AUTH_URL=https://fhir.epic.com/interconnect-fhir-oauth/oauth2/authorize
EHR_CONNECTION_EPIC_TOKEN_URL=https://fhir.epic.com/interconnect-fhir-oauth/oauth2/token
EHR_CONNECTION_EPIC_CLIENT_ID=your-epic-client-id
EHR_CONNECTION_EPIC_CLIENT_SECRET=your-epic-secret
EHR_CONNECTION_EPIC_SOURCE_SYSTEM_ID=epic-mychart

# Cerner Connection
EHR_CONNECTION_CERNER_SYSTEM=cerner
EHR_CONNECTION_CERNER_BASE_URL=https://fhir.cerner.com/...
EHR_CONNECTION_CERNER_AUTH_URL=https://authorization.cerner.com/...
EHR_CONNECTION_CERNER_TOKEN_URL=https://authorization.cerner.com/...
EHR_CONNECTION_CERNER_CLIENT_ID=your-cerner-client-id
EHR_CONNECTION_CERNER_SOURCE_SYSTEM_ID=cerner-millennium
```

## License

Part of Mycelix-Health. See root LICENSE file.
