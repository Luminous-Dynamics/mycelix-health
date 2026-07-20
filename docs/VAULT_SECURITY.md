# Browser Vault Security Boundary

The portal stores only a versioned, authenticated wrapper for the patient's
32-byte health-data key.

## Current envelope: v2

- Entropy, salt, and nonce come from `crypto.getRandomValues`.
- Patient keys are derived with HMAC-HKDF-SHA256.
- Passphrases are stretched with PBKDF2-HMAC-SHA256, a random 128-bit salt,
  and a versioned work factor of 210,000 iterations.
- Keys are wrapped with XChaCha20-Poly1305 using a random 192-bit nonce.
- Envelope version, work factor, and key fingerprint are authenticated as AEAD
  associated data.
- Malformed fields, weak work factors, all-zero randomness, wrong passphrases,
  and modified ciphertext fail closed.

The work factor is deliberately encoded in each envelope so it can be raised by
future migrations. Argon2id remains the preferred next KDF once the browser
worker and memory-budget design is benchmarked across supported devices.

## Legacy handling

The former fixed-salt, iterated-SHA256, XOR wrapper provides no ciphertext
integrity. Current code detects that format but never decrypts it. A patient must
recover from the 24-word seed phrase and seal a new v2 vault.

## Remaining boundary

An authenticated wrapper is not a complete session manager. The unwrapped key
must be retained only in a bounded, zeroizing in-memory session and must never be
placed in persistent browser storage, logs, DOM attributes, or analytics.
