#![forbid(unsafe_code)]
//! Bounded zeroizing in-memory vault-session reference semantics.
//!
//! This crate deliberately does **not** implement browser storage, passphrase
//! derivation, encryption, Holochain access, Leptos reactivity, or distributed
//! key sharing. It freezes VAULT-SESSION-001 (#174): one local root key is owned
//! by one bounded session, can only be borrowed through a closure, and is dropped
//! (therefore zeroized by `zeroize`) on lock/expiry/replacement/teardown.

use core::fmt;
use zeroize::Zeroizing;

pub const VAULT_SESSION_V1: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockReason {
    NeverUnlocked,
    User,
    AbsoluteExpiry,
    IdleTimeout,
    ClockRollback,
    Replaced,
    VaultDestroyed,
    SessionTeardown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionStatus {
    Locked {
        reason: LockReason,
    },
    Unlocked {
        unlocked_at_millis: u64,
        last_used_at_millis: u64,
        absolute_expires_at_millis: u64,
        idle_expires_at_millis: Option<u64>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionConfigError {
    ZeroAbsoluteTtl,
    ZeroIdleTtl,
    IdleTtlExceedsAbsoluteTtl,
    ExpiryOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionUseError {
    Locked(LockReason),
    AbsoluteExpired,
    IdleExpired,
    ClockRollback,
}

/// Owns exactly one zeroizing vault key while unlocked.
///
/// Deliberately **not** `Clone`, `Copy`, `Serialize`, or `Deserialize`.
/// Callers can use the key only through [`VaultSession::with_key`], which lends a
/// shared reference for the duration of one closure and never returns key bytes.
pub struct VaultSession {
    version: u16,
    key: Option<Zeroizing<[u8; 32]>>,
    unlocked_at_millis: Option<u64>,
    last_used_at_millis: Option<u64>,
    absolute_expires_at_millis: Option<u64>,
    idle_ttl_millis: Option<u64>,
    lock_reason: LockReason,
}

impl Default for VaultSession {
    fn default() -> Self {
        Self::new()
    }
}

impl VaultSession {
    pub const fn new() -> Self {
        Self {
            version: VAULT_SESSION_V1,
            key: None,
            unlocked_at_millis: None,
            last_used_at_millis: None,
            absolute_expires_at_millis: None,
            idle_ttl_millis: None,
            lock_reason: LockReason::NeverUnlocked,
        }
    }

    pub const fn version(&self) -> u16 {
        self.version
    }

    /// Transfers ownership of a zeroizing key into a finite session.
    ///
    /// If a key is already present, it is dropped/zeroized before the new key is
    /// installed. `idle_ttl_millis`, when supplied, must not exceed the absolute
    /// TTL; the absolute bound always wins.
    pub fn unlock(
        &mut self,
        key: Zeroizing<[u8; 32]>,
        now_millis: u64,
        absolute_ttl_millis: u64,
        idle_ttl_millis: Option<u64>,
    ) -> Result<(), SessionConfigError> {
        if absolute_ttl_millis == 0 {
            return Err(SessionConfigError::ZeroAbsoluteTtl);
        }
        if let Some(idle) = idle_ttl_millis {
            if idle == 0 {
                return Err(SessionConfigError::ZeroIdleTtl);
            }
            if idle > absolute_ttl_millis {
                return Err(SessionConfigError::IdleTtlExceedsAbsoluteTtl);
            }
        }

        let absolute_expires_at_millis = now_millis
            .checked_add(absolute_ttl_millis)
            .ok_or(SessionConfigError::ExpiryOverflow)?;
        if let Some(idle) = idle_ttl_millis {
            now_millis
                .checked_add(idle)
                .ok_or(SessionConfigError::ExpiryOverflow)?;
        }

        if self.key.is_some() {
            self.lock(LockReason::Replaced);
        }

        self.key = Some(key);
        self.unlocked_at_millis = Some(now_millis);
        self.last_used_at_millis = Some(now_millis);
        self.absolute_expires_at_millis = Some(absolute_expires_at_millis);
        self.idle_ttl_millis = idle_ttl_millis;
        // `NeverUnlocked` is used as the neutral sentinel while a session is live;
        // status never exposes it as a live lock reason until the key is absent.
        self.lock_reason = LockReason::NeverUnlocked;
        Ok(())
    }

    /// Lend the key for one operation without copying it out of the session.
    ///
    /// Expiry is enforced *before* the closure receives the key. Any timeout or
    /// clock rollback locks the session first, which drops/zeroizes the key.
    pub fn with_key<R>(
        &mut self,
        now_millis: u64,
        operation: impl FnOnce(&[u8; 32]) -> R,
    ) -> Result<R, SessionUseError> {
        self.enforce_time(now_millis)?;
        let key = self
            .key
            .as_deref()
            .ok_or(SessionUseError::Locked(self.lock_reason))?;
        let result = operation(key);
        self.last_used_at_millis = Some(now_millis);
        Ok(result)
    }

    /// Enforce timeout state and return a secret-free status snapshot.
    pub fn status(&mut self, now_millis: u64) -> SessionStatus {
        if self.key.is_some() {
            let _ = self.enforce_time(now_millis);
        }

        let Some(unlocked_at_millis) = self.unlocked_at_millis else {
            return SessionStatus::Locked {
                reason: self.lock_reason,
            };
        };
        let Some(last_used_at_millis) = self.last_used_at_millis else {
            return SessionStatus::Locked {
                reason: self.lock_reason,
            };
        };
        let Some(absolute_expires_at_millis) = self.absolute_expires_at_millis else {
            return SessionStatus::Locked {
                reason: self.lock_reason,
            };
        };
        if self.key.is_none() {
            return SessionStatus::Locked {
                reason: self.lock_reason,
            };
        }

        SessionStatus::Unlocked {
            unlocked_at_millis,
            last_used_at_millis,
            absolute_expires_at_millis,
            idle_expires_at_millis: self
                .idle_ttl_millis
                .and_then(|ttl| last_used_at_millis.checked_add(ttl)),
        }
    }

    pub fn is_unlocked(&mut self, now_millis: u64) -> bool {
        matches!(self.status(now_millis), SessionStatus::Unlocked { .. })
    }

    /// Explicitly lock and zeroize the current key by dropping its `Zeroizing`
    /// owner. Safe to call repeatedly.
    pub fn lock(&mut self, reason: LockReason) {
        self.key.take();
        self.unlocked_at_millis = None;
        self.last_used_at_millis = None;
        self.absolute_expires_at_millis = None;
        self.idle_ttl_millis = None;
        self.lock_reason = reason;
    }

    fn enforce_time(&mut self, now_millis: u64) -> Result<(), SessionUseError> {
        if self.key.is_none() {
            return Err(SessionUseError::Locked(self.lock_reason));
        }

        let unlocked_at = self
            .unlocked_at_millis
            .ok_or(SessionUseError::Locked(self.lock_reason))?;
        let last_used = self
            .last_used_at_millis
            .ok_or(SessionUseError::Locked(self.lock_reason))?;
        let absolute_expiry = self
            .absolute_expires_at_millis
            .ok_or(SessionUseError::Locked(self.lock_reason))?;

        if now_millis < unlocked_at || now_millis < last_used {
            self.lock(LockReason::ClockRollback);
            return Err(SessionUseError::ClockRollback);
        }

        if now_millis >= absolute_expiry {
            self.lock(LockReason::AbsoluteExpiry);
            return Err(SessionUseError::AbsoluteExpired);
        }

        if let Some(idle_ttl) = self.idle_ttl_millis {
            let Some(idle_expiry) = last_used.checked_add(idle_ttl) else {
                self.lock(LockReason::ClockRollback);
                return Err(SessionUseError::ClockRollback);
            };
            if now_millis >= idle_expiry {
                self.lock(LockReason::IdleTimeout);
                return Err(SessionUseError::IdleExpired);
            }
        }

        Ok(())
    }
}

impl fmt::Debug for VaultSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VaultSession")
            .field("version", &self.version)
            .field("key", &self.key.as_ref().map(|_| "[redacted]"))
            .field("unlocked_at_millis", &self.unlocked_at_millis)
            .field("last_used_at_millis", &self.last_used_at_millis)
            .field(
                "absolute_expires_at_millis",
                &self.absolute_expires_at_millis,
            )
            .field("idle_ttl_millis", &self.idle_ttl_millis)
            .field("lock_reason", &self.lock_reason)
            .finish()
    }
}

impl Drop for VaultSession {
    fn drop(&mut self) {
        // `take()` drops Zeroizing<[u8; 32]> immediately rather than waiting for
        // the rest of the struct's fields to be dropped.
        self.key.take();
        self.lock_reason = LockReason::SessionTeardown;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret(byte: u8) -> Zeroizing<[u8; 32]> {
        Zeroizing::new([byte; 32])
    }

    #[test]
    fn new_session_is_locked() {
        let mut session = VaultSession::new();
        assert_eq!(
            session.status(0),
            SessionStatus::Locked {
                reason: LockReason::NeverUnlocked
            }
        );
    }

    #[test]
    fn zero_absolute_ttl_is_rejected() {
        let mut session = VaultSession::new();
        assert_eq!(
            session.unlock(secret(7), 100, 0, None),
            Err(SessionConfigError::ZeroAbsoluteTtl)
        );
    }

    #[test]
    fn zero_idle_ttl_is_rejected() {
        let mut session = VaultSession::new();
        assert_eq!(
            session.unlock(secret(7), 100, 1_000, Some(0)),
            Err(SessionConfigError::ZeroIdleTtl)
        );
    }

    #[test]
    fn idle_ttl_cannot_exceed_absolute_ttl() {
        let mut session = VaultSession::new();
        assert_eq!(
            session.unlock(secret(7), 100, 1_000, Some(1_001)),
            Err(SessionConfigError::IdleTtlExceedsAbsoluteTtl)
        );
    }

    #[test]
    fn expiry_overflow_is_rejected_without_installing_key() {
        let mut session = VaultSession::new();
        assert_eq!(
            session.unlock(secret(7), u64::MAX - 10, 20, None),
            Err(SessionConfigError::ExpiryOverflow)
        );
        assert_eq!(
            session.status(0),
            SessionStatus::Locked {
                reason: LockReason::NeverUnlocked
            }
        );
    }

    #[test]
    fn key_is_only_borrowed_through_closure() {
        let mut session = VaultSession::new();
        session.unlock(secret(0xA5), 100, 1_000, Some(500)).unwrap();
        let first = session.with_key(200, |key| key[0]).unwrap();
        assert_eq!(first, 0xA5);
        assert!(session.is_unlocked(200));
    }

    #[test]
    fn absolute_expiry_locks_before_key_use() {
        let mut session = VaultSession::new();
        session.unlock(secret(7), 100, 100, None).unwrap();
        let result = session.with_key(200, |_| panic!("expired key must not be lent"));
        assert_eq!(result, Err(SessionUseError::AbsoluteExpired));
        assert_eq!(
            session.status(200),
            SessionStatus::Locked {
                reason: LockReason::AbsoluteExpiry
            }
        );
    }

    #[test]
    fn idle_expiry_locks_before_key_use() {
        let mut session = VaultSession::new();
        session.unlock(secret(7), 100, 1_000, Some(100)).unwrap();
        session.with_key(150, |_| ()).unwrap();
        let result = session.with_key(250, |_| panic!("idle-expired key must not be lent"));
        assert_eq!(result, Err(SessionUseError::IdleExpired));
        assert_eq!(
            session.status(250),
            SessionStatus::Locked {
                reason: LockReason::IdleTimeout
            }
        );
    }

    #[test]
    fn successful_use_refreshes_idle_deadline() {
        let mut session = VaultSession::new();
        session.unlock(secret(7), 100, 1_000, Some(100)).unwrap();
        session.with_key(150, |_| ()).unwrap();
        session.with_key(249, |_| ()).unwrap();
        assert!(session.is_unlocked(300));
    }

    #[test]
    fn clock_rollback_locks_fail_closed() {
        let mut session = VaultSession::new();
        session.unlock(secret(7), 100, 1_000, Some(100)).unwrap();
        session.with_key(150, |_| ()).unwrap();
        let result = session.with_key(149, |_| panic!("rolled-back clock must not lend key"));
        assert_eq!(result, Err(SessionUseError::ClockRollback));
        assert_eq!(
            session.status(149),
            SessionStatus::Locked {
                reason: LockReason::ClockRollback
            }
        );
    }

    #[test]
    fn explicit_lock_removes_key_access() {
        let mut session = VaultSession::new();
        session.unlock(secret(7), 100, 1_000, None).unwrap();
        session.lock(LockReason::User);
        assert_eq!(
            session.with_key(200, |_| ()),
            Err(SessionUseError::Locked(LockReason::User))
        );
    }

    #[test]
    fn replacement_uses_new_secret_only() {
        let mut session = VaultSession::new();
        session.unlock(secret(1), 100, 1_000, None).unwrap();
        assert_eq!(session.with_key(110, |key| key[0]).unwrap(), 1);
        session.unlock(secret(2), 120, 1_000, None).unwrap();
        assert_eq!(session.with_key(130, |key| key[0]).unwrap(), 2);
    }

    #[test]
    fn vault_destroy_reason_is_preserved() {
        let mut session = VaultSession::new();
        session.unlock(secret(7), 100, 1_000, None).unwrap();
        session.lock(LockReason::VaultDestroyed);
        assert_eq!(
            session.status(200),
            SessionStatus::Locked {
                reason: LockReason::VaultDestroyed
            }
        );
    }

    #[test]
    fn debug_output_redacts_secret() {
        let mut session = VaultSession::new();
        session.unlock(secret(0xAB), 100, 1_000, None).unwrap();
        let debug = format!("{session:?}");
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains("171, 171"));
        assert!(!debug.contains("abababab"));
    }
}
