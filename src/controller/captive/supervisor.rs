//! Captive Core Process Lifecycle Supervisor
//!
//! Implements a dedicated supervisor thread that monitors Captive Core process health,
//! detects frozen IPC states, and manages automatic recovery including stale lock cleanup.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::RwLock;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

use crate::error::{Error, Result};
use super::process::{CaptiveCoreProcess, ProcessState, GRACEFUL_SHUTDOWN_TIMEOUT};

/// Configuration for the Captive Core supervisor
#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    /// Path to Captive Core binary
    pub core_binary: PathBuf,
    /// Path to lock file
    pub lock_path: PathBuf,
    /// Health check interval
    pub health_check_interval: Duration,
    /// IPC timeout - if no response in this duration, consider IPC frozen
    pub ipc_timeout: Duration,
    /// Maximum time to wait before forcing termination
    pub force_termination_timeout: Duration,
    /// Enable automatic restart on failure
    pub enable_auto_restart: bool,
    /// Maximum restart attempts before giving up
    pub max_restart_attempts: u32,
    /// Arguments to pass to Captive Core
    pub core_args: Vec<String>,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            core_binary: PathBuf::from("stellar-core"),
            lock_path: PathBuf::from("/var/lib/stellar/core.lock"),
            health_check_interval: Duration::from_secs(5),
            ipc_timeout: Duration::from_secs(30),
            force_termination_timeout: GRACEFUL_SHUTDOWN_TIMEOUT,
            enable_auto_restart: true,
            max_restart_attempts: 3,
            core_args: vec![],
        }
    }
}

/// Supervisor state tracking
#[derive(Debug, Clone)]
pub struct SupervisorState {
    /// Last successful health check timestamp
    pub last_healthy_check: Option<SystemTime>,
    /// Number of consecutive failed health checks
    pub consecutive_failures: u32,
    /// Number of restart attempts
    pub restart_attempts: u32,
    /// Whether supervisor is running
    pub is_running: bool,
    /// Last error message
    pub last_error: Option<String>,
    /// Whether IPC is responsive
    pub ipc_responsive: bool,
}

impl Default for SupervisorState {
    fn default() -> Self {
        Self {
            last_healthy_check: None,
            consecutive_failures: 0,
            restart_attempts: 0,
            is_running: true,
            last_error: None,
            ipc_responsive: false,
        }
    }
}

/// Captive Core Supervisor
///
/// Monitors process health and manages recovery workflows.
pub struct CaptiveCoreSupervisor {
    config: SupervisorConfig,
    process: Arc<RwLock<CaptiveCoreProcess>>,
    state: Arc<RwLock<SupervisorState>>,
}

impl CaptiveCoreSupervisor {
    /// Create a new supervisor instance
    pub fn new(config: SupervisorConfig) -> Self {
        let process = CaptiveCoreProcess::new(
            config.core_binary.clone(),
            Some(config.lock_path.clone()),
        );

        Self {
            config,
            process: Arc::new(RwLock::new(process)),
            state: Arc::new(RwLock::new(SupervisorState::default())),
        }
    }

