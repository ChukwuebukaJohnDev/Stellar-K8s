//! Maintenance Window Controller logic
//!
//! Manages the lifecycle of maintenance windows and triggers DB compaction.
//! This is a thin facade over [`super::compactor`]: the heavy lifting (drain,
//! compact, verify, rejoin) lives in [`super::compactor::run_compaction_cycle`].

use std::sync::Arc;

use kube::{Client, ResourceExt};
use sqlx::PgPool;
use tracing::debug;

use super::compactor::{self, CompactionCoordinator};
use crate::crd::StellarNode;
use crate::error::Result;

pub struct MaintenanceController {
    client: Client,
    coordinator: Arc<CompactionCoordinator>,
}

impl MaintenanceController {
    pub fn new(client: Client, coordinator: Arc<CompactionCoordinator>) -> Self {
        Self {
            client,
            coordinator,
        }
    }

    /// Check if we are currently in a maintenance window (cron schedule or
    /// `windowStart`/`windowDuration` fallback).
    pub fn is_in_window(&self, node: &StellarNode) -> bool {
        let config = match &node.spec.db_maintenance_config {
            Some(c) if c.enabled => c,
            _ => return false,
        };

        if let Some(schedule) = &config.schedule {
            return compactor::cron_schedule_due(schedule, None, chrono::Utc::now());
        }

        let (start, duration) = compactor::window_config(config);
        compactor::is_in_window(start, duration)
    }

    /// Run maintenance tasks for a node if needed.
    ///
    /// Executes the full compaction cycle: quiet check → fragmentation
    /// evaluation → traffic drain → VACUUM FULL / REINDEX → ledger pruning →
    /// post-compaction checksum verification → traffic rejoin.
    pub async fn run_maintenance(&self, node: &StellarNode, pool: PgPool) -> Result<()> {
        let report =
            compactor::run_compaction_cycle(&self.client, None, &self.coordinator, node, &pool)
                .await?;

        if let Some(skipped) = &report.skipped_reason {
            debug!(
                "Maintenance skipped for node {}: {skipped}",
                node.name_any()
            );
        }
        Ok(())
    }
}
