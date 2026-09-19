# Mycelix Vault Session Core

Reference semantics for VAULT-SESSION-001 (#174).

The Leptos portal already has an authenticated v2 wrapper for the patient vault key. This crate addresses the next boundary: what happens **after** successful unwrap.

## Contract

A `VaultSession`:

- owns at most one `Zeroizing<[u8; 32]>`;
- is intentionally not cloneable or serializable;
- accepts a `Zeroizing` key by ownership transfer;
- exposes secret use only through `with_key(now_millis, |key| ...)`;
- enforces absolute expiry before every use;
- optionally enforces idle expiry;
- fails closed on caller-clock rollback;
- drops/zeroizes the current key on explicit lock, timeout, replacement, vault destruction, or object teardown;
- returns only secret-free status snapshots;
- redacts the key in `Debug`.

The caller supplies time to keep the core deterministic and browser-independent. A Leptos adapter can source monotonic/application time and own a single shared session handle without cloning the key bytes.

## Non-goals

This crate does not implement:

- browser storage;
- passphrase/KDF policy;
- record encryption;
- Holochain capabilities;
- distributed key wrapping;
- CareVault selective disclosure;
- Symthaea access;
- protection against a compromised browser/runtime/extension.

The local root vault key must never appear in CARE-DISC projections or Symthaea task envelopes.

## Qualification scope

A green reference run establishes only the in-memory session state-machine invariants for the exact tested source/toolchain. Leptos integration requires a separate PR proving that UI `Unlocked` state cannot diverge from the actual session and that vault destruction/lock paths close the session first.
