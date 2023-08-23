#![deny(rust_2018_idioms)]
mod electrum_client;

use std::str::FromStr;
use std::time::Duration;

use bitcoincash::consensus::Encodable;
use bitcoincash::hashes::hex::ToHex;
use bitcoincash::hashes::sha256;
use bitcoincash::secp256k1::{rand, Message, Secp256k1};
use bitcoincash::{KeyPair, TxIn, OutPoint, Script, TxOut, Transaction, PackedLockTime};
use bitcoincash::hashes;
use futures::future::FutureExt;
use futures::StreamExt;
use leptos::{
    component, create_node_ref, create_rw_signal, create_signal, event_target_value, log,
    mount_to_body, on_cleanup, view, CollectView, For, IntoAttribute, IntoView, MaybeSignal,
    RwSignal, Scope, Signal, SignalUpdate, SignalWith, WriteSignal, ReadSignal, IntoProperty, SignalGet,
};

use crate::electrum_client::ElectrumClient;

fn main() {
    mount_to_body(|cx| view! { cx, <App/> });
}

#[derive(Copy, Clone)]
struct TxInputState {
    txid: RwSignal<String>,
    vout: RwSignal<u32>,
    script_sig: RwSignal<String>,
    key: usize,
}

impl TxInputState {
    fn new(cx: Scope, key: usize) -> Self {
        Self {
            txid: create_rw_signal(cx, "".into()),
            vout: create_rw_signal(cx, 0),
            script_sig: create_rw_signal(cx, "".into()),
            key,
        }
    }
}

#[derive(Copy, Clone)]
struct TxOutputState {
    value: RwSignal<u64>,
    script_pubkey: RwSignal<String>,
    key: usize,
}

impl TxOutputState {
    fn new(cx: Scope, key: usize) -> Self {
        Self {
            value: create_rw_signal(cx, 0),
            script_pubkey: create_rw_signal(cx, "".into()),
            key,
        }
    }
}

#[component]
fn TxInput(cx: Scope, tx_input: TxInputState) -> impl IntoView {
    let set_txid = tx_input.txid.write_only();
    let set_script_sig = tx_input.script_sig.write_only();
    view! { cx,
        <div class="mb-1">
            <input on:change=move |e| set_txid(event_target_value(&e)) class="border border-solid rounded border-black px-1" placeholder="Transaction ID"/>
        </div>
        <div class="mb-1">
            <ParsedInput input_type="number" value=tx_input.vout placeholder="Index"/>
        </div>
        <div class="my-1">
            <textarea on:change=move |e| set_script_sig(event_target_value(&e)) class="border border-solid rounded border-black px-1" placeholder="Unlocking Script Hex"/>
        </div>
    }
}

#[component]
fn TxOutput(cx: Scope, tx_output: TxOutputState) -> impl IntoView {
    let set_script_pubkey = tx_output.script_pubkey.write_only();
    view! { cx,
        <div class="mb-1">
            <input on:change=move |e| set_script_pubkey(event_target_value(&e)) class="border border-solid rounded border-black px-1" placeholder="Locking Script Hex"/>
        </div>
        <div class="my-1">
            <ParsedInput input_type="number" value=tx_output.value placeholder="Sats"/>
        </div>
    }
}


