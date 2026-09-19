# Mycelix Admin Authorization Core

Reference semantics for #176.

The key rule is deliberately simple:

> Administrative authorization dependency failure may reduce availability, but it may never increase authority.

## Ordinary authorization

Only explicit, valid `ConfirmedAdmin` evidence with a non-zero registry epoch can produce `Allow`.

Every dependency failure maps to denial, including:

- consent-zome unavailable;
- network failure;
- authentication failure;
- unauthorized cross-zome call;
- countersigning/session error;
- decode failure;
- missing/malformed admin registry.

There is no bootstrap fallback in the ordinary authorization state machine.

## Bootstrap

Bootstrap is a separate one-time transition configured outside ordinary request handling. It binds:

- exact DNA lineage;
- exact bootstrap principal;
- exact opaque token digest;
- finite not-before/expiry window.

The first arbitrary caller cannot become admin. Once bootstrap succeeds, the state irreversibly becomes `Initialized` and the same bootstrap authority cannot be consumed again.

A consent-zome outage does not mutate or activate bootstrap state.

## Deliberate boundaries

This crate does not:

- define how membership proofs are cryptographically signed;
- define the production Holochain admin registry entry/link schema;
- decide how installer/genesis configuration is authenticated;
- modify the current `require_admin_authorization()` implementation;
- establish legal/organizational administrator policy.

Those are integration proof lines after the fail-closed semantics are qualified.

## Production integration target

The packaged helper should translate every non-success `consent::is_admin` call outcome into denial. A separately qualified initialization path should create the first admin membership proof; ordinary authorization failure must never act as that path.

Tracks #176.
