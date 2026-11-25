// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

import { describe, expect, it, vi } from 'vitest';
import {
  authorizeGatewayRequestV1,
  type GatewayAuthorityGrantV1,
  type GatewayPrincipalV1,
  type GatewayReceiptVerifierV1,
  type GatewayRequestContextV1,
} from '../src/auth/gateway-request-context.js';

const NOW = 100_000;
const digest = 'a'.repeat(64);

const principal: GatewayPrincipalV1 = {
  issuer: 'https://idp.example.test',
  subject: 'subject-42',
  clientId: 'ehr-client-7',
  organizationId: 'provider-org-3',
};

const context: GatewayRequestContextV1 = {
  version: 1,
  requestId: 'req-100',
  principal,
  tenantId: 'tenant-3',
  patientBinding: 'patient-binding-hash-9',
  sourceConnectionId: 'ehr-connection-7',
  purpose: 'treatment',
  recipientId: 'mycelix-health-tenant-3',
  action: 'fhir_export',
  requestedResourceTypes: ['Patient', 'Observation'],
  issuedAtMs: 90_000,
  expiresAtMs: 200_000,
  authenticationReceiptDigest: digest,
  consentReceiptDigest: 'b'.repeat(64),
};

const grant: GatewayAuthorityGrantV1 = {
  grantId: 'grant-12',
  principal,
  tenantId: 'tenant-3',
  patientBinding: 'patient-binding-hash-9',
  sourceConnectionId: 'ehr-connection-7',
  purpose: 'treatment',
  recipientId: 'mycelix-health-tenant-3',
  allowedActions: ['fhir_export'],
  allowedResourceTypes: ['Patient', 'Observation'],
  notBeforeMs: 80_000,
  expiresAtMs: 240_000,
  revokedAtMs: null,
};

function verifier(
  overrides: Partial<GatewayReceiptVerifierV1> = {},
): GatewayReceiptVerifierV1 {
  return {
    verifyPrincipal: vi.fn().mockResolvedValue(true),
    verifyConsent: vi.fn().mockResolvedValue(true),
    ...overrides,
  };
}

