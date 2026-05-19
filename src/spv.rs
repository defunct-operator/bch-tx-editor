use std::{
    borrow::Cow,
    collections::hash_map::Entry,
    pin::pin,
    sync::{Arc, Mutex, RwLock},
};

use bitcoincash::Txid;
use futures::{FutureExt, StreamExt, future::Either};
use gloo::storage::{LocalStorage, Storage};
use jsonrpsee::core::ClientError;
use leptos::{
    IntoView, component,
    prelude::{
        ArcReadSignal, ArcRwSignal, ArcWriteSignal, ClassAttribute, ElementChild, FromStream,
        NodeRef, NodeRefAttribute, OnAttribute, PropAttribute, ReadSignal, event_target_value,
        expect_context, provide_context,
    },
    reactive::traits::Set,
    view,
};
use leptos_use::on_click_outside;
use thiserror::Error;
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::WatchStream;
use tracing::{error, info, trace};

use crate::{
    electrum_client::ElectrumClient,
    macros::{DropGuard, StrEnum},
    unbounded_rx_mut_stream::UnboundedReceiverMutStream,
};

#[component]
pub fn SpvModal(mut on_exit: impl FnMut() + Clone + 'static) -> impl IntoView {
    let spv = use_spv();
    let spv_settings_recv = spv.settings.subscribe();
    let spv_settings = ReadSignal::from_stream(WatchStream::new(spv_settings_recv));

    let spv_status = ReadSignal::from(spv.status.clone());
    let spv_height = ReadSignal::from(spv.height.clone());
    let spv_modal_node_ref = NodeRef::new();
    _ = on_click_outside(spv_modal_node_ref, move |_| on_exit());
    view! {
        <div class="fixed top-0 left-0 w-full h-full opacity-50 bg-black"></div>
        <div class="fixed top-0 left-0 w-full h-full flex justify-center-safe items-center">
            <div
                node_ref=spv_modal_node_ref
                class="bg-stone-800 w-2xl p-2 border border-solid rounded border-stone-600"
            >
                <div>
                    "Server: "
                    <input
                        type="text"
                        list="servers"
                        placeholder="wss://bch.imaginary.cash:50004"
                        class="border border-solid rounded px-1 bg-stone-900 placeholder:text-stone-600 border-stone-600 w-md"
                        on:change=move |e| {
                            let value = event_target_value(&e);
                            if value.is_empty() {
                                spv.set_address(None);
                            } else {
                                spv.set_address(Some(value));
                            }
                        }
                        prop:value=move || {
                            spv_settings().and_then(|s| s.address).unwrap_or_default()
                        }
                    /> <datalist id="servers">
                        <option value="wss://bch.imaginary.cash:50004"></option>
                        <option value="wss://blackie.c3-soft.com:50004"></option>
                    </datalist>
                </div>
                <div>Status: {move || spv_status().to_str()}</div>
                <div>
                    Blockchain:
                    {move || {
                        if spv_status() == SpvConnStatus::Disabled {
                            Cow::Borrowed("Not connected")
                        } else {
                            Cow::Owned(format!("{} blocks", spv_height()))
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

str_enum! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum SpvConnStatus {
        Disabled = "Disabled",
        Connecting = "Connecting",
        Connected = "Connected",
        Disconnected = "Disconnected",
    }
}

#[derive(Clone, Debug, Default)]
pub struct SpvSettings {
    enabled: bool,
    address: Option<String>,
}

async fn spv_task(
    status: ArcWriteSignal<SpvConnStatus>,
    height: ArcWriteSignal<i64>,
    mut settings: watch::Receiver<SpvSettings>,
    tx_cache: Arc<TxCache>,
) {
    let default_address = "wss://bch.imaginary.cash:50004";
    let mut conn_fut = pin!(Either::Right(std::future::pending()));
    let mut address = None;
    loop {
        tokio::select! {
            _ = conn_fut.as_mut() => {
                // The connection future exited.
                // TODO conn_fut should return an error and depending on the error, we reconnect.
            },
            r = settings.changed() => {
                if r.is_err() {
                    error!("SpvSettings watch was closed");
                    break;
                }
                let settings = settings.borrow_and_update();
                if settings.address != address {
                    address = settings.address.clone();
                    // make the conn future get recreated with the new address
                    conn_fut.set(Either::Right(std::future::pending()));
                }
                if settings.enabled != matches!(*conn_fut, Either::Left(_)) {
                    if settings.enabled {
                        let addr = address.as_deref().unwrap_or(default_address).into();
                        conn_fut.set(Either::Left(conn_task(
                            status.clone(),
                            height.clone(),
                            addr,
                            tx_cache.clone(),
                        ).fuse()));
                    } else {
                        conn_fut.set(Either::Right(std::future::pending()));
                        status(SpvConnStatus::Disabled);
                    }
                }
            }
        }
    }
}

async fn conn_task(
    status: ArcWriteSignal<SpvConnStatus>,
    height: ArcWriteSignal<i64>,
    addr: String,
    tx_cache: Arc<TxCache>,
) -> Result<(), jsonrpsee::core::ClientError> {
    let _disconnect_guard = DropGuard::new(|| status(SpvConnStatus::Disconnected));
    status(SpvConnStatus::Connecting);
    info!("Connecting to {addr}");
    let client = jsonrpsee::wasm_client::WasmClientBuilder::new()
        .build(addr)
        .await?;
    let client = ElectrumClient::new(client);
    let version = client.server_version("").await?;
    status(SpvConnStatus::Connected);
    info!(
        server_version = version.server_software_version,
        protocol_version = version.protocol_version,
        "Connected",
    );

    let (current_tip, mut subscription) = client.blockchain_headers_subscribe().await?;
    height(current_tip.height);
    info!(?current_tip, "Subscribed to headers");

    // Currently, only one instance of conn_task is expected to be running at a time. We could
    // potentially switch to an MPMC channel later.
    let mut requests = tx_cache.requests.try_lock().unwrap();
    let request_handler =
        UnboundedReceiverMutStream::new(&mut requests).for_each_concurrent(10, |txid| {
            let client = &client;
            let tx_cache = &*tx_cache;
            async move {
                let r = client
                    .blockchain_transaction_get(txid)
                    .await
                    .map(Arc::<[u8]>::from)
                    .map_err(|e| TxCacheError::RpcError(Arc::new(e)));
                tx_cache
                    .map
                    .write()
                    .unwrap()
                    .get(&txid)
                    .unwrap()
                    .set(Some(r));
            }
        });

    tokio::select! {
        _ = request_handler => (),
        _ = client.ping_loop() => (),
        _ = async move {
            loop {
                let new_tip = subscription.next().await;
                let Some(Ok(block)) = new_tip else { break };
                info!(?block, "Got new block header");
                height(block.height);
            }
        } => (),
    }
    Ok(())
}

#[derive(Clone)]
pub struct Spv {
    pub settings: watch::Sender<SpvSettings>,
    pub status: ArcReadSignal<SpvConnStatus>,
    pub height: ArcReadSignal<i64>,
    pub tx_fetcher: Arc<TxCache>,
}

impl Spv {
    pub fn set_enabled(&self, enabled: bool) {
        self.settings.send_if_modified(|v| {
            if v.enabled != enabled {
                v.enabled = enabled;
                true
            } else {
                false
            }
        });
    }

    pub fn set_address(&self, addr: Option<String>) {
        self.settings.send_if_modified(|v| {
            if v.address != addr {
                v.address = addr;
                true
            } else {
                false
            }
        });
    }
}

/// Should only be called once.
pub fn provide_spv() {
    const SPV_SERVER_ADDRESS_LOCALSTORAGE_KEY: &str = "spv_server_address";
    let address = LocalStorage::get::<Option<String>>(SPV_SERVER_ADDRESS_LOCALSTORAGE_KEY)
        .ok()
        .flatten();
    let mut spv_settings = watch::channel(SpvSettings {
        address,
        enabled: false,
    });
    let status = ArcRwSignal::new(SpvConnStatus::Disabled);
    let height = ArcRwSignal::new(0);
    let tx_cache = Arc::default();
    let spv = Spv {
        settings: spv_settings.0.clone(),
        status: status.read_only(),
        height: height.read_only(),
        tx_fetcher: tx_cache,
    };
    leptos::task::spawn_local(spv_task(
        status.write_only(),
        height.write_only(),
        spv_settings.1.clone(),
        spv.tx_fetcher.clone(),
    ));
    leptos::task::spawn_local(async move {
        while spv_settings.1.changed().await.is_ok() {
            let addr = spv_settings.1.borrow().address.clone();
            _ = LocalStorage::set(SPV_SERVER_ADDRESS_LOCALSTORAGE_KEY, addr);
        }
    });
    provide_context(spv);
}

pub fn use_spv() -> Spv {
    expect_context()
}

type Map<K, V> = std::collections::HashMap<K, V>;

/// A cache for transaction requests. Ensures that only one request is made per TXID.
pub struct TxCache {
    map: RwLock<Map<Txid, ArcRwSignal<Option<Result<Arc<[u8]>, TxCacheError>>>>>,
    requests: Mutex<mpsc::UnboundedReceiver<Txid>>,
    request_sender: mpsc::UnboundedSender<Txid>,
}

impl TxCache {
    /// Lookup a transaction by TXID. If not present in the cache, queues a request to fetch it.
    pub fn get(&self, txid: Txid) -> ArcRwSignal<Option<Result<Arc<[u8]>, TxCacheError>>> {
        let mut is_new = false;
        let entry = match self.map.write().unwrap().entry(txid) {
            Entry::Occupied(e) => e,
            Entry::Vacant(vacant_entry) => {
                is_new = true;
                vacant_entry.insert_entry(Default::default())
            }
        }
        .get()
        .clone();
        if is_new {
            trace!(?txid, "New request");
            self.request_sender.send(txid).unwrap();
        } else {
            trace!(?txid, "Existing request");
        }
        entry
    }

    /// Lookup a transaction by TXID. If not present in the cache, returns `None`.
    pub fn try_get(
        &self,
        txid: &Txid,
    ) -> Option<ArcRwSignal<Option<Result<Arc<[u8]>, TxCacheError>>>> {
        self.map.read().unwrap().get(txid).cloned()
    }
}

impl Default for TxCache {
    fn default() -> Self {
        let (request_sender, requests) = mpsc::unbounded_channel();
        Self {
            map: Default::default(),
            requests: Mutex::new(requests),
            request_sender,
        }
    }
}

#[derive(Debug, Clone, Error)]
pub enum TxCacheError {
    #[error("spv disconnected")]
    Disconnected,
    #[error("rpc error: {0}")]
    RpcError(Arc<ClientError>),
    #[error("transaction not found")]
    NotFound,
}
