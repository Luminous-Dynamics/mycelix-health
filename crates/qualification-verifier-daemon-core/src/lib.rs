#![forbid(unsafe_code)]
//! Linux/Unix process-ownership and bounded RPC framing core for the isolated
//! Mycelix Health qualification verifier daemon.
//!
//! QUAL-EVID-009C4A deliberately proves only:
//! - one kernel-managed exclusive daemon lease for one private runtime directory;
//! - strict runtime/lock/socket filesystem metadata rules;
//! - stale Unix-socket recovery only while that exclusive lease is held; and
//! - deterministic, bounded, versioned binary RPC framing.
//!
//! It does **not** establish peer identity, capability authentication, request
//! authorization, durable mint ordering, or product trust.

#[cfg(not(unix))]
compile_error!("qualification-verifier-daemon-core V1 is Unix-only");

use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::{
    DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt,
};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use fs2::FileExt;

pub const RPC_SCHEMA_V1: u16 = 1;
pub const MAX_RPC_PAYLOAD_V1: usize = 64 * 1024;
pub const REQUEST_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-DAEMON-REQUEST-V1\0";
pub const RESPONSE_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-DAEMON-RESPONSE-V1\0";
pub const REQUEST_HEADER_LEN_V1: usize = REQUEST_DOMAIN_V1.len() + 2 + 1 + 32 + 4;
pub const RESPONSE_HEADER_LEN_V1: usize = RESPONSE_DOMAIN_V1.len() + 2 + 1 + 1 + 32 + 4;

const LOCK_FILE_NAME_V1: &str = "verifier.lock";
const SOCKET_FILE_NAME_V1: &str = "verifier.sock";
const LOCK_MODE_V1: u32 = 0o600;
const SOCKET_MODE_V1: u32 = 0o600;
const RUNTIME_MODE_V1: u32 = 0o700;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaemonRuntimeConfigV1 {
    runtime_dir: PathBuf,
    expected_uid: u32,
    expected_gid: u32,
}

impl DaemonRuntimeConfigV1 {
    pub fn new(runtime_dir: impl Into<PathBuf>, expected_uid: u32, expected_gid: u32) -> Self {
        Self {
            runtime_dir: runtime_dir.into(),
            expected_uid,
            expected_gid,
        }
    }

    pub fn runtime_dir(&self) -> &Path {
        &self.runtime_dir
    }

    pub fn expected_uid(&self) -> u32 {
        self.expected_uid
    }

    pub fn expected_gid(&self) -> u32 {
        self.expected_gid
    }

    pub fn lock_path(&self) -> PathBuf {
        self.runtime_dir.join(LOCK_FILE_NAME_V1)
    }

    pub fn socket_path(&self) -> PathBuf {
        self.runtime_dir.join(SOCKET_FILE_NAME_V1)
    }
}

/// Holds both the kernel process lease and the bound Unix endpoint.
///
/// The raw listener is intentionally private. A later authenticated transport
/// child must add the accept/read loop without exposing an unauthenticated
/// bypass from this crate.
pub struct VerifierDaemonLeaseV1 {
    config: DaemonRuntimeConfigV1,
    _lock_file: File,
    _listener: UnixListener,
}

impl VerifierDaemonLeaseV1 {
    pub fn acquire(config: DaemonRuntimeConfigV1) -> Result<Self, DaemonCoreError> {
        ensure_runtime_directory(&config)?;
        let mut lock_file = open_secure_lock_file(&config)?;

        match FileExt::try_lock_exclusive(&lock_file) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                return Err(DaemonCoreError::InstanceAlreadyRunning)
            }
            Err(error) => return Err(DaemonCoreError::Io(error)),
        }

        // Diagnostic only. Lock authority is the live kernel lock above.
        lock_file.set_len(0)?;
        lock_file.seek(SeekFrom::Start(0))?;
        writeln!(lock_file, "schema={RPC_SCHEMA_V1}")?;
        writeln!(lock_file, "pid={}", std::process::id())?;
        lock_file.sync_data()?;

        prepare_socket_path_under_lease(&config)?;
        let listener = UnixListener::bind(config.socket_path())?;
        fs::set_permissions(
            config.socket_path(),
            fs::Permissions::from_mode(SOCKET_MODE_V1),
        )?;
        validate_socket_metadata(&config)?;

        Ok(Self {
            config,
            _lock_file: lock_file,
            _listener: listener,
        })
    }

    pub fn runtime_dir(&self) -> &Path {
        self.config.runtime_dir()
    }

    pub fn socket_path(&self) -> PathBuf {
        self.config.socket_path()
    }

    pub fn lock_path(&self) -> PathBuf {
        self.config.lock_path()
    }
}

