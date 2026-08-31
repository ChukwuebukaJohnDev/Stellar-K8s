//! Maintenance Window controller for Horizon DB maintenance tasks.
//!
//! Handles scheduling and coordination of VACUUM FULL, reindexing, ledger
//! pruning, and post-compaction integrity verification.
//!
//! # Modules
//!
//! - [`db`] — fragmentation metrics, checksum integrity verification, and
//!   ledger pruning against the Postgres store.
//! - [`compactor`] — cron-triggered compaction daemon with quorum-safe
//!   coordination (`CompactionDaemon`, `CompactionCoordinator`).
//! - [`coordinator`] — zero-downtime traffic drain/rejoin via Service selector
//!   patching (`MaintenanceCoordinator`).
//! - [`bloat`] — bloat detection helpers (`BloatDetector`).
//! - [`controller`] — the original `MaintenanceController` facade.

pub mod bloat;
pub mod compactor;
pub mod controller;
pub mod coordinator;
pub mod db;
pub mod node_drain;

pub use bloat::BloatDetector;
pub use compactor::{
    run_compaction_cycle, CompactionCoordinator, CompactionDaemon, CompactionGuard,
    CompactionReport, COMPACTION_MARKER_ANNOTATION,
};
pub use controller::MaintenanceController;
pub use coordinator::MaintenanceCoordinator;
pub use db::{
    evaluate_fragmentation, total_relation_size, verify_integrity, DatabaseIntegrityVerifier,
    FragmentationMetrics, IntegrityReport, LedgerPruner, PruningReport,
};
pub use node_drain::NodeDrainOrchestrator;
