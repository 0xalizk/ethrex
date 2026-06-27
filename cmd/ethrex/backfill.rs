use std::{path::Path, time::Duration};

use eyre::eyre;
use ethrex_p2p::{
    DiscoveryConfig,
    discv4::server::INITIAL_LOOKUP_INTERVAL_MS,
    network::{P2PContext, start_network},
    peer_handler::PeerHandler,
    peer_table::PeerTableServer,
    rlpx::initiator::RLPxInitiator,
    sync::backfill_block_range,
};
use tokio_util::task::TaskTracker;
use tracing::info;

use ethrex_config::networks::Network;

use crate::{
    cli::Options,
    initializers::{
        get_bootnodes, get_local_p2p_node, get_signer, init_blockchain, open_store_unchecked,
    },
    utils::get_client_version_string,
};
use ethrex_blockchain::BlockchainOptions;

/// Download and store block headers + bodies from mainnet p2p peers for blocks `from..=to`.
///
/// The mainnet EL node must be stopped before calling this — both this tool and the node
/// open the same RocksDB datadir exclusively.  After this returns, restart the node; it will
/// now serve `eth_getBlockByNumber` for the previously-missing range.
///
/// Typical usage (fill the snap-sync body gap so catch-up can run locally):
/// ```
/// ulimit -n 1048576
/// ethrex --network mainnet --datadir ~/.local/share/ethrex/mainnet \
///   backfill-bodies --from 25340001 --to 25401794
/// ```
pub async fn backfill_bodies(
    datadir: &Path,
    from: u64,
    to: u64,
    opts: &Options,
    network: &Network,
) -> eyre::Result<()> {
    if from > to {
        return Err(eyre!("--from {from} is greater than --to {to}"));
    }

    info!("backfill-bodies: opening store at {datadir:?}");
    let mut store = open_store_unchecked(datadir)
        .map_err(|e| eyre!("failed to open store: {e}"))?;
    let genesis = network.get_genesis().map_err(|e| eyre!("failed to load genesis: {e}"))?;
    // add_initial_state: sets chain_config, warms latest_block_header cache, returns early
    // without writes if genesis already exists in the DB (snap-synced mainnet EL).
    store.add_initial_state(genesis).await.map_err(|e| eyre!("failed to initialize store from genesis: {e}"))?;
    let blockchain = init_blockchain(
        store.clone(),
        BlockchainOptions {
            r#type: ethrex_blockchain::BlockchainType::L1,
            ..Default::default()
        },
    );

    // Identify blocks that still need bodies so we can skip an already-done range.
    info!("backfill-bodies: scanning {from}..={to} for missing bodies ({} blocks)", to - from + 1);
    let first_missing = {
        let mut first = None;
        for n in from..=to {
            if store.get_block_body(n).await?.is_none() {
                first = Some(n);
                break;
            }
        }
        first
    };
    let Some(start) = first_missing else {
        info!("backfill-bodies: all bodies already present in {from}..={to}, nothing to do");
        return Ok(());
    };

    info!("backfill-bodies: first missing body at block {start}; connecting to mainnet p2p");

    // Start the p2p stack (discovery + RLPx).  Uses the same port config as the node
    // (default 30303) — the node must be stopped so the port is free.
    let signer = get_signer(datadir);
    let local_node = get_local_p2p_node(opts, &signer);
    let peer_table = PeerTableServer::spawn(opts.target_peers, store.clone());
    let tracker = TaskTracker::new();

    let p2p_context = P2PContext::new(
        local_node,
        tracker.clone(),
        signer,
        peer_table.clone(),
        store.clone(),
        blockchain,
        get_client_version_string(),
        None,
        opts.tx_broadcasting_time_interval,
        INITIAL_LOOKUP_INTERVAL_MS as f64,
    )
    .map_err(|e| eyre!("P2P context: {e}"))?;

    let initiator = RLPxInitiator::spawn(p2p_context.clone());
    let peer_handler = PeerHandler::new(peer_table.clone(), initiator);

    let bootnodes = get_bootnodes(opts, network, datadir);
    let discovery_config = DiscoveryConfig {
        discv4_enabled: opts.discv4_enabled,
        discv5_enabled: opts.discv5_enabled,
    };
    start_network(p2p_context, bootnodes, discovery_config)
        .await
        .map_err(|e| eyre!("start_network: {e}"))?;

    // Give discovery time to find peers before the first request.
    info!("backfill-bodies: waiting 20s for peers to connect...");
    tokio::time::sleep(Duration::from_secs(20)).await;

    info!("backfill-bodies: downloading headers + bodies for {start}..={to}");
    backfill_block_range(start, to, peer_handler, store)
        .await
        .map_err(|e| eyre!("backfill error: {e}"))?;

    info!("backfill-bodies: complete — blocks {start}..={to} now have bodies");
    Ok(())
}