impl Drop for VerifierDaemonLeaseV1 {
    fn drop(&mut self) {
        let socket_path = self.config.socket_path();
        if let Ok(metadata) = fs::symlink_metadata(&socket_path) {
            if metadata.file_type().is_socket() {
                let _ = fs::remove_file(socket_path);
            }
        }
        // Closing `_lock_file` releases the kernel-managed advisory lock.
    }
}

fn ensure_runtime_directory(config: &DaemonRuntimeConfigV1) -> Result<(), DaemonCoreError> {
    match fs::symlink_metadata(config.runtime_dir()) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut builder = DirBuilder::new();
            builder.mode(RUNTIME_MODE_V1);
            match builder.create(config.runtime_dir()) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(DaemonCoreError::Io(error)),
            }
        }
        Err(error) => return Err(DaemonCoreError::Io(error)),
    }

    let metadata = fs::symlink_metadata(config.runtime_dir())?;
    if metadata.file_type().is_symlink() {
        return Err(DaemonCoreError::RuntimePathIsSymlink);
    }
    if !metadata.is_dir() {
        return Err(DaemonCoreError::RuntimePathIsNotDirectory);
    }
    if metadata.uid() != config.expected_uid || metadata.gid() != config.expected_gid {
        return Err(DaemonCoreError::RuntimeOwnershipMismatch);
    }
    if metadata.permissions().mode() & 0o777 != RUNTIME_MODE_V1 {
        return Err(DaemonCoreError::RuntimePermissionsMismatch);
    }
    Ok(())
}

fn open_secure_lock_file(config: &DaemonRuntimeConfigV1) -> Result<File, DaemonCoreError> {
    let path = config.lock_path();
    match fs::symlink_metadata(&path) {
        Ok(metadata) => validate_lock_metadata(config, &metadata)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(DaemonCoreError::Io(error)),
    }

    let file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(LOCK_MODE_V1)
        .open(&path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = fs::symlink_metadata(&path)?;
            validate_lock_metadata(config, &metadata)?;
            OpenOptions::new().read(true).write(true).open(&path)?
        }
        Err(error) => return Err(DaemonCoreError::Io(error)),
    };

    fs::set_permissions(&path, fs::Permissions::from_mode(LOCK_MODE_V1))?;
    let metadata = fs::symlink_metadata(&path)?;
    validate_lock_metadata(config, &metadata)?;
    Ok(file)
}

fn validate_lock_metadata(
    config: &DaemonRuntimeConfigV1,
    metadata: &fs::Metadata,
) -> Result<(), DaemonCoreError> {
    if metadata.file_type().is_symlink() {
        return Err(DaemonCoreError::LockPathIsSymlink);
    }
    if !metadata.is_file() {
        return Err(DaemonCoreError::LockPathIsNotRegularFile);
    }
    if metadata.uid() != config.expected_uid || metadata.gid() != config.expected_gid {
        return Err(DaemonCoreError::LockOwnershipMismatch);
    }
    if metadata.permissions().mode() & 0o777 != LOCK_MODE_V1 {
        return Err(DaemonCoreError::LockPermissionsMismatch);
    }
    Ok(())
}

fn prepare_socket_path_under_lease(config: &DaemonRuntimeConfigV1) -> Result<(), DaemonCoreError> {
    let path = config.socket_path();
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(DaemonCoreError::Io(error)),
    };

    if metadata.file_type().is_symlink() {
        return Err(DaemonCoreError::SocketPathIsSymlink);
    }
    if !metadata.file_type().is_socket() {
        return Err(DaemonCoreError::SocketPathIsNotSocket);
    }
    if metadata.uid() != config.expected_uid || metadata.gid() != config.expected_gid {
        return Err(DaemonCoreError::SocketOwnershipMismatch);
    }

    match UnixStream::connect(&path) {
        Ok(_) => Err(DaemonCoreError::PreexistingLiveSocket),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
            ) =>
        {
            fs::remove_file(path)?;
            Ok(())
        }
        Err(error) => Err(DaemonCoreError::SocketProbeFailed(error.kind())),
    }
}