    /// Start the supervisor (spawns monitoring thread)
    pub async fn start(&self) -> Result<()> {
        info!("Starting Captive Core supervisor");

        let process = self.process.clone();
        let state = self.state.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            Self::run_supervisor_loop(process, state, config).await
        });

        Ok(())
    }

    /// Main supervisor loop
    async fn run_supervisor_loop(
        process: Arc<RwLock<CaptiveCoreProcess>>,
        state: Arc<RwLock<SupervisorState>>,
        config: SupervisorConfig,
    ) {
        let mut check_interval = interval(config.health_check_interval);

        loop {
            check_interval.tick().await;

            if let Err(e) = Self::perform_health_check(&process, &state, &config).await {
                error!("Health check failed: {}", e);
                let mut s = state.write().await;
                s.last_error = Some(e.to_string());
            }
        }
    }

    /// Perform a single health check
    async fn perform_health_check(
        process: Arc<RwLock<CaptiveCoreProcess>>,
        state: Arc<RwLock<SupervisorState>>,
        config: &SupervisorConfig,
    ) -> Result<()> {
        let mut proc = process.write().await;
        let mut sup_state = state.write().await;

        debug!("Performing health check");

        // Check if process is still running
        let proc_state = proc.state();
        if proc_state == ProcessState::Stopped {
            sup_state.ipc_responsive = false;
            sup_state.consecutive_failures += 1;

            if config.enable_auto_restart && sup_state.restart_attempts < config.max_restart_attempts {
                warn!(
                    "Process is not running, attempting restart (attempt {}/{})",
                    sup_state.restart_attempts + 1,
                    config.max_restart_attempts
                );

                // Try to restart
                if let Err(e) = proc.restart(config.core_args.clone()).await {
                    error!("Failed to restart process: {}", e);
                    sup_state.last_error = Some(format!("Restart failed: {}", e));
                    sup_state.restart_attempts += 1;
                } else {
                    info!("Process restarted successfully");
                    sup_state.consecutive_failures = 0;
                    sup_state.restart_attempts = 0;
                    sup_state.last_healthy_check = Some(SystemTime::now());
                    sup_state.ipc_responsive = true;
                }
            } else if sup_state.restart_attempts >= config.max_restart_attempts {
                error!("Max restart attempts reached, supervisor giving up");
                sup_state.is_running = false;
            }

            return Ok(());
        }

        // Try to perform IPC health check (detect frozen IPC)
        if let Err(e) = Self::check_ipc_health(&proc).await {
            warn!("IPC health check failed: {}", e);
            sup_state.ipc_responsive = false;
            sup_state.consecutive_failures += 1;

            // If IPC is frozen, attempt recovery
            if sup_state.consecutive_failures >= 3 {
                info!("IPC appears to be frozen, initiating recovery");
                drop(sup_state); // Release lock before calling methods that need to write-lock
                Self::recover_frozen_ipc(&mut proc).await?;
                let mut sup_state = state.write().await;
                sup_state.consecutive_failures = 0;
            }
        } else {
            // IPC is responsive
            sup_state.last_healthy_check = Some(SystemTime::now());
            sup_state.consecutive_failures = 0;
            sup_state.ipc_responsive = true;
            debug!("IPC health check passed");
        }

        // Check for stale lock files
        if let Ok(is_stale) = proc.is_lock_stale().await {
            if is_stale {
                warn!("Detected stale lock file, attempting cleanup");
                if let Err(e) = proc.remove_stale_lock().await {
                    error!("Failed to remove stale lock: {}", e);
                    sup_state.last_error = Some(format!("Lock cleanup failed: {}", e));
                }
            }
        }

        Ok(())
    }

    /// Check IPC responsiveness
    ///
    /// In a real scenario, this would check for actual IPC communication.
    /// For now, we check if the process is running and the lock file is accessible.
    async fn check_ipc_health(process: &CaptiveCoreProcess) -> Result<()> {
        // Verify process is still running
        if process.pid().is_none() {
            return Err(Error::Other("Process has no PID".to_string()));
        }

        // Check lock file is accessible (IPC is healthy if lock is up-to-date)
        let lock_info = process.check_lock_file().await?;

        if !lock_info.exists {
            return Err(Error::Other("Lock file does not exist".to_string()));
        }

        // If lock file is very recent (updated in the last 10 seconds), IPC is healthy
        if let Some(age) = lock_info.age {
            if age > Duration::from_secs(10) {
                return Err(Error::Other(format!(
                    "Lock file is stale (age: {:?})",
                    age
                )));
            }
        }

        Ok(())
    }

    /// Recover from a frozen IPC state
    async fn recover_frozen_ipc(process: &mut CaptiveCoreProcess) -> Result<()> {
        info!("Starting frozen IPC recovery procedure");

        // Step 1: Graceful shutdown
        info!("Step 1: Attempting graceful shutdown");
        match process.terminate_graceful(GRACEFUL_SHUTDOWN_TIMEOUT).await {
            Ok(true) => {
                info!("Graceful shutdown succeeded");
            }
            Ok(false) => {
                warn!("Graceful shutdown failed, proceeding with forced termination");
            }
            Err(e) => {
                error!("Error during graceful shutdown: {}", e);
            }
        }

        // Step 2: Verify process is truly dead
        sleep(Duration::from_millis(500)).await;

        // Step 3: Force terminate if still running
        if process.state() == ProcessState::Running {
            info!("Step 2: Forcing termination with SIGKILL");
            process.terminate_forced().await?;
        }

        // Step 4: Clean up stale lock file
        if process.is_lock_stale().await.unwrap_or(false) {
            info!("Step 3: Removing stale lock file");
            process.remove_stale_lock().await?;
        }

        info!("Frozen IPC recovery completed");
        Ok(())
    }

    /// Get current supervisor state
    pub async fn get_state(&self) -> SupervisorState {
        self.state.read().await.clone()
    }

    /// Stop the supervisor
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Captive Core supervisor");

        let mut state = self.state.write().await;
        state.is_running = false;

        let mut proc = self.process.write().await;
        proc.shutdown().await?;

        Ok(())
    }

    /// Force restart the process
    pub async fn force_restart(&self) -> Result<()> {
        info!("Force restarting Captive Core");

        let mut proc = self.process.write().await;
        let mut state = self.state.write().await;

        proc.restart(self.config.core_args.clone()).await?;
        state.restart_attempts = 0;
        state.consecutive_failures = 0;
        state.last_healthy_check = Some(SystemTime::now());

        Ok(())
    }

    /// Get process state
    pub async fn get_process_state(&self) -> ProcessState {
        self.process.read().await.state()
    }

    /// Get process PID
    pub async fn get_process_pid(&self) -> Option<u32> {
        self.process.read().await.pid()
    }

    /// Check if lock file is stale
    pub async fn is_lock_stale(&self) -> Result<bool> {
        self.process.read().await.is_lock_stale().await
    }

    /// Check lock file info
    pub async fn get_lock_file_info(&self) -> Result<super::process::LockFileInfo> {
        self.process.read().await.check_lock_file().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supervisor_config_default() {
        let config = SupervisorConfig::default();
        assert_eq!(config.health_check_interval, Duration::from_secs(5));
        assert_eq!(config.ipc_timeout, Duration::from_secs(30));
        assert!(config.enable_auto_restart);
        assert_eq!(config.max_restart_attempts, 3);
    }

    #[test]
    fn test_supervisor_state_default() {
        let state = SupervisorState::default();
        assert!(state.is_running);
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.restart_attempts, 0);
        assert!(!state.ipc_responsive);
    }

    #[tokio::test]
    async fn test_supervisor_creation() {
        let config = SupervisorConfig::default();
        let supervisor = CaptiveCoreSupervisor::new(config);

        let state = supervisor.get_state().await;
        assert!(state.is_running);
    }
}
