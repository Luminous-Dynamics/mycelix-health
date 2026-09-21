# Qualification verifier daemon core

QUAL-EVID-009C4A freezes the first Linux/Unix process boundary below the durable producer (#236) and above later authenticated transport handling.

## Process lease

`VerifierDaemonLeaseV1` owns:

- one kernel-managed exclusive advisory lock file;
- one bound Unix-domain socket in the same private runtime directory.

The live file lock is the singleton authority. PID text written into the lock file is diagnostic only and is never used to steal or infer ownership.

## Runtime filesystem profile

V1 requires:

```text
runtime directory: real directory, configured UID/GID, 0700
lock file:         real regular file, configured UID/GID, 0600
socket:            real Unix socket, configured UID/GID, 0600
```

A symlink final component is denied. A regular file at the socket path is denied.

A stale socket may be removed only after this process already holds the exclusive daemon lock and a connection probe demonstrates that no listener is reachable. A reachable pre-existing socket denies startup.

## RPC framing

V1 uses deterministic binary framing, not JSON.

Request:

```text
REQUEST_DOMAIN_V1
u16 schema
u8 operation
32-byte nonzero request id
u32 payload length
exact payload bytes
```

Response:

```text
RESPONSE_DOMAIN_V1
u16 schema
u8 operation
u8 result class
32-byte nonzero request id
u32 payload length
exact payload bytes
```

The hard payload limit is 64 KiB. Challenge and Status requests require an empty payload; Mint requires a non-empty payload.

`decode_request_header_v1()` allows the later socket reader to reject an oversized declared payload **before allocating or reading that body**.

## Deliberate separation

This crate does not expose its raw `UnixListener`. 009C4B will add an authenticated accept/read loop on top of the same source lineage.

## Non-claims

V1 does not establish:

- peer credentials or caller identity;
- capability-token authentication;
- request authorization;
- request read/write timeouts;
- durable consume/signing semantics (#236 owns those);
- external rollback resistance;
- cryptographic provider evidence;
- product trust / 009D;
- hostile same-UID process resistance;
- ancestor-directory anti-race guarantees outside the configured runtime directory;
- cross-platform socket/filesystem semantics.

The exact qualification profile is Ubuntu Linux. Other Unix systems require their own evidence.