fn validate_socket_metadata(config: &DaemonRuntimeConfigV1) -> Result<(), DaemonCoreError> {
    let metadata = fs::symlink_metadata(config.socket_path())?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_socket() {
        return Err(DaemonCoreError::SocketPathIsNotSocket);
    }
    if metadata.uid() != config.expected_uid || metadata.gid() != config.expected_gid {
        return Err(DaemonCoreError::SocketOwnershipMismatch);
    }
    if metadata.permissions().mode() & 0o777 != SOCKET_MODE_V1 {
        return Err(DaemonCoreError::SocketPermissionsMismatch);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RpcOperationV1 {
    Challenge = 1,
    Mint = 2,
    Status = 3,
}

impl RpcOperationV1 {
    fn from_u8(value: u8) -> Result<Self, DaemonCoreError> {
        match value {
            1 => Ok(Self::Challenge),
            2 => Ok(Self::Mint),
            3 => Ok(Self::Status),
            _ => Err(DaemonCoreError::UnknownOperation),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RpcResultClassV1 {
    Ok = 1,
    Error = 2,
}

impl RpcResultClassV1 {
    fn from_u8(value: u8) -> Result<Self, DaemonCoreError> {
        match value {
            1 => Ok(Self::Ok),
            2 => Ok(Self::Error),
            _ => Err(DaemonCoreError::UnknownResultClass),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RpcRequestIdV1([u8; 32]);

impl RpcRequestIdV1 {
    pub fn new(bytes: [u8; 32]) -> Result<Self, DaemonCoreError> {
        if bytes == [0; 32] {
            return Err(DaemonCoreError::ZeroRequestId);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for RpcRequestIdV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RpcRequestIdV1([redacted])")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpcRequestHeaderV1 {
    operation: RpcOperationV1,
    request_id: RpcRequestIdV1,
    payload_len: u32,
}

impl RpcRequestHeaderV1 {
    pub fn operation(&self) -> RpcOperationV1 {
        self.operation
    }

    pub fn request_id(&self) -> RpcRequestIdV1 {
        self.request_id
    }

    pub fn payload_len(&self) -> u32 {
        self.payload_len
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaemonRpcRequestV1 {
    operation: RpcOperationV1,
    request_id: RpcRequestIdV1,
    payload: Vec<u8>,
}

impl DaemonRpcRequestV1 {
    pub fn new(
        operation: RpcOperationV1,
        request_id: RpcRequestIdV1,
        payload: Vec<u8>,
    ) -> Result<Self, DaemonCoreError> {
        validate_request_payload_shape(operation, payload.len())?;
        Ok(Self {
            operation,
            request_id,
            payload,
        })
    }

    pub fn operation(&self) -> RpcOperationV1 {
        self.operation
    }

    pub fn request_id(&self) -> RpcRequestIdV1 {
        self.request_id
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(REQUEST_HEADER_LEN_V1 + self.payload.len());
        out.extend_from_slice(REQUEST_DOMAIN_V1);
        out.extend_from_slice(&RPC_SCHEMA_V1.to_be_bytes());
        out.push(self.operation as u8);
        out.extend_from_slice(self.request_id.as_bytes());
        out.extend_from_slice(&(self.payload.len() as u32).to_be_bytes());
        out.extend_from_slice(&self.payload);
        out
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaemonRpcResponseV1 {
    operation: RpcOperationV1,
    result: RpcResultClassV1,
    request_id: RpcRequestIdV1,
    payload: Vec<u8>,
}

impl DaemonRpcResponseV1 {
    pub fn new(
        operation: RpcOperationV1,
        result: RpcResultClassV1,
        request_id: RpcRequestIdV1,
        payload: Vec<u8>,
    ) -> Result<Self, DaemonCoreError> {
        validate_payload_bound(payload.len())?;
        Ok(Self {
            operation,
            result,
            request_id,
            payload,
        })
    }

    pub fn operation(&self) -> RpcOperationV1 {
        self.operation
    }

    pub fn result(&self) -> RpcResultClassV1 {
        self.result
    }

    pub fn request_id(&self) -> RpcRequestIdV1 {
        self.request_id
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(RESPONSE_HEADER_LEN_V1 + self.payload.len());
        out.extend_from_slice(RESPONSE_DOMAIN_V1);
        out.extend_from_slice(&RPC_SCHEMA_V1.to_be_bytes());
        out.push(self.operation as u8);
        out.push(self.result as u8);
        out.extend_from_slice(self.request_id.as_bytes());
        out.extend_from_slice(&(self.payload.len() as u32).to_be_bytes());
        out.extend_from_slice(&self.payload);
        out
    }
}

pub fn decode_request_header_v1(bytes: &[u8]) -> Result<RpcRequestHeaderV1, DaemonCoreError> {
    if bytes.len() != REQUEST_HEADER_LEN_V1 {
        return Err(DaemonCoreError::InvalidHeaderLength);
    }
    let mut reader = Reader::new(bytes);
    reader.expect(REQUEST_DOMAIN_V1)?;
    if reader.read_u16()? != RPC_SCHEMA_V1 {
        return Err(DaemonCoreError::UnsupportedRpcSchema);
    }
    let operation = RpcOperationV1::from_u8(reader.read_u8()?)?;
    let request_id = RpcRequestIdV1::new(reader.read_array_32()?)?;
    let payload_len = reader.read_u32()?;
    validate_request_payload_shape(operation, payload_len as usize)?;
    if !reader.finished() {
        return Err(DaemonCoreError::TrailingBytes);
    }
    Ok(RpcRequestHeaderV1 {
        operation,
        request_id,
        payload_len,
    })
}

pub fn decode_request_v1(bytes: &[u8]) -> Result<DaemonRpcRequestV1, DaemonCoreError> {
    if bytes.len() < REQUEST_HEADER_LEN_V1 {
        return Err(DaemonCoreError::TruncatedInput);
    }
    let header = decode_request_header_v1(&bytes[..REQUEST_HEADER_LEN_V1])?;
    let expected = REQUEST_HEADER_LEN_V1
        .checked_add(header.payload_len as usize)
        .ok_or(DaemonCoreError::PayloadTooLarge)?;
    if bytes.len() < expected {
        return Err(DaemonCoreError::TruncatedInput);
    }
    if bytes.len() > expected {
        return Err(DaemonCoreError::TrailingBytes);
    }
    DaemonRpcRequestV1::new(
        header.operation,
        header.request_id,
        bytes[REQUEST_HEADER_LEN_V1..expected].to_vec(),
    )
}

pub fn decode_response_v1(bytes: &[u8]) -> Result<DaemonRpcResponseV1, DaemonCoreError> {
    if bytes.len() < RESPONSE_HEADER_LEN_V1 {
        return Err(DaemonCoreError::TruncatedInput);
    }
    let mut reader = Reader::new(&bytes[..RESPONSE_HEADER_LEN_V1]);
    reader.expect(RESPONSE_DOMAIN_V1)?;
    if reader.read_u16()? != RPC_SCHEMA_V1 {
        return Err(DaemonCoreError::UnsupportedRpcSchema);
    }
    let operation = RpcOperationV1::from_u8(reader.read_u8()?)?;
    let result = RpcResultClassV1::from_u8(reader.read_u8()?)?;
    let request_id = RpcRequestIdV1::new(reader.read_array_32()?)?;
    let payload_len = reader.read_u32()? as usize;
    validate_payload_bound(payload_len)?;
    let expected = RESPONSE_HEADER_LEN_V1
        .checked_add(payload_len)
        .ok_or(DaemonCoreError::PayloadTooLarge)?;
    if bytes.len() < expected {
        return Err(DaemonCoreError::TruncatedInput);
    }
    if bytes.len() > expected {
        return Err(DaemonCoreError::TrailingBytes);
    }
    DaemonRpcResponseV1::new(
        operation,
        result,
        request_id,
        bytes[RESPONSE_HEADER_LEN_V1..expected].to_vec(),
    )
}

fn validate_request_payload_shape(
    operation: RpcOperationV1,
    payload_len: usize,
) -> Result<(), DaemonCoreError> {
    validate_payload_bound(payload_len)?;
    match operation {
        RpcOperationV1::Challenge | RpcOperationV1::Status if payload_len != 0 => {
            Err(DaemonCoreError::UnexpectedRequestPayload)
        }
        RpcOperationV1::Mint if payload_len == 0 => Err(DaemonCoreError::MissingMintPayload),
        _ => Ok(()),
    }
}

fn validate_payload_bound(payload_len: usize) -> Result<(), DaemonCoreError> {
    if payload_len > MAX_RPC_PAYLOAD_V1 {
        return Err(DaemonCoreError::PayloadTooLarge);
    }
    Ok(())
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn finished(&self) -> bool {
        self.pos == self.bytes.len()
    }

    fn read(&mut self, len: usize) -> Result<&'a [u8], DaemonCoreError> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or(DaemonCoreError::TruncatedInput)?;
        if end > self.bytes.len() {
            return Err(DaemonCoreError::TruncatedInput);
        }
        let out = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    fn expect(&mut self, expected: &[u8]) -> Result<(), DaemonCoreError> {
        if self.read(expected.len())? != expected {
            return Err(DaemonCoreError::RpcDomainMismatch);
        }
        Ok(())
    }

    fn read_u8(&mut self) -> Result<u8, DaemonCoreError> {
        Ok(self.read(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, DaemonCoreError> {
        Ok(u16::from_be_bytes(
            self.read(2)?
                .try_into()
                .map_err(|_| DaemonCoreError::TruncatedInput)?,
        ))
    }

    fn read_u32(&mut self) -> Result<u32, DaemonCoreError> {
        Ok(u32::from_be_bytes(
            self.read(4)?
                .try_into()
                .map_err(|_| DaemonCoreError::TruncatedInput)?,
        ))
    }

    fn read_array_32(&mut self) -> Result<[u8; 32], DaemonCoreError> {
        self.read(32)?
            .try_into()
            .map_err(|_| DaemonCoreError::TruncatedInput)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DaemonCoreError {
    #[error("another verifier daemon already owns the process lease")]
    InstanceAlreadyRunning,
    #[error("runtime path is a symlink")]
    RuntimePathIsSymlink,
    #[error("runtime path is not a directory")]
    RuntimePathIsNotDirectory,
    #[error("runtime directory ownership does not match configured uid/gid")]
    RuntimeOwnershipMismatch,
    #[error("runtime directory permissions must be exactly 0700")]
    RuntimePermissionsMismatch,
    #[error("lock path is a symlink")]
    LockPathIsSymlink,
    #[error("lock path is not a regular file")]
    LockPathIsNotRegularFile,
    #[error("lock file ownership does not match configured uid/gid")]
    LockOwnershipMismatch,
    #[error("lock file permissions must be exactly 0600")]
    LockPermissionsMismatch,
    #[error("socket path is a symlink")]
    SocketPathIsSymlink,
    #[error("pre-existing socket path is not a Unix socket")]
    SocketPathIsNotSocket,
    #[error("socket ownership does not match configured uid/gid")]
    SocketOwnershipMismatch,
    #[error("socket permissions must be exactly 0600")]
    SocketPermissionsMismatch,
    #[error("a live Unix socket already exists under the acquired runtime lease")]
    PreexistingLiveSocket,
    #[error("could not safely classify stale Unix socket: {0:?}")]
    SocketProbeFailed(std::io::ErrorKind),
    #[error("RPC domain mismatch")]
    RpcDomainMismatch,
    #[error("unsupported RPC schema")]
    UnsupportedRpcSchema,
    #[error("unknown RPC operation")]
    UnknownOperation,
    #[error("unknown RPC result class")]
    UnknownResultClass,
    #[error("RPC request id must be nonzero")]
    ZeroRequestId,
    #[error("RPC header has the wrong exact length")]
    InvalidHeaderLength,
    #[error("RPC payload exceeds V1 hard limit")]
    PayloadTooLarge,
    #[error("challenge/status request must not contain a payload")]
    UnexpectedRequestPayload,
    #[error("mint request requires a non-empty payload")]
    MissingMintPayload,
    #[error("truncated RPC input")]
    TruncatedInput,
    #[error("unexpected trailing RPC bytes")]
    TrailingBytes,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(1);

    fn temp_path(name: &str) -> PathBuf {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "mycelix-verifier-daemon-{name}-{}-{id}",
            std::process::id()
        ))
    }

    fn current_ids() -> (u32, u32) {
        let probe = temp_path("identity-probe");
        fs::write(&probe, b"probe").expect("write probe");
        let metadata = fs::metadata(&probe).expect("metadata");
        let ids = (metadata.uid(), metadata.gid());
        fs::remove_file(probe).expect("remove probe");
        ids
    }

    fn config(name: &str) -> DaemonRuntimeConfigV1 {
        let (uid, gid) = current_ids();
        DaemonRuntimeConfigV1::new(temp_path(name), uid, gid)
    }

    fn cleanup(config: &DaemonRuntimeConfigV1) {
        let _ = fs::remove_file(config.socket_path());
        let _ = fs::remove_file(config.lock_path());
        let _ = fs::remove_dir(config.runtime_dir());
    }

    fn id(value: u8) -> RpcRequestIdV1 {
        RpcRequestIdV1::new([value; 32]).expect("nonzero id")
    }

    #[test]
    fn first_daemon_owns_private_runtime_and_second_is_denied() {
        let config = config("singleton");
        cleanup(&config);
        let first = VerifierDaemonLeaseV1::acquire(config.clone()).expect("first lease");
        let runtime = fs::symlink_metadata(first.runtime_dir()).expect("runtime metadata");
        let lock = fs::symlink_metadata(first.lock_path()).expect("lock metadata");
        let socket = fs::symlink_metadata(first.socket_path()).expect("socket metadata");
        assert_eq!(runtime.permissions().mode() & 0o777, RUNTIME_MODE_V1);
        assert_eq!(lock.permissions().mode() & 0o777, LOCK_MODE_V1);
        assert_eq!(socket.permissions().mode() & 0o777, SOCKET_MODE_V1);
        assert!(socket.file_type().is_socket());

        assert!(matches!(
            VerifierDaemonLeaseV1::acquire(config.clone()),
            Err(DaemonCoreError::InstanceAlreadyRunning)
        ));
        drop(first);
        cleanup(&config);
    }

    #[test]
    fn stale_socket_is_removed_only_under_exclusive_lease() {
        let config = config("stale-socket");
        cleanup(&config);
        let mut builder = DirBuilder::new();
        builder.mode(RUNTIME_MODE_V1);
        builder.create(config.runtime_dir()).expect("runtime");
        let stale = UnixListener::bind(config.socket_path()).expect("bind stale");
        drop(stale);
        assert!(fs::symlink_metadata(config.socket_path()).unwrap().file_type().is_socket());

        let lease = VerifierDaemonLeaseV1::acquire(config.clone()).expect("recover stale");
        assert!(fs::symlink_metadata(lease.socket_path()).unwrap().file_type().is_socket());
        drop(lease);
        cleanup(&config);
    }

    #[test]
    fn live_preexisting_socket_denies_startup() {
        let config = config("live-socket");
        cleanup(&config);
        let mut builder = DirBuilder::new();
        builder.mode(RUNTIME_MODE_V1);
        builder.create(config.runtime_dir()).expect("runtime");
        let live = UnixListener::bind(config.socket_path()).expect("bind live");
        fs::set_permissions(config.socket_path(), fs::Permissions::from_mode(SOCKET_MODE_V1))
            .expect("socket mode");

        assert!(matches!(
            VerifierDaemonLeaseV1::acquire(config.clone()),
            Err(DaemonCoreError::PreexistingLiveSocket)
        ));
        drop(live);
        cleanup(&config);
    }

    #[test]
    fn insecure_runtime_directory_is_denied() {
        let config = config("insecure-dir");
        cleanup(&config);
        fs::create_dir(config.runtime_dir()).expect("dir");
        fs::set_permissions(config.runtime_dir(), fs::Permissions::from_mode(0o755)).expect("mode");
        assert!(matches!(
            VerifierDaemonLeaseV1::acquire(config.clone()),
            Err(DaemonCoreError::RuntimePermissionsMismatch)
        ));
        cleanup(&config);
    }

    #[test]
    fn preexisting_regular_file_at_socket_path_is_denied() {
        let config = config("socket-file");
        cleanup(&config);
        let mut builder = DirBuilder::new();
        builder.mode(RUNTIME_MODE_V1);
        builder.create(config.runtime_dir()).expect("runtime");
        fs::write(config.socket_path(), b"not a socket").expect("file");
        assert!(matches!(
            VerifierDaemonLeaseV1::acquire(config.clone()),
            Err(DaemonCoreError::SocketPathIsNotSocket)
        ));
        cleanup(&config);
    }

    #[test]
    fn request_header_rejects_oversized_body_before_body_is_present() {
        let mut header = Vec::new();
        header.extend_from_slice(REQUEST_DOMAIN_V1);
        header.extend_from_slice(&RPC_SCHEMA_V1.to_be_bytes());
        header.push(RpcOperationV1::Mint as u8);
        header.extend_from_slice(id(1).as_bytes());
        header.extend_from_slice(&((MAX_RPC_PAYLOAD_V1 as u32) + 1).to_be_bytes());
        assert_eq!(header.len(), REQUEST_HEADER_LEN_V1);
        assert!(matches!(
            decode_request_header_v1(&header),
            Err(DaemonCoreError::PayloadTooLarge)
        ));
    }

    #[test]
    fn request_round_trip_is_exact_and_trailing_bytes_fail() {
        let request = DaemonRpcRequestV1::new(RpcOperationV1::Mint, id(2), vec![7, 8, 9])
            .expect("request");
        let bytes = request.canonical_bytes();
        assert_eq!(decode_request_v1(&bytes).expect("decode"), request);

        let mut trailing = bytes;
        trailing.push(0);
        assert!(matches!(
            decode_request_v1(&trailing),
            Err(DaemonCoreError::TrailingBytes)
        ));
    }

    #[test]
    fn challenge_and_status_require_empty_payload_and_mint_requires_body() {
        assert!(DaemonRpcRequestV1::new(RpcOperationV1::Challenge, id(3), vec![]).is_ok());
        assert!(DaemonRpcRequestV1::new(RpcOperationV1::Status, id(4), vec![]).is_ok());
        assert!(matches!(
            DaemonRpcRequestV1::new(RpcOperationV1::Challenge, id(5), vec![1]),
            Err(DaemonCoreError::UnexpectedRequestPayload)
        ));
        assert!(matches!(
            DaemonRpcRequestV1::new(RpcOperationV1::Mint, id(6), vec![]),
            Err(DaemonCoreError::MissingMintPayload)
        ));
    }

    #[test]
    fn unknown_schema_operation_zero_id_and_truncation_are_denied() {
        let request = DaemonRpcRequestV1::new(RpcOperationV1::Mint, id(7), vec![1])
            .expect("request");
        let baseline = request.canonical_bytes();

        let mut wrong_schema = baseline.clone();
        let schema_offset = REQUEST_DOMAIN_V1.len();
        wrong_schema[schema_offset + 1] ^= 1;
        assert!(matches!(
            decode_request_v1(&wrong_schema),
            Err(DaemonCoreError::UnsupportedRpcSchema)
        ));

        let mut wrong_op = baseline.clone();
        wrong_op[REQUEST_DOMAIN_V1.len() + 2] = 99;
        assert!(matches!(
            decode_request_v1(&wrong_op),
            Err(DaemonCoreError::UnknownOperation)
        ));

        let mut zero_id = baseline.clone();
        let id_start = REQUEST_DOMAIN_V1.len() + 3;
        zero_id[id_start..id_start + 32].fill(0);
        assert!(matches!(
            decode_request_v1(&zero_id),
            Err(DaemonCoreError::ZeroRequestId)
        ));

        assert!(matches!(
            decode_request_v1(&baseline[..baseline.len() - 1]),
            Err(DaemonCoreError::TruncatedInput)
        ));
    }

    #[test]
    fn response_round_trip_binds_operation_result_and_request_id() {
        let response = DaemonRpcResponseV1::new(
            RpcOperationV1::Status,
            RpcResultClassV1::Ok,
            id(8),
            vec![1, 2, 3],
        )
        .expect("response");
        let bytes = response.canonical_bytes();
        assert_eq!(decode_response_v1(&bytes).expect("decode"), response);

        let mut trailing = bytes;
        trailing.push(0);
        assert!(matches!(
            decode_response_v1(&trailing),
            Err(DaemonCoreError::TrailingBytes)
        ));
    }
}
