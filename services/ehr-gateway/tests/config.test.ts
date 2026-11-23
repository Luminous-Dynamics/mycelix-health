// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

import { describe, expect, it } from 'vitest';
import { ServerConfigSchema, getTestConfig } from '../src/config.js';
import { isLoopbackHost } from '../src/server.js';

describe('EHR gateway safe network defaults', () => {
  it('defaults to loopback binding', () => {
    expect(ServerConfigSchema.parse({}).host).toBe('127.0.0.1');
    expect(getTestConfig().host).toBe('127.0.0.1');
  });

  it('does not enable cross-origin access by default', () => {
    expect(ServerConfigSchema.parse({}).corsOrigins).toEqual([]);
    expect(getTestConfig().corsOrigins).toEqual([]);
  });

  it('rejects wildcard CORS when credentialed requests are enabled', () => {
    expect(() => ServerConfigSchema.parse({ corsOrigins: ['*'] })).toThrow(
      'Wildcard CORS origins are forbidden',
    );
    expect(() => ServerConfigSchema.parse({ corsOrigins: '*' })).toThrow(
      'Wildcard CORS origins are forbidden',
    );
  });

  it('normalizes explicitly configured exact origins', () => {
    expect(
      ServerConfigSchema.parse({
        corsOrigins: 'https://portal.example, https://provider.example ',
      }).corsOrigins,
    ).toEqual([
      'https://portal.example',
      'https://provider.example',
    ]);
  });

  it('recognizes loopback hosts and rejects network interfaces', () => {
    for (const host of ['localhost', '127.0.0.1', '127.12.34.56', '::1', '[::1]']) {
      expect(isLoopbackHost(host)).toBe(true);
    }

    for (const host of ['0.0.0.0', '192.168.1.10', '10.0.0.2', 'example.org']) {
      expect(isLoopbackHost(host)).toBe(false);
    }
  });
});
