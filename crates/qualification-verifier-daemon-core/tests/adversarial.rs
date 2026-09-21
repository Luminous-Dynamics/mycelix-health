#![cfg(unix)]

use std::fs::{self, DirBuilder};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use mycelix_qualification_verifier_daemon_core::{
    DaemonCoreError, DaemonRuntimeConfigV1, VerifierDaemonLeaseV1,
};

static TEST_ID: AtomicU64 = AtomicU64::new(1);

fn temp_path(name: &str) -> PathBuf {
    let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "mycelix-verifier-daemon-adversarial-{name}-{}-{id}",
        std::process::id()
    ))
}

fn current_ids() -> (u32, u32) {
    let probe = temp_path("probe");
    fs::write(&probe, b"probe").expect("probe");
    let metadata = fs::metadata(&probe).expect("metadata");
    let ids = (metadata.uid(), metadata.gid());
    fs::remove_file(probe).expect("cleanup probe");
    ids
}

fn private_dir(path: &Path) {
    let mut builder = DirBuilder::new();
    builder.mode(0o700);
    builder.create(path).expect("private dir");
}

fn cleanup_tree(path: &Path) {
    let _ = fs::remove_file(path.join("verifier.sock"));
    let _ = fs::remove_file(path.join("verifier.lock"));
    let _ = fs::remove_file(path);
    let _ = fs::remove_dir(path);
}

#[test]
fn runtime_symlink_is_denied() {
    let (uid, gid) = current_ids();
    let target = temp_path("runtime-target");
    let link = temp_path("runtime-link");
    private_dir(&target);
    symlink(&target, &link).expect("symlink");

    let config = DaemonRuntimeConfigV1::new(&link, uid, gid);
    assert!(matches!(
        VerifierDaemonLeaseV1::acquire(config),
        Err(DaemonCoreError::RuntimePathIsSymlink)
    ));

    let _ = fs::remove_file(&link);
    cleanup_tree(&target);
}

#[test]
fn lock_symlink_is_denied() {
    let (uid, gid) = current_ids();
    let runtime = temp_path("lock-link");
    private_dir(&runtime);
    let target = runtime.join("target-file");
    fs::write(&target, b"target").expect("target");
    symlink(&target, runtime.join("verifier.lock")).expect("lock symlink");

    let config = DaemonRuntimeConfigV1::new(&runtime, uid, gid);
    assert!(matches!(
        VerifierDaemonLeaseV1::acquire(config),
        Err(DaemonCoreError::LockPathIsSymlink)
    ));

    let _ = fs::remove_file(runtime.join("verifier.lock"));
    let _ = fs::remove_file(target);
    cleanup_tree(&runtime);
}

#[test]
fn socket_symlink_is_denied() {
    let (uid, gid) = current_ids();
    let runtime = temp_path("socket-link");
    private_dir(&runtime);
    let target = runtime.join("target-file");
    fs::write(&target, b"target").expect("target");
    symlink(&target, runtime.join("verifier.sock")).expect("socket symlink");

    let config = DaemonRuntimeConfigV1::new(&runtime, uid, gid);
    assert!(matches!(
        VerifierDaemonLeaseV1::acquire(config),
        Err(DaemonCoreError::SocketPathIsSymlink)
    ));

    let _ = fs::remove_file(runtime.join("verifier.sock"));
    let _ = fs::remove_file(target);
    cleanup_tree(&runtime);
}

#[test]
fn configured_owner_mismatch_is_denied() {
    let (uid, gid) = current_ids();
    let runtime = temp_path("owner-mismatch");
    private_dir(&runtime);
    let wrong_uid = uid.wrapping_add(1);
    let config = DaemonRuntimeConfigV1::new(&runtime, wrong_uid, gid);

    assert!(matches!(
        VerifierDaemonLeaseV1::acquire(config),
        Err(DaemonCoreError::RuntimeOwnershipMismatch)
    ));
    cleanup_tree(&runtime);
}