describe('authorizeGatewayRequestV1', () => {
  it('authorizes an exact subject-, tenant-, purpose-, source- and recipient-scoped request', async () => {
    const result = await authorizeGatewayRequestV1(context, grant, NOW, verifier());

    expect(result).toEqual({
      authorized: true,
      grantId: 'grant-12',
      requestId: 'req-100',
      principal,
      tenantId: 'tenant-3',
      patientBinding: 'patient-binding-hash-9',
      action: 'fhir_export',
      expiresAtMs: 200_000,
    });
  });

  it('fails closed when no verifier or current grant is configured', async () => {
    expect(await authorizeGatewayRequestV1(context, grant, NOW, null))
      .toEqual({ authorized: false, reason: 'verification_unavailable' });
    expect(await authorizeGatewayRequestV1(context, null, NOW, verifier()))
      .toEqual({ authorized: false, reason: 'verification_unavailable' });
  });

  it('requires verified authentication and consent receipts', async () => {
    const principalVerifier = verifier({
      verifyPrincipal: vi.fn().mockResolvedValue(false),
    });
    expect(await authorizeGatewayRequestV1(context, grant, NOW, principalVerifier))
      .toEqual({ authorized: false, reason: 'principal_unverified' });

    const consentVerifier = verifier({
      verifyConsent: vi.fn().mockResolvedValue(false),
    });
    expect(await authorizeGatewayRequestV1(context, grant, NOW, consentVerifier))
      .toEqual({ authorized: false, reason: 'consent_unverified' });
  });

  it('fails closed when either independent verifier throws', async () => {
    const principalVerifier = verifier({
      verifyPrincipal: vi.fn().mockRejectedValue(new Error('issuer/JWKS unavailable')),
    });
    expect(await authorizeGatewayRequestV1(context, grant, NOW, principalVerifier))
      .toEqual({ authorized: false, reason: 'verification_unavailable' });

    const consentVerifier = verifier({
      verifyConsent: vi.fn().mockRejectedValue(new Error('consent registry unavailable')),
    });
    expect(await authorizeGatewayRequestV1(context, grant, NOW, consentVerifier))
      .toEqual({ authorized: false, reason: 'verification_unavailable' });
  });

  it.each([
    ['issuer', { principal: { ...principal, issuer: 'https://attacker.example.test' } }, 'principal_mismatch'],
    ['subject', { principal: { ...principal, subject: 'other-subject' } }, 'principal_mismatch'],
    ['client', { principal: { ...principal, clientId: 'other-client' } }, 'principal_mismatch'],
    ['organization', { principal: { ...principal, organizationId: 'other-org' } }, 'principal_mismatch'],
    ['tenant', { tenantId: 'tenant-other' }, 'tenant_mismatch'],
    ['patient binding', { patientBinding: 'patient-binding-other' }, 'patient_mismatch'],
    ['source connection', { sourceConnectionId: 'other-ehr-connection' }, 'source_connection_mismatch'],
    ['purpose', { purpose: 'research' }, 'purpose_mismatch'],
    ['recipient', { recipientId: 'another-recipient' }, 'recipient_mismatch'],
    ['action', { action: 'sync_push' }, 'action_not_permitted'],
    ['resource type', { requestedResourceTypes: ['Patient', 'MedicationRequest'] }, 'resource_type_not_permitted'],
  ] as const)('rejects a mismatched %s', async (_name, override, reason) => {
    const changed = { ...context, ...override } as GatewayRequestContextV1;
    expect(await authorizeGatewayRequestV1(changed, grant, NOW, verifier()))
      .toEqual({ authorized: false, reason });
  });

  it('rejects malformed authority grants before invoking verifiers', async () => {
    const invalidGrant = { ...grant, expiresAtMs: Number.NaN };
    const receiptVerifier = verifier();

    expect(await authorizeGatewayRequestV1(context, invalidGrant, NOW, receiptVerifier))
      .toEqual({ authorized: false, reason: 'invalid_grant' });
    expect(receiptVerifier.verifyPrincipal).not.toHaveBeenCalled();
    expect(receiptVerifier.verifyConsent).not.toHaveBeenCalled();
  });

  it('rejects revoked, expired and not-yet-valid grants', async () => {
    expect(await authorizeGatewayRequestV1(
      context,
      { ...grant, revokedAtMs: NOW - 1 },
      NOW,
      verifier(),
    )).toEqual({ authorized: false, reason: 'grant_revoked' });

    expect(await authorizeGatewayRequestV1(
      context,
      { ...grant, expiresAtMs: NOW },
      NOW,
      verifier(),
    )).toEqual({ authorized: false, reason: 'grant_expired' });

    expect(await authorizeGatewayRequestV1(
      context,
      { ...grant, notBeforeMs: 95_000 },
      NOW,
      verifier(),
    )).toEqual({ authorized: false, reason: 'grant_not_yet_valid' });
  });

  it('rejects expired, future-issued and overlong request contexts', async () => {
    expect(await authorizeGatewayRequestV1(
      { ...context, expiresAtMs: NOW },
      grant,
      NOW,
      verifier(),
    )).toEqual({ authorized: false, reason: 'context_expired' });

    expect(await authorizeGatewayRequestV1(
      { ...context, issuedAtMs: NOW + 60_000 },
      grant,
      NOW,
      verifier(),
    )).toEqual({ authorized: false, reason: 'context_not_yet_valid' });

    expect(await authorizeGatewayRequestV1(
      { ...context, expiresAtMs: context.issuedAtMs + 300_001 },
      grant,
      NOW,
      verifier(),
    )).toEqual({ authorized: false, reason: 'context_ttl_exceeded' });
  });

  it('rejects missing or malformed receipt digests and incomplete context', async () => {
    expect(await authorizeGatewayRequestV1(
      { ...context, authenticationReceiptDigest: 'not-a-digest' },
      grant,
      NOW,
      verifier(),
    )).toEqual({ authorized: false, reason: 'invalid_receipt_digest' });

    expect(await authorizeGatewayRequestV1(
      { ...context, purpose: '   ' },
      grant,
      NOW,
      verifier(),
    )).toEqual({ authorized: false, reason: 'invalid_context' });
  });
});
