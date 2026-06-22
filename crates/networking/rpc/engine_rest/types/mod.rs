//! SSZ wire types for the engine REST API.
//!
//! `common` holds fork-invariant types (PayloadStatus, ForkchoiceState,
//! PayloadId, Withdrawal). Per-fork modules (`osaka`, `amsterdam`) hold the
//! ExecutionPayload, ExecutionPayloadEnvelope, and PayloadAttributes shapes.

pub mod amsterdam;
pub mod blobs;
pub mod bodies;
pub mod built_payload;
pub mod common;
pub mod conversions;
pub mod forkchoice_update;
pub mod osaka;
