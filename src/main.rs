#![deny(rust_2018_idioms)]
mod components;
mod electrum_client;

use std::time::Duration;
use anyhow::Result;

use bitcoincash::consensus::Encodable;
use bitcoincash::hashes::hex::ToHex;
use bitcoincash::hashes::{self, sha256};
use bitcoincash::secp256k1::{rand, Message, Secp256k1};
use bitcoincash::{KeyPair, OutPoint, PackedLockTime, Transaction, TxIn, Sequence};
use components::ParsedInput;
use components::tracker::Tracker;
use futures::future::FutureExt;
use futures::StreamExt;
use leptos::{
    component, create_rw_signal, create_signal, event_target_value, log, mount_to_body, on_cleanup,
    view, For, IntoView, RwSignal, SignalGet, SignalUpdate, SignalWith, SignalDispose,
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
    sequence: RwSignal<u32>,
    script_sig: RwSignal<String>,
    key: usize,
}

impl TxInputState {
    fn new(key: usize) -> Self {
        Self {
            txid: create_rw_signal("".into()),
            vout: create_rw_signal(0),
            sequence: create_rw_signal(4294967295),
            script_sig: create_rw_signal("".into()),
            key,
        }
    }

    fn dispose(self) {
        self.txid.dispose();
        self.vout.dispose();
        self.sequence.dispose();
        self.script_sig.dispose();
    }
}

impl TryFrom<TxInputState> for TxIn {
    type Error = hashes::hex::Error;
    fn try_from(tx_input: TxInputState) -> Result<Self, Self::Error> {
        let mut script_sig = tx_input.script_sig.get();
        script_sig.retain(|c| !c.is_ascii_whitespace());
        Ok(TxIn {
            previous_output: OutPoint {
                txid: tx_input.txid.get().parse()?,
                vout: tx_input.vout.get(),
            },
            script_sig: script_sig.parse()?,
            sequence: Sequence(tx_input.sequence.get()),
            witness: Default::default(),
        })
    }
}

#[component]
fn TxInput(tx_input: TxInputState) -> impl IntoView {
    let set_txid = tx_input.txid.write_only();
    let set_script_sig = tx_input.script_sig.write_only();
    view! {
        <div class="mb-1 flex">
            <input
                on:change=move |e| set_txid(event_target_value(&e))
                class="border border-solid rounded border-stone-600 px-1 w-full bg-stone-900 placeholder:text-stone-600 font-mono grow"
                placeholder="Transaction ID"
            />
            <span>:</span>
            <ParsedInput value=tx_input.vout placeholder="Index" class="w-16"/>
        </div>
        <div class="mb-1">
            <textarea
                on:change=move |e| set_script_sig(event_target_value(&e))
                class="border border-solid rounded border-stone-600 px-1 w-full bg-inherit placeholder:text-stone-600 font-mono bg-stone-900"
                placeholder="Unlocking Script Hex"
            />
        </div>
        <div class="my-1">
            <ParsedInput value=tx_input.sequence placeholder="Sequence"/>
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
            let index_to_remove = tx_inputs.iter().enumerate().find(|(_, t)| t.key == key_to_remove).unwrap().0;
            let removed = tx_inputs.remove(index_to_remove);
            removed.dispose();
        });
    };
    let delete_tx_output = move |key_to_remove| {
        set_tx_outputs.update(|tx_outputs| {
            tx_outputs.retain(|t| t.key != key_to_remove);
        });
    };
    let serialize_tx = move || -> Result<String> {
        let input: Result<_, _> = tx_inputs.with(|tx_inputs| {
            tx_inputs
                .iter()
                .map(|&tx_input| tx_input.try_into())
                .collect()
        });
        let input = input?;
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
        Ok(tx_serialized.to_hex())
    };

    view! {
        <div class="table">
            <div class="table-row">
                <div class="table-cell pr-1 pb-1">
                    <label for="tx_version">TX version:</label>
                </div>
                <div class="table-cell pb-1">
                    <ParsedInput id="tx_version" value=tx_version_rw placeholder="2"/>
                </div>
            </div>
            <div class="table-row">
                <div class="table-cell pr-1">
                    <label for="tx_locktime">Locktime:</label>
                </div>
                <div class="table-cell">
                    <ParsedInput id="tx_locktime" value=tx_locktime_rw placeholder="TX locktime"/>
                </div>
            </div>
        </div>
        <div class="flex flex-wrap gap-3 mt-3">
            <div class="basis-[32rem] grow">
                <p class="mb-1">Inputs</p>
                <ol start="0">
                    <For
                        each=move || 0..tx_inputs.with(Vec::len)
                        key=move |i| tx_inputs.with(|v| v[*i].key)
                        view=move |i| {
                            let tx_input = tx_inputs.with(|v| v[i]);
                            view! {
                                <li class="border border-solid rounded-md border-stone-600 p-1 mb-2 bg-stone-800">
                                    <TxInput tx_input=tx_input />
                                    <button
                                        on:click=move |_| delete_tx_input(tx_input.key)
                                        class="border border-solid rounded border-stone-600 px-2 bg-red-950"
                                    >
                                        "-"
                                    </button>
                                </li>
                            }
                        }
                    />
                </ol>
                <button
                    on:click=move |_| new_tx_input()
                    class="border border-solid rounded border-stone-600 px-2"
                >
                    "+"
                </button>
            </div>
            <div class="basis-[32rem] grow">
                <p class="mb-1">Outputs</p>
                <ol start="0">
                    <For
                        each=move || 0..tx_outputs.with(Vec::len)
                        key=move |i| tx_outputs.with(|v| v[*i].key)
                        view=move |i| {
                            let tx_output = tx_outputs.with(|v| v[i]);
                            view! {
                                <li class="border border-solid rounded border-stone-600 p-1 bg-stone-800 mb-2">
                                    <TxOutput tx_output=tx_output />
                                    <button
                                        on:click=move |_| delete_tx_output(tx_output.key)
                                        class="border border-solid rounded border-stone-600 px-2 bg-red-950"
                                    >"-"</button>
                                </li>
                            }
                        }
                    />
                </ol>
                <button
                    on:click=move |_| new_tx_output()
                    class="border border-solid rounded border-stone-600 px-2"
                >
                    "+"
                </button>
            </div>
        </div>
        <div class="mt-3">
            <button
                class="border border-solid rounded border-stone-600 px-1"
                on:click=move |_| {
                    match serialize_tx() {
                        Ok(tx) => set_serialized_tx(tx),
                        Err(e) => set_serialized_tx(e.to_string()),
                    }
                }
            >
                "Serialize"
            </button>
            <button
                class="border border-solid rounded border-stone-600 px-1 mx-1"
                on:click=move |_| {
                    log!("deserialize");
                }
            >
                "Deserialize"
            </button>
            <textarea
                class="border border-solid rounded border-stone-600 px-1 w-full bg-inherit placeholder:text-stone-600 font-mono grow bg-stone-900 my-1"
                prop:value={serialized_tx}
            />
        </div>
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
