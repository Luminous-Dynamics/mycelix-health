# PHI Storage Policy

Production builds of the records DNA reject plaintext protected health
information at both enforcement layers:

1. Coordinator externs refuse legacy plaintext create and update operations.
2. Integrity validation rejects direct plaintext `EntryTypes` commits, including
   commits made through a modified or malicious coordinator.

The production write path is `store_encrypted_record`. Encryption occurs on the
patient or provider client before the zome call. The zome receives ciphertext,
a nonce, and routing metadata, but never receives plaintext or an encryption or
decryption key.

Clinical data is stored as an `EncryptedRecord` version-1 envelope using
XChaCha20-Poly1305. The patient hash, key fingerprint, data category, entry type,
envelope version, and encryption timestamp are serialized as
`EncryptedRecordAad` and authenticated as AEAD associated data. Changing any of
that routing metadata therefore invalidates decryption.

## Supported encrypted routes

- `Encounter` → `Procedures`
- `Diagnosis` → `Diagnoses`
- `ProcedurePerformed` → `Procedures`
- `LabResult` → `LabResults`
- `ImagingStudy` → `ImagingStudies`
- `VitalSigns` → `VitalSigns`
- `SdohScreening` → `Demographics`

Unknown categories and mismatched entry-type/category pairs fail closed. The
`All` category is not accepted for encrypted writes.

## Migration features

Two escape hatches exist only for isolated migration tooling:

- `dangerous-plaintext-phi` permits legacy plaintext entry types.
- `dangerous-server-side-phi` permits plaintext and key material to enter zome
  calls for the old encrypt/decrypt APIs.

A production DNA must never enable either feature. Because integrity rules form
part of the DNA hash, migration and production artifacts must be built and
distributed separately.

## Remaining work

Amendment requests currently have no encrypted workflow and are disabled in
production builds. A replacement should encrypt requested changes and denial
reasons while keeping only minimal workflow state and deadlines in cleartext.
Client SDKs should expose a single envelope builder so every platform serializes
`EncryptedRecordAad` identically.
