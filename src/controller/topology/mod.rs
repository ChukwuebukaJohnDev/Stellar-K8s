//! Dynamic topology enforcement for Stellar node StatefulSets.
//!
//! Provides a mutating controller that inspects availability-zone labels on
//! cluster nodes and injects the correct `topologySpreadConstraints` and
//! `podAntiAffinity` rules into Stellar node StatefulSet pod templates.
//!
//! # Sub-modules
//!
//! - [`enforcer`] — controller that patches StatefulSets
//! - [`rules`]    — rule generation and topology types

pub mod enforcer;
pub mod rules;

pub use enforcer::{
    build_statefulset_patch, discover_cluster_topology, enforce_namespace, enforce_on_statefulset,
    EnforcementResult,
};
pub use rules::{
    build_rule_set, hard_host_anti_affinity, soft_host_anti_affinity, zone_node_affinity_terms,
    ClusterTopology, TopologyMode, TopologyRuleSet, TopologySpreadConstraint, WhenUnsatisfiable,
    TOPOLOGY_HOST_KEY, TOPOLOGY_ZONE_KEY,
};
