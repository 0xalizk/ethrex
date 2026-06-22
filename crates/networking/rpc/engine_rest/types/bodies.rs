//! SSZ wire types for the engine REST bodies endpoints (execution-apis #793).
//!
//! Request (`POST /{fork}/bodies/hash`) is a bare `List[Hash32, MAX_BODIES_REQUEST]`.
//! Response is a bare `List[BodyEntry, MAX_BODIES_REQUEST]` — NOT wrapped in a
//! named container — where `BodyEntry { available: Boolean, body: ExecutionPayloadBody }`.
//! When `available == false` the `body` is zero-valued (every list empty) and CLs
//! MUST ignore it. Each fork URL returns only its own era's blocks.

use libssz_derive::{HashTreeRoot, SszDecode, SszEncode};
use libssz_types::SszList;

use super::common::{
    MAX_BLOCK_ACCESS_LIST_BYTES, MAX_BYTES_PER_TRANSACTION, MAX_TRANSACTIONS_PER_PAYLOAD,
    MAX_WITHDRAWALS_PER_PAYLOAD, Withdrawal,
};

/// Spec cap on hashes per `/{fork}/bodies/hash` request and on entries in any
/// bodies response (`MAX_BODIES_REQUEST = 2**5`); matches the consensoor CL.
pub const MAX_BODIES_PER_REQUEST: usize = 32;

/// Inner block-hash list wrapped by `BodiesByHashRequest`.
pub type BlockHashList = SszList<[u8; 32], MAX_BODIES_PER_REQUEST>;

/// `POST /{fork}/bodies/hash` request. Per execution-apis #793 the request is a
/// single-field SSZ **container** wrapping the list, NOT a bare top-level list.
#[derive(Debug, Clone, PartialEq, Eq, SszEncode, SszDecode, HashTreeRoot)]
pub struct BodiesByHashRequest {
    pub block_hashes: BlockHashList,
}

// ── Per-fork ExecutionPayloadBody ─────────────────────────────────────────────

/// Osaka body: transactions + withdrawals.
#[derive(Debug, Clone, PartialEq, Eq, SszEncode, SszDecode, HashTreeRoot)]
pub struct BodyOsaka {
    pub transactions: SszList<SszList<u8, MAX_BYTES_PER_TRANSACTION>, MAX_TRANSACTIONS_PER_PAYLOAD>,
    pub withdrawals: SszList<Withdrawal, MAX_WITHDRAWALS_PER_PAYLOAD>,
}

/// Amsterdam body: BodyOsaka + raw block_access_list bytes.
#[derive(Debug, Clone, PartialEq, Eq, SszEncode, SszDecode, HashTreeRoot)]
pub struct BodyAmsterdam {
    pub transactions: SszList<SszList<u8, MAX_BYTES_PER_TRANSACTION>, MAX_TRANSACTIONS_PER_PAYLOAD>,
    pub withdrawals: SszList<Withdrawal, MAX_WITHDRAWALS_PER_PAYLOAD>,
    pub block_access_list: SszList<u8, MAX_BLOCK_ACCESS_LIST_BYTES>,
}

impl BodyOsaka {
    /// Zero-valued body for an `available == false` entry (CLs MUST ignore it).
    pub fn empty() -> Self {
        BodyOsaka {
            transactions: Vec::new().try_into().expect("empty list fits"),
            withdrawals: Vec::new().try_into().expect("empty list fits"),
        }
    }
}

impl BodyAmsterdam {
    /// Zero-valued body for an `available == false` entry (CLs MUST ignore it).
    pub fn empty() -> Self {
        BodyAmsterdam {
            transactions: Vec::new().try_into().expect("empty list fits"),
            withdrawals: Vec::new().try_into().expect("empty list fits"),
            block_access_list: Vec::new().try_into().expect("empty list fits"),
        }
    }
}

// ── Per-fork BodyEntry { available, body } ────────────────────────────────────

/// Osaka bodies response entry.
#[derive(Debug, Clone, PartialEq, Eq, SszEncode, SszDecode, HashTreeRoot)]
pub struct BodyEntryOsaka {
    pub available: bool,
    pub body: BodyOsaka,
}

/// Amsterdam bodies response entry.
#[derive(Debug, Clone, PartialEq, Eq, SszEncode, SszDecode, HashTreeRoot)]
pub struct BodyEntryAmsterdam {
    pub available: bool,
    pub body: BodyAmsterdam,
}

impl BodyEntryOsaka {
    pub fn available(body: BodyOsaka) -> Self {
        Self {
            available: true,
            body,
        }
    }
    pub fn unavailable() -> Self {
        Self {
            available: false,
            body: BodyOsaka::empty(),
        }
    }
}

impl BodyEntryAmsterdam {
    pub fn available(body: BodyAmsterdam) -> Self {
        Self {
            available: true,
            body,
        }
    }
    pub fn unavailable() -> Self {
        Self {
            available: false,
            body: BodyAmsterdam::empty(),
        }
    }
}

// ── Response containers (single-field, per execution-apis #793) ───────────────
//
// Shared by both `POST /{fork}/bodies/hash` and `GET /{fork}/bodies` (range).

/// Osaka bodies response: `{ entries: List[BodyEntryOsaka, N] }`.
#[derive(Debug, Clone, PartialEq, Eq, SszEncode, SszDecode, HashTreeRoot)]
pub struct BodiesResponseOsaka {
    pub entries: SszList<BodyEntryOsaka, MAX_BODIES_PER_REQUEST>,
}
/// Amsterdam bodies response.
#[derive(Debug, Clone, PartialEq, Eq, SszEncode, SszDecode, HashTreeRoot)]
pub struct BodiesResponseAmsterdam {
    pub entries: SszList<BodyEntryAmsterdam, MAX_BODIES_PER_REQUEST>,
}
