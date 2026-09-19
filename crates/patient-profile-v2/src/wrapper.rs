#![forbid(unsafe_code)]

#[path = "lib.rs"]
mod inner;

pub use inner::*;

// Unit-test assertion formatting must never force the production protected
// profile to expose its fields through Debug. Keep this implementation test-only
// and deliberately redacted.
#[cfg(test)]
impl core::fmt::Debug for inner::ProtectedPatientProfileV2 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("ProtectedPatientProfileV2([protected])")
    }
}
