# Boundaries

## Qualified claim if exact workflow passes

Only the Linux/Unix 009C4A theorem:

1. one held kernel advisory lock prevents a second cooperating daemon instance from owning the same runtime lease;
2. final runtime/lock/socket objects satisfy the configured UID/GID and strict mode/type requirements;
3. stale Unix sockets are removed only while the exclusive lease is held and only after a failed connection probe;
4. the raw listener is not publicly exposed;
5. request/response frames are domain-separated, versioned, bounded and exact-length;
6. oversized request bodies can be rejected from the fixed header before body read/allocation;
7. unknown schema/op/result, zero request ids, truncation and trailing bytes fail closed.

## Cooperative locking boundary

Advisory file locking prevents other cooperating instances from acquiring the same lease. It is not a mandatory-access-control boundary against arbitrary same-UID processes that ignore the lock.

## Filesystem boundary

The final runtime directory, lock file and socket path are validated. V1 does not prove that every ancestor path component is race-free against a hostile same-UID filesystem adversary.

## Socket boundary

Binding a private Unix socket does not authenticate clients. 009C4B must establish peer credentials + capability admission before any producer operation is reachable.

## Transport boundary

This crate freezes frame bytes and ownership only. It does not yet perform socket reads/writes, timeouts, cancellation handling, backpressure, or request dispatch.

## Authority boundary

No result from this crate is a qualification receipt, provider receipt, Health authority token, or Patient-v2 activation authority.
