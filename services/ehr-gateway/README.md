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

## Testing

```bash
npm test
```

## Environment Variables

```bash
# Epic
EPIC_CLIENT_ID=your-epic-client-id
EPIC_BASE_URL=https://fhir.epic.com/...

# Cerner
CERNER_CLIENT_ID=your-cerner-client-id
CERNER_BASE_URL=https://fhir.cerner.com/...

# Generic FHIR
FHIR_BASE_URL=https://your-fhir-server/fhir
```

## License

Part of Mycelix-Health. See root LICENSE file.