#[component]
fn App(cx: Scope) -> impl IntoView {
    let (tx_inputs, set_tx_inputs) = create_signal(cx, vec![TxInputState::new(cx, 0)]);
    let (tx_outputs, set_tx_outputs) = create_signal(cx, vec![TxOutputState::new(cx, 0)]);
    let tx_version_rw = create_rw_signal(cx, 2i32);
    let tx_locktime_rw = create_rw_signal(cx, 0u32);
    let (serialized_tx, set_serialized_tx) = create_signal(cx, String::new());

    let mut new_tx_input = {
        let mut id = 0;
        move |cx| {
            id += 1;
            set_tx_inputs.update(|tx_inputs| tx_inputs.push(TxInputState::new(cx, id)));
        }
    };
    let mut new_tx_output = {
        let mut id = 0;
        move |cx| {
            id += 1;
            set_tx_outputs.update(|tx_outputs| tx_outputs.push(TxOutputState::new(cx, id)));
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
        let input = tx_inputs.with(|tx_inputs| {
            tx_inputs.iter().map(|tx_input| {
                Ok(TxIn {
                    previous_output: OutPoint {
                        txid: tx_input.txid.get().parse()?,
                        vout: tx_input.vout.get(),
                    },
                    script_sig: tx_input.script_sig.get().parse()?,
                    ..Default::default()
                })
            }).collect::<Result<Vec<TxIn>, hashes::hex::Error>>()
        })?;
        let output = tx_outputs.with(|tx_outputs| {
            tx_outputs.iter().map(|tx_output| {
                Ok(TxOut {
                    value: tx_output.value.get(),
                    script_pubkey: tx_output.script_pubkey.get().parse()?,
                    token: None, // TODO
                })
            })
            .collect::<Result<Vec<TxOut>, hashes::hex::Error>>()
        })?;
        let tx = Transaction {
            version: tx_version_rw.get(),
            lock_time: PackedLockTime(tx_locktime_rw.get()),
            input,
            output,
        };
        let mut tx_serialized = vec![];
        tx.consensus_encode(&mut tx_serialized).unwrap();
        Ok::<_, hashes::hex::Error>(tx_serialized.to_hex())
    };

    view! { cx,
        <div>
            <div class="m-1">
                <p>TX version number</p>
                <ParsedInput input_type="number" value=tx_version_rw placeholder="TX version"/>
            </div>
            <div class="m-1">
                <p>Locktime</p>
                <ParsedInput input_type="number" value=tx_locktime_rw placeholder="TX locktime"/>
            </div>
            <p class="mx-1 mt-3">Inputs</p>
            <ol start="0" class="max-w-md">
                <For
                    each=move || 0..tx_inputs.with(Vec::len)
                    key=move |i| tx_inputs.with(|v| v[*i].key)
                    view=move |cx, i| {
                        let tx_input = tx_inputs.with(|v| v[i]);
                        view! {cx,
                            <li class="border border-solid rounded border-black m-1 p-1">
                                <TxInput tx_input=tx_input />
                                <button on:click=move |_| delete_tx_input(tx_input.key) class="border border-solid rounded border-black">"Delete"</button>
                            </li>
                        }
                    }
                />
            </ol>
            <button on:click=move |_| new_tx_input(cx) class="border border-solid rounded border-black m-1">"Add input"</button>
            <p class="mx-1 mt-3">Outputs</p>
            <ol start="0" class="max-w-md">
                <For
                    each=move || 0..tx_outputs.with(Vec::len)
                    key=move |i| tx_outputs.with(|v| v[*i].key)
                    view=move |cx, i| {
                        let tx_output = tx_outputs.with(|v| v[i]);
                        view! {cx,
                            <li class="border border-solid rounded border-black m-1 p-1">
                                <TxOutput tx_output=tx_output />
                                <button on:click=move |_| delete_tx_output(tx_output.key) class="border border-solid rounded border-black">"Delete"</button>
                            </li>
                        }
                    }
                />
            </ol>
            <button on:click=move |_| new_tx_output(cx) class="border border-solid rounded border-black m-1">"Add output"</button>
            <div class="m-1 mt-3">
                <button
                    class="border border-solid rounded border-black"
                    on:click=move |_| {
                        match serialize_tx() {
                            Ok(tx) => set_serialized_tx(tx),
                            Err(e) => set_serialized_tx(format!("{e:?}")),
                        }
                    }
                >
                    "Serialize to hex"
                </button>
            </div>
            <p>{serialized_tx}</p>
        </div>
    }
}

#[component]
fn ParsedInput<T: FromStr + Clone + 'static>(
    cx: Scope,
    value: RwSignal<T>,
    #[prop(default = "")] placeholder: &'static str,
    #[prop(default = "")] input_type: &'static str,
) -> impl IntoView
where
    (Scope, ReadSignal<T>): IntoProperty,
{
    let (parse_success, set_parse_success) = create_signal(cx, true);
    let (thevalue, set_value) = value.split();

    view! { cx,
        <input
            on:input=move |e| {
                let new_value = event_target_value(&e);
                match new_value.parse() {
                    Ok(v) => {
                        set_value(v);
                        set_parse_success(true);
                    }
                    Err(_) => {
                        set_parse_success(false);
                    }
                }
            }
            prop:value=thevalue
            type=input_type
            class="border border-solid rounded px-1"
            class=("border-black", parse_success)
            class=("border-red-700", move || !parse_success())
            placeholder=placeholder
        />
    }
}

#[component]
fn ElectrumThingo(cx: Scope) -> impl IntoView {
    let (cancel_send, mut cancel_recv) = futures::channel::oneshot::channel::<()>();
    on_cleanup(cx, || {
        cancel_send.send(()).ok();
    });

    leptos::spawn_local(async move {
        let client = jsonrpsee::wasm_client::WasmClientBuilder::new()
            .build("wss://chipnet.imaginary.cash:50004")
            .await
            .unwrap();
        leptos::log!("Connected");
        let client = ElectrumClient::new(client);

        // Protocol version negotiation
        let version = client.server_version("").await.unwrap();
        leptos::log!(
            "Server version: {}, protocol version: {}",
            version.server_software_version,
            version.protocol_version
        );

        let (current_head, mut subscription) = client.blockchain_headers_subscribe().await.unwrap();
        leptos::log!("\n{current_head:?}");

        futures::select! {
            _ = cancel_recv => (),
            _ = client.ping_loop().fuse() => (),
            _ = async move {
                loop {
                    let result = subscription.next().await;
                    leptos::log!("\n{result:?}");
                    if result.is_none() {
                        break;
                    }
                }
            }.fuse() => (),
        }
        leptos::log!("Disconnect");
    });
}

#[component]
fn AsyncCounter(cx: Scope) -> impl IntoView {
    let (count, set_count) = create_signal(cx, 0);
    let async_task = || async move {
        loop {
            gloo::timers::future::sleep(Duration::from_secs(1)).await;
            set_count.update(|x| *x += 1);
        }
    };
    leptos::spawn_local(async_task());
    view! { cx, <p>{count}</p> }
}

#[component]
fn SimpleWallet(cx: Scope) -> impl IntoView {
    let secp = Secp256k1::new();
    let mut rng = rand::thread_rng();
    let keypair = KeyPair::new(&secp, &mut rng);

    let (value, set_value) = create_signal(cx, String::new());
    let pubkey = keypair.public_key().to_string();

    view! { cx,
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
fn AddressConverter(cx: Scope) -> impl IntoView {
    let (value, set_value) = create_signal(cx, String::new());

    view! { cx,
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
fn Pump(cx: Scope, #[prop(default = 100)] max: u16) -> impl IntoView {
    let (value, set_value) = create_signal(cx, 0);
    view! { cx,
        <button
            on:click = move |_| set_value.update(|c| *c += 1)
            style="margin-right: 10px"
        >
            "Pump"
        </button>
        <progress max=max value=value/>
    }
}
