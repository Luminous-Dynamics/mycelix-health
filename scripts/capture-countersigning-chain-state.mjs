#!/usr/bin/env node
/** Capture bounded, entry-redacted source-chain and DHT summaries for the lab. */
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import {
  AdminWebsocket,
  CellType,
  encodeHashToBase64,
} from '../sdk/node_modules/@holochain/client/lib/index.js';

function fail(message) {
  process.stderr.write(`chain-state capture failed: ${message}\n`);
  process.exit(1);
}

function binding(value) {
  return value instanceof Uint8Array ? encodeHashToBase64(value) : null;
}

function actionSummary(record) {
  const action = record?.action ?? {};
  return {
    action_hash: binding(record?.action_address),
    type: typeof action.type === 'string' ? action.type : 'Unknown',
    action_seq: Number.isInteger(action.action_seq) ? action.action_seq : null,
    author: binding(action.author),
    timestamp: action.timestamp ?? null,
    entry_hash: binding(action.entry_hash),
    entry_redacted: true,
  };
}

const inventoryPath = resolve(process.argv[2] ?? 'target/countersigning-lab/runtime/inventory.json');
const outputPath = resolve(process.argv[3] ?? 'target/countersigning-lab/runtime/chain-dht-summary.json');

try {
  const inventory = JSON.parse(await readFile(inventoryPath, 'utf8'));
  if (inventory.schema_version !== 1 || !Array.isArray(inventory.participants)) throw new Error('runtime inventory is invalid');
  const participants = [];
  for (const item of inventory.participants) {
    const admin = await AdminWebsocket.connect({
      url: new URL(item.admin_url),
      wsClientOptions: { origin: item.allowed_origin },
    });
    try {
      const apps = await admin.listApps({});
      const app = apps.find((candidate) => candidate.installed_app_id === item.installed_app_id);
      if (!app) throw new Error(`${item.name}: installed app is missing`);
      const provisioned = (app.cell_info[item.role_name] ?? []).find((cell) => cell.type === CellType.Provisioned);
      if (!provisioned) throw new Error(`${item.name}: provisioned cell is missing`);
      const state = await admin.dumpFullState({ cell_id: provisioned.value.cell_id });
      const records = state.source_chain_dump?.records ?? [];
      participants.push({
        name: item.name,
        binding: item.binding,
        dna_binding: encodeHashToBase64(provisioned.value.cell_id[0]),
        source_chain: {
          record_count: records.length,
          published_ops_count: state.source_chain_dump?.published_ops_count ?? null,
          tail: records.slice(-64).map(actionSummary),
        },
        dht: {
          validation_limbo_count: state.integration_dump?.validation_limbo?.length ?? 0,
          integration_limbo_count: state.integration_dump?.integration_limbo?.length ?? 0,
          integrated_count: state.integration_dump?.integrated?.length ?? 0,
          dht_ops_cursor: state.integration_dump?.dht_ops_cursor ?? null,
        },
        network: {
          peer_count: state.peer_dump?.peers?.length ?? 0,
          has_local_agent_info: state.peer_dump?.this_agent_info !== undefined,
        },
      });
    } finally {
      await admin.client.close();
    }
  }
  const output = {
    schema_version: 1,
    capture_policy: 'entry_redacted_tail_64',
    observed_at: new Date().toISOString(),
    participants,
  };
  await mkdir(dirname(outputPath), { recursive: true });
  await writeFile(outputPath, `${JSON.stringify(output, null, 2, (_, value) => typeof value === 'bigint' ? value.toString() : value)}\n`, { flag: 'wx', mode: 0o600 });
  process.stdout.write(`${outputPath}\n`);
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
}
