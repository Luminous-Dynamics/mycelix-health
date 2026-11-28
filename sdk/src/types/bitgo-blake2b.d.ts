// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Minimal local declaration for the CommonJS @bitgo/blake2b 3.2.4 package.
 * API verified against the package's published README and pinned lock entry.
 */
declare module "@bitgo/blake2b" {
  interface Blake2bHash {
    update(input: Uint8Array): Blake2bHash;
    digest(output: Uint8Array): Uint8Array;
    digest(encoding: "hex"): string;
    digest(): Uint8Array;
  }

  function blake2b(
    outputLength: number,
    key?: Uint8Array,
    salt?: Uint8Array,
    personal?: Uint8Array,
    noAssert?: boolean,
  ): Blake2bHash;

  export = blake2b;
}
