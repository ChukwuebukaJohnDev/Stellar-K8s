//! Captive Core Process Lifecycle Management
//!
//! This module provides process lifecycle management and supervision for Stellar's
//! Captive Core embedded in Soroban RPC instances.
//!
//! # Overview
//!
//! The Captive Core supervision system provides:
//!
//! - **Process Lifecycle Management** ([`process::CaptiveCoreProcess`]):
//!   - Process spawning with controlled arguments
//!   - Graceful termination (SIGTERM with timeout)
//!   - Forced termination (SIGKILL) fallback
//!   - Lock file management and cleanup
//!
//! - **Health Monitoring** ([`supervisor::CaptiveCoreSupervisor`]):
//!   - Continuous process health checks
//!   - IPC responsiveness monitoring
//!   - Stale lock detection and cleanup
//!   - Automatic recovery on process crashes
//!   - Graceful recovery from frozen IPC states
//!
//! # Architecture
//!
//! The supervisor runs in a dedicated background task that periodically:
//!
//! 1. Checks if the process is running
//! 2. Verifies IPC is responsive
//! 3. Detects and cleans up stale lock files
//! 4. Initiates automatic recovery if process crashes
//! 5. Performs graceful shutdown followed by forced termination if needed
//!
//! # Example
//!
//! ```ignore
//! use stellar_k8s::controller::captive::supervisor::{CaptiveCoreSupervisor, SupervisorConfig};
//! use std::path::PathBuf;
//!
//! let config = SupervisorConfig {
//!     core_binary: PathBuf::from("/usr/bin/stellar-core"),
//!     lock_path: PathBuf::from("/var/lib/stellar/core.lock"),
//!     enable_auto_restart: true,
//!     ..Default::default()
//! };
//!
//! let supervisor = CaptiveCoreSupervisor::new(config);
//! supervisor.start().await?;
//! ```
//!
//! # Lock File Management
//!
//! The module implements safe lock file removal that:
//! - Verifies the process in the lock file is actually terminated
//! - Only removes locks when confirmed stale
//! - Prevents dual-process storage corruption
//! - Logs all lock file operations for audit trail

pub mod process;
pub mod supervisor;

pub use process::{CaptiveCoreProcess, LockFileInfo, ProcessState, CORE_LOCK_PATH};
pub use supervisor::{CaptiveCoreSupervisor, SupervisorConfig, SupervisorState};
