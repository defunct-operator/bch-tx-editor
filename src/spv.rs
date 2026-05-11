use std::{borrow::Cow, pin::pin};

use futures::{FutureExt, StreamExt, future::Either};
use leptos::{
    IntoView, component,
    prelude::{
        ArcReadSignal, ArcRwSignal, ArcWriteSignal, ClassAttribute, Effect, ElementChild,
        FromStream, NodeRef, NodeRefAttribute, OnAttribute, PropAttribute, ReadSignal,
        event_target_value, expect_context, provide_context,
    },
    view,
};
use leptos_use::on_click_outside;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use tracing::{error, info};

use crate::{
    electrum_client::ElectrumClient,
    macros::{DropGuard, StrEnum},
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
        <div class="fixed top-0 left-0 w-full h-full opacity-50 bg-black">
        </div>
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
                        prop:value=move || spv_settings().and_then(|s| s.address).unwrap_or_default()
                    />
                    <datalist id="servers">
                        <option value="wss://bch.imaginary.cash:50004"></option>
                        <option value="wss://blackie.c3-soft.com:50004"></option>
                    </datalist>
                </div>
                <div>Status: {move || spv_status().to_str()}</div>
                <div>Blockchain: {move ||
                    if spv_status() == SpvStatus::Disabled {
                        Cow::Borrowed("Not connected")
                    } else {
                        Cow::Owned(format!("{} blocks", spv_height()))
                    }
                }</div>
            </div>
        </div>
    }
}

str_enum! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum SpvStatus {
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
    status: ArcWriteSignal<SpvStatus>,
    height: ArcWriteSignal<i64>,
    mut settings: watch::Receiver<SpvSettings>,
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
                        status(SpvStatus::Connecting);
                        conn_fut.set(Either::Left(conn_task(status.clone(), height.clone(), addr).fuse()));
                    } else {
                        status(SpvStatus::Disabled);
                        conn_fut.set(Either::Right(std::future::pending()));
                    }
                }
            }
        }
    }
}

async fn conn_task(
    status: ArcWriteSignal<SpvStatus>,
    height: ArcWriteSignal<i64>,
    addr: String,
) -> Result<(), jsonrpsee::core::ClientError> {
    let _disconnect_guard = DropGuard::new(|| status(SpvStatus::Disconnected));
    status(SpvStatus::Connecting);
    info!("Connecting to {addr}");
    let client = jsonrpsee::wasm_client::WasmClientBuilder::new()
        .build(addr)
        .await
        .unwrap();
    let client = ElectrumClient::new(client);
    let version = client.server_version("").await?;
    status(SpvStatus::Connected);
    info!(
        "Connected, server version: {}, protocol version: {}",
        version.server_software_version, version.protocol_version
    );

    let (current_tip, mut subscription) = client.blockchain_headers_subscribe().await?;
    height(current_tip.height);
    info!(?current_tip, "Subscribed to headers");

    futures::select! {
        _ = client.ping_loop().fuse() => (),
        _ = async move {
            loop {
                let new_tip = subscription.next().await;
                info!(?new_tip, "Got new block header");
                let Some(Ok(block)) = new_tip else { break };
                height(block.height);
            }
        }.fuse() => (),
    }
    Ok(())
}

#[derive(Clone)]
pub struct Spv {
    pub settings: watch::Sender<SpvSettings>,
    pub status: ArcReadSignal<SpvStatus>,
    pub height: ArcReadSignal<i64>,
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
    let spv_settings = watch::channel(SpvSettings::default());
    let status = ArcRwSignal::new(SpvStatus::Disabled);
    let enabled = ArcRwSignal::new(false);
    let height = ArcRwSignal::new(0);
    let address = ArcRwSignal::new(None);
    let spv = Spv {
        settings: spv_settings.0.clone(),
        status: status.read_only(),
        height: height.read_only(),
    };
    Effect::new(move || {
        spv_settings.0.send_replace(SpvSettings {
            enabled: enabled(),
            address: address(),
        });
    });
    leptos::task::spawn_local(spv_task(
        status.write_only(),
        height.write_only(),
        spv_settings.1,
    ));
    provide_context(spv);
}

pub fn use_spv() -> Spv {
    expect_context()
}
