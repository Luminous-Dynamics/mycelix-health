# PR review notes

The final composition core is intentionally isolated from #204 and #206 implementation branches.

Review should focus on:

1. exact cross-cutover identity matching;
2. required lower-layer contract versions;
3. final-authority binding to the exact migration destination;
4. qualified composition-commitment adapter boundary;
5. durable prepare timestamp and recovery time fence;
6. durable activation timestamp, destination discoverability, and legacy-v1 read-only requirement;
7. exact idempotent replay vs conflicting second activation;
8. absence of any legacy-writable or force-activation state.

The branch is source-staged and unexecuted until its exact-head qualifier actually runs.
