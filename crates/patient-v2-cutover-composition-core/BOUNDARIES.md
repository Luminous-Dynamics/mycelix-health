# Final composition boundaries

The composition core consumes adapter-verified receipts; it does not establish their truth.

In particular:

- `qualified=true` is an adapter boundary, not CI/signature verification inside this crate;
- `CompositionCommitmentReceipt` is an adapter assertion that its digest commits to the canonical transcript; hashing is external;
- timestamps are semantic inputs; trusted time is external;
- `durable=true` is a persistence-adapter assertion; fsync/Holochain/database durability is external;
- `v2_discoverable=true` and `legacy_v1_read_only=true` are adapter assertions whose production truth requires independent integration qualification;
- crash recovery here is structural receipt reconciliation, not a distributed transaction protocol.

None of these adapter claims may be generated from UI state, a Git branch name, merge permission, MATL/reputation, or model confidence.
