use std::sync::Arc;

use alloy_provider::{Provider, ProviderBuilder, RootProvider};
use alloy_pubsub::PubSubFrontend;
use alloy_reth::{
    layer::{db::new_layer_from_db, NoopCanonStateSubscriptions},
    RethProvider,
};
use alloy_rpc_types::Filter;
use alloy_transport_ws::WsConnect;
use reth_blockchain_tree::noop::NoopBlockchainTree;
use reth_db::DatabaseEnv;
use reth_network_api::noop::NoopNetwork;
use reth_provider::providers::BlockchainProvider;
use reth_transaction_pool::noop::NoopTransactionPool;

type RethDBProvider = RethProvider<
    BlockchainProvider<Arc<DatabaseEnv>, NoopBlockchainTree>,
    NoopTransactionPool,
    NoopNetwork,
    NoopCanonStateSubscriptions,
    RootProvider<PubSubFrontend>,
    PubSubFrontend,
>;

#[tokio::main]
async fn main() {
    let ws = WsConnect::new("ws://localhost:8545");

    let db_path = "/root/.local/share/reth/mainnet".into();
    let reth_db_layer = new_layer_from_db(db_path).unwrap();
    let provider = ProviderBuilder::new().layer(reth_db_layer).on_ws(ws).await.unwrap();

    batch_get_logs_from_db(Arc::new(provider)).await;
}

async fn batch_get_logs_from_db(provider: Arc<RethDBProvider>) {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(50));
    let mut tasks = Vec::new();

    let latest_block = provider.get_block_number().await.unwrap();
    println!("Latest block: {}", latest_block);

    for start in (0..latest_block).step_by(20) {
        let end = start + 20;
        let provider = provider.clone();
        let semaphore = semaphore.clone();

        let task = tokio::spawn(async move {
            let _permit = semaphore.clone().acquire_owned().await.unwrap();
            let filter = Filter::new().from_block(start).to_block(end);

            let logs = provider.get_logs(&filter).await.unwrap();
            println!("Got {} logs from block {} to {}", logs.len(), start, end);
        });

        tasks.push(task);
    }

    for task in tasks {
        task.await.unwrap();
    }
}
