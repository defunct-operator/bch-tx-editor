#![deny(rust_2018_idioms)]
mod components;
mod electrum_client;

use std::time::Duration;

use bitcoincash::consensus::Encodable;
use bitcoincash::hashes::hex::ToHex;
use bitcoincash::hashes::{self, sha256};
use bitcoincash::secp256k1::{rand, Message, Secp256k1};
use bitcoincash::{KeyPair, OutPoint, PackedLockTime, Transaction, TxIn};
use components::ParsedInput;
use futures::future::FutureExt;
use futures::StreamExt;
use leptos::{
    component, create_rw_signal, create_signal, event_target_value, log, mount_to_body, on_cleanup,
    view, For, IntoView, RwSignal, SignalGet, SignalUpdate, SignalWith,
};

use crate::components::tx_output::{TxOutput, TxOutputState};
use crate::electrum_client::ElectrumClient;

fn main() {
    mount_to_body(|| view! { <App/> });
}

#[derive(Copy, Clone)]
struct TxInputState {
    txid: RwSignal<String>,
    vout: RwSignal<u32>,
    script_sig: RwSignal<String>,
    key: usize,
}

impl TxInputState {
    fn new(key: usize) -> Self {
        Self {
            txid: create_rw_signal("".into()),
            vout: create_rw_signal(0),
            script_sig: create_rw_signal("".into()),
            key,
        }
    }
}

impl TryFrom<TxInputState> for TxIn {
    type Error = hashes::hex::Error;
    fn try_from(tx_input: TxInputState) -> Result<Self, Self::Error> {
        Ok(TxIn {
            previous_output: OutPoint {
                txid: tx_input.txid.get().parse()?,
                vout: tx_input.vout.get(),
            },
            script_sig: tx_input.script_sig.get().parse()?,
            ..Default::default()
        })
    }
}

#[component]
fn TxInput(tx_input: TxInputState) -> impl IntoView {
    let set_txid = tx_input.txid.write_only();
    let set_script_sig = tx_input.script_sig.write_only();
    view! {
        <div class="mb-1">
            <input
                on:change=move |e| set_txid(event_target_value(&e))
                class="border border-solid rounded border-stone-600 px-1 w-full bg-inherit placeholder:text-stone-600 font-mono"
                placeholder="Transaction ID"
            />
        </div>
        <div class="mb-1">
            <ParsedInput value=tx_input.vout placeholder="Index" class="w-16"/>
        </div>
        <div class="my-1">
            <textarea
                on:change=move |e| set_script_sig(event_target_value(&e))
                class="border border-solid rounded border-stone-600 px-1 w-full bg-inherit placeholder:text-stone-600 font-mono"
                placeholder="Unlocking Script Hex"
            />
        </div>
    }
}

#[component]
fn App() -> impl IntoView {
    let (tx_inputs, set_tx_inputs) = create_signal(vec![TxInputState::new(0)]);
    let (tx_outputs, set_tx_outputs) = create_signal(vec![TxOutputState::new(0)]);
    let tx_version_rw = create_rw_signal(2i32);
    let tx_locktime_rw = create_rw_signal(0u32);
    let (serialized_tx, set_serialized_tx) = create_signal(String::new());

    let mut new_tx_input = {
        let mut id = 0;
        move || {
            id += 1;
            set_tx_inputs.update(|tx_inputs| tx_inputs.push(TxInputState::new(id)));
        }
    };
    let mut new_tx_output = {
        let mut id = 0;
        move || {
            id += 1;
            set_tx_outputs.update(|tx_outputs| tx_outputs.push(TxOutputState::new(id)));
        }
    };
    let delete_tx_input = move |key_to_remove| {
        set_tx_inputs.update(|tx_inputs| {
            tx_inputs.retain(|t| t.key != key_to_remove);
        });
    };
    let delete_tx_output = move |key_to_remove| {
        set_tx_outputs.update(|tx_outputs| {
            tx_outputs.retain(|t| t.key != key_to_remove);
        });
    };
    let serialize_tx = move || {
        let input: Result<_, _> = tx_inputs.with(|tx_inputs| {
            tx_inputs
                .iter()
                .map(|&tx_input| tx_input.try_into())
                .collect()
        });
        let input = input.map_err(|e| format!("{e}"))?;
        let output: Result<_, _> = tx_outputs.with(|tx_outputs| {
            tx_outputs
                .iter()
                .map(|&tx_output| tx_output.try_into())
                .collect()
        });
        let output = output?;
        let tx = Transaction {
            version: tx_version_rw.get(),
            lock_time: PackedLockTime(tx_locktime_rw.get()),
            input,
            output,
        };
        let mut tx_serialized = vec![];
        tx.consensus_encode(&mut tx_serialized).unwrap();
        Ok::<_, String>(tx_serialized.to_hex())
    };

    view! {
        <div class="m-1">
            <p>TX version number</p>
            <ParsedInput value=tx_version_rw placeholder="TX version"/>
        </div>
        <div class="m-1">
            <p>Locktime</p>
            <ParsedInput value=tx_locktime_rw placeholder="TX locktime"/>
        </div>
        <div class="flex flex-wrap border">
            <div class="border basis-[32rem] grow">
                <p class="mx-1 mt-3">Inputs</p>
                <ol start="0">
                    <For
                        each=move || 0..tx_inputs.with(Vec::len)
                        key=move |i| tx_inputs.with(|v| v[*i].key)
                        view=move |i| {
                            let tx_input = tx_inputs.with(|v| v[i]);
                            view! {
                                <li class="border border-solid rounded border-stone-600 m-1 p-1">
                                    <TxInput tx_input=tx_input />
                                    <button on:click=move |_| delete_tx_input(tx_input.key) class="border border-solid rounded border-stone-600">"Delete"</button>
                                </li>
                            }
                        }
                    />
                </ol>
                <button on:click=move |_| new_tx_input() class="border border-solid rounded border-stone-600 m-1 px-2">"+"</button>
            </div>
            <div class="border basis-[32rem] grow">
                <p class="mx-1 mt-3">Outputs</p>
                <ol start="0">
                    <For
                        each=move || 0..tx_outputs.with(Vec::len)
                        key=move |i| tx_outputs.with(|v| v[*i].key)
                        view=move |i| {
                            let tx_output = tx_outputs.with(|v| v[i]);
                            view! {
                                <li class="border border-solid rounded border-stone-600 m-1 p-1">
                                    <TxOutput tx_output=tx_output />
                                    <button on:click=move |_| delete_tx_output(tx_output.key) class="border border-solid rounded border-stone-600">"Delete"</button>
                                </li>
                            }
                        }
                    />
                </ol>
                <button on:click=move |_| new_tx_output() class="border border-solid rounded border-stone-600 m-1 px-2">"+"</button>
            </div>
        </div>
        <div class="m-1 mt-3">
            <button
                class="border border-solid rounded border-stone-600"
                on:click=move |_| {
                    match serialize_tx() {
                        Ok(tx) => set_serialized_tx(tx),
                        Err(e) => set_serialized_tx(e),
                    }
                }
            >
                "Serialize to hex"
            </button>
        </div>
        <p>{serialized_tx}</p>
    }
}

