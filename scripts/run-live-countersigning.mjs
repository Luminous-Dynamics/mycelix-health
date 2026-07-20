#!/usr/bin/env node
/**
 * Execute one externally managed live native-countersigning ceremony.
 *
 * This runner never launches conductors and never fabricates success. Every
 * configured participant must already expose an authenticated app websocket.
 * A success marker is written only after the SDK independently verifies the
 * exported Holochain action evidence.
 */

import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { AppWebsocket, encodeHashToBase64 } from '../sdk/node_modules/@holochain/client/lib/index.js';
import {
  ClinicalCountersigningClientV1,
  executeClinicalCountersigningCeremonyV1,
} from '../sdk/dist/index.mjs';

function fail(message) {
  process.stderr.write(`live countersigning failed: ${message}\n`);
  process.exit(1);
}

function bytes(value, label) {
  if (!Array.isArray(value) || value.some((item) => !Number.isInteger(item) || item < 0 || item > 255)) {
    throw new Error(`${label} must be a JSON byte array`);
  }
  return Uint8Array.from(value);
}

function decodeIntent(value) {
  const intent = structuredClone(value);
  intent.session_start_ms = BigInt(intent.session_start_ms);
  intent.session_end_ms = BigInt(intent.session_end_ms);
  intent.nonce = bytes(intent.nonce, 'intent nonce');
  intent.proposal.created_at_ms = BigInt(intent.proposal.created_at_ms);
  intent.proposal.expires_at_ms = BigInt(intent.proposal.expires_at_ms);
  for (const field of ['mutation_binding_digest', 'deployment_evidence_sha256', 'policy_digest', 'rationale_sha256', 'nonce']) {
    intent.proposal[field] = bytes(intent.proposal[field], `proposal ${field}`);
  }
  for (const participant of intent.participants) participant.agent_bytes = bytes(participant.agent_bytes, 'participant agent bytes');
  return intent;
}

function canonicalJson(value) {
  const normalize = (item) => {
    if (typeof item === 'bigint') return item.toString();
    if (item instanceof Uint8Array) return Array.from(item);
    if (Array.isArray(item)) return item.map(normalize);
    if (item && typeof item === 'object') {
      return Object.fromEntries(Object.keys(item).sort().map((key) => [key, normalize(item[key])]));
    }
    return item;
  };
  return `${JSON.stringify(normalize(value), null, 2)}\n`;
}

const configPath = process.argv[2];
const outputPath = process.argv[3];
if (!configPath || !outputPath) fail('usage: run-live-countersigning.mjs CONFIG.json OUTPUT_DIR');

try {
  const config = JSON.parse(await readFile(resolve(configPath), 'utf8'));
  if (config.schema_version !== 1 || typeof config.scenario_id !== 'string' || !Array.isArray(config.participants)) {
    throw new Error('live configuration schema is invalid');
  }
  const intent = decodeIntent(config.intent);
  if (config.participants.length !== intent.participants.length) throw new Error('configured participant count does not match intent');

  const sockets = [];
  try {
    const participants = [];
    for (const [index, participantConfig] of config.participants.entries()) {
      if (participantConfig.binding !== intent.participants[index]?.agent_binding) throw new Error(`participant ${index} is not in exact intent order`);
      const token = participantConfig.token_base64 === undefined
        ? undefined
        : Uint8Array.from(Buffer.from(participantConfig.token_base64, 'base64'));
      if (token !== undefined && token.length === 0) throw new Error(`participant ${index} app token is empty`);
      const socket = await AppWebsocket.connect({ url: new URL(participantConfig.url), token });
      sockets.push(socket);
      const actualBinding = encodeHashToBase64(socket.myPubKey);
      if (actualBinding !== participantConfig.binding) throw new Error(`participant ${index} websocket agent does not match configured binding`);
      participants.push({
        participant_binding: actualBinding,
        client: new ClinicalCountersigningClientV1(socket, participantConfig.role_name ?? 'health'),
      });
    }

    const completed = await executeClinicalCountersigningCeremonyV1(participants, intent);
    const outputDir = resolve(outputPath);
    await mkdir(outputDir, { recursive: true });
    await writeFile(resolve(outputDir, 'evidence.json'), canonicalJson(completed.evidence), { flag: 'wx' });
    await writeFile(resolve(outputDir, 'verified-summary.json'), canonicalJson(completed.verified), { flag: 'wx' });
    await writeFile(resolve(outputDir, 'execution-audits.json'), canonicalJson(completed.audits), { flag: 'wx' });
    await writeFile(resolve(outputDir, 'LIVE_COUNTERSIGNING_VERIFIED.json'), canonicalJson({
      schema_version: 1,
      scenario_id: config.scenario_id,
      status: 'verified',
      participant_bindings: completed.verified.participants,
      participant_action_hashes: completed.verified.participant_action_hashes,
      countersigned_entry_hash: completed.verified.countersigned_entry_hash,
      intent_digest: completed.verified.intent_digest,
    }), { flag: 'wx' });
  } finally {
    await Promise.allSettled(sockets.map((socket) => socket.client.close()));
  }
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
}
