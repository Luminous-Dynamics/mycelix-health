// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

/**
 * Return true only for IPv4/IPv6 loopback hosts. This helper deliberately
 * has no server or Holochain imports so safe-bind policy can be tested in
 * isolation from transport dependencies.
 */
export function isLoopbackHost(host: string): boolean {
  const normalized = host.trim().toLowerCase();
  return normalized === 'localhost'
    || normalized === '::1'
    || normalized === '[::1]'
    || (
      /^127(?:\.\d{1,3}){3}$/.test(normalized)
      && normalized.split('.').slice(1).every(octet => Number(octet) <= 255)
    );
}