#[component]
fn ElectrumThingo() -> impl IntoView {
    let (cancel_send, mut cancel_recv) = futures::channel::oneshot::channel::<()>();
    on_cleanup(|| {
        cancel_send.send(()).ok();
    });

    leptos::spawn_local(async move {
        let client = jsonrpsee::wasm_client::WasmClientBuilder::new()
            .build("wss://chipnet.imaginary.cash:50004")
            .await
            .unwrap();
        log!("Connected");
        let client = ElectrumClient::new(client);

        // Protocol version negotiation
        let version = client.server_version("").await.unwrap();
        log!(
            "Server version: {}, protocol version: {}",
            version.server_software_version,
            version.protocol_version
        );

        let (current_head, mut subscription) = client.blockchain_headers_subscribe().await.unwrap();
        log!("\n{current_head:?}");

        futures::select! {
            _ = cancel_recv => (),
            _ = client.ping_loop().fuse() => (),
            _ = async move {
                loop {
                    let result = subscription.next().await;
                    log!("\n{result:?}");
                    if result.is_none() {
                        break;
                    }
                }
            }.fuse() => (),
        }
        log!("Disconnect");
    });
}

#[component]
fn AsyncCounter() -> impl IntoView {
    let (count, set_count) = create_signal(0);
    let async_task = || async move {
        loop {
            gloo::timers::future::sleep(Duration::from_secs(1)).await;
            set_count.update(|x| *x += 1);
        }
    };
    leptos::spawn_local(async_task());
    view! { <p>{count}</p> }
}

#[component]
fn SimpleWallet() -> impl IntoView {
    let secp = Secp256k1::new();
    let mut rng = rand::thread_rng();
    let keypair = KeyPair::new(&secp, &mut rng);

    let (value, set_value) = create_signal(String::new());
    let pubkey = keypair.public_key().to_string();

    view! {
        <p>"Public key: " {pubkey}</p>
        <p>
            "Message to sign: "
            <input
                on:change=move |e| set_value(event_target_value(&e))
            />
        </p>
        <p>
            "Signature: "
            {move || {
                let sig = secp.sign_ecdsa(&Message::from_hashed_data::<sha256::Hash>(value().as_bytes()), &keypair.secret_key());
                sig.to_string()
            }}
        </p>
    }
}

fn convert(input: &str) -> Option<String> {
    cashaddr::convert::from_legacy(input, "bitcoincash")
        .or_else(|_| cashaddr::convert::to_legacy(input))
        .ok()
}
#[component]
fn AddressConverter() -> impl IntoView {
    let (value, set_value) = create_signal(String::new());

    view! {
        "Convert address: "
        <input
            on:change=move |e| set_value(event_target_value(&e))
        />
        <p>{move || match value.with(|v| convert(v)) {
            Some(s) => s,
            None => "Invalid address".into(),
        }}</p>
    }
}

/// Consists of a button and a progress bar
#[component]
fn Pump(#[prop(default = 100)] max: u16) -> impl IntoView {
    let (value, set_value) = create_signal(0);
    view! {
        <button
            on:click = move |_| set_value.update(|c| *c += 1)
            style="margin-right: 10px"
        >
            "Pump"
        </button>
        <progress max=max value=value/>
    }
}
