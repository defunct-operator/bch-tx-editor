#![deny(rust_2018_idioms)]
mod electrum_client;

use std::str::FromStr;
use std::time::Duration;

use bitcoincash::consensus::Encodable;
use bitcoincash::{hashes, Address, Network};
use bitcoincash::hashes::hex::ToHex;
use bitcoincash::hashes::sha256;
use bitcoincash::secp256k1::{rand, Message, Secp256k1};
use bitcoincash::{KeyPair, OutPoint, PackedLockTime, Script, Transaction, TxIn, TxOut};
use futures::future::FutureExt;
use futures::StreamExt;
use leptos::{
    component, create_node_ref, create_rw_signal, create_signal, event_target_value, log,
    mount_to_body, on_cleanup, view, CollectView, For, IntoAttribute, IntoProperty, IntoView,
    MaybeSignal, ReadSignal, RwSignal, Signal, SignalGet, SignalUpdate, SignalWith, WriteSignal, AttributeValue,
};

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

#[derive(Clone)]
enum ScriptPubkeyData {
    Hex(String),
    Addr(String),
}

impl TryFrom<ScriptPubkeyData> for Script {
    type Error = String;
    fn try_from(s: ScriptPubkeyData) -> Result<Self, Self::Error> {
        match s {
            ScriptPubkeyData::Hex(s) => s.parse::<Script>().map_err(|e| e.to_string()),
            ScriptPubkeyData::Addr(s) => s.parse::<Address>().map(|a| a.script_pubkey()).map_err(|e| e.to_string()),
        }
    }
}

#[derive(Copy, Clone)]
struct TxOutputState {
    value: RwSignal<u64>,
    script_pubkey: RwSignal<ScriptPubkeyData>,
    key: usize,
}

impl TxOutputState {
    fn new(key: usize) -> Self {
        Self {
            value: create_rw_signal(0),
            script_pubkey: create_rw_signal(ScriptPubkeyData::Hex("".into())),
            key,
        }
    }
}

impl TryFrom<TxOutputState> for TxOut {
    type Error = String;
    fn try_from(tx_output: TxOutputState) -> Result<Self, Self::Error> {
        let script_pubkey = tx_output.script_pubkey.get().try_into()?;
        Ok(TxOut {
            value: tx_output.value.get(),
            script_pubkey,
            token: None, // TODO
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
fn TxOutput(tx_output: TxOutputState) -> impl IntoView {
    let (script_pubkey, set_script_pubkey) = tx_output.script_pubkey.split();
    let (script_format, set_script_format) = create_signal(String::from("hex"));
    let (script_pubkey_enabled, set_script_pubkey_enabled) = create_signal(true);
    let (script_pubkey_error, set_script_pubkey_error) = create_signal(false);

    let render_script_pubkey = move || {
        match &*script_format() {
            "hex" => {
                match script_pubkey() { // If empty addr or already hex, render as is
                    ScriptPubkeyData::Addr(s) if s.is_empty() => {
                        set_script_pubkey_enabled(true);
                        set_script_pubkey_error(false);
                        return s
                    }
                    ScriptPubkeyData::Hex(s) => {
                        set_script_pubkey_enabled(true);
                        set_script_pubkey_error(false);
                        return s
                    }
                    _ => (),
                }
                match Script::try_from(script_pubkey()) {
                    Ok(s) => {
                        set_script_pubkey_enabled(true);
                        set_script_pubkey_error(false);
                        s.to_hex()
                    }
                    Err(e) => {
                        set_script_pubkey_enabled(false);
                        set_script_pubkey_error(true);
                        e
                    }
                }
            }
            "asm" => {
                set_script_pubkey_enabled(false);
                let script: Result<Script, String> = script_pubkey().try_into();
                match script {
                    Ok(s) => {
                        set_script_pubkey_error(false);
                        s.asm()
                    }
                    Err(e) => {
                        set_script_pubkey_error(true);
                        e
                    }
                }
            }
            "addr" => {
                match script_pubkey() { // If empty hex or already addr, render as is
                    ScriptPubkeyData::Hex(s) if s.is_empty() => {
                        set_script_pubkey_error(false);
                        set_script_pubkey_enabled(true);
                        return s;
                    }
                    ScriptPubkeyData::Addr(s) => {
                        set_script_pubkey_error(false);
                        set_script_pubkey_enabled(true);
                        return s;
                    }
                    _ => (),
                }
                let script = match script_pubkey().try_into() {
                    Ok(s) => s,
                    Err(e) => {
                        set_script_pubkey_error(true);
                        set_script_pubkey_enabled(false);
                        return e
                    }
                };
                match Address::from_script(&script, Network::Bitcoin) {
                    Ok(a) => {
                        set_script_pubkey_enabled(true);
                        set_script_pubkey_error(false);
                        a.to_string()
                    }
                    Err(e) => {
                        set_script_pubkey_enabled(false);
                        set_script_pubkey_error(true);
                        e.to_string()
                    }

                }
            }
            _ => {
                set_script_pubkey_error(true);
                set_script_pubkey_enabled(false);
                "???".into()
            }
        }
    };

    view! {
        <div class="mb-1">
            <div class="flex">
                <textarea
                    rows=1
                    on:change=move |e| {
                        match &*script_format() {
                            "hex" => set_script_pubkey(ScriptPubkeyData::Hex(event_target_value(&e))),
                            "addr" => set_script_pubkey(ScriptPubkeyData::Addr(event_target_value(&e))),
                            _ => unreachable!(),
                        }
                    }
                    class="border border-solid rounded border-stone-600 px-1 w-full bg-inherit placeholder:text-stone-600 font-mono grow"
                    placeholder="Locking Script Hex"
                    prop:value=render_script_pubkey
                    disabled=move || !script_pubkey_enabled()
                    class=("text-red-700", script_pubkey_error)
                />
                <div>
                    <select
                        class="bg-inherit border rounded ml-1 p-1"
                        on:input=move |e| set_script_format(event_target_value(&e))
                    >
                        <option value="hex">Hex</option>
                        <option value="asm">Asm</option>
                        <option value="addr">Address</option>
                    </select>
                </div>
            </div>
        </div>
        <div class="my-1">
            <ParsedInput value=tx_output.value placeholder="Sats" class="w-52"/>
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
fn ParsedInput<T: FromStr + Clone + 'static>(
    value: RwSignal<T>,
    #[prop(default = "")] placeholder: &'static str,
    #[prop(default = "")] class: &'static str,
) -> impl IntoView
where
    ReadSignal<T>: IntoProperty,
{
    let (parse_success, set_parse_success) = create_signal(true);
    let (thevalue, set_value) = value.split();

    view! {
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
            class={move || format!("border border-solid rounded px-1 bg-inherit placeholder:text-stone-600 {}", class)}
            class=("border-stone-600", parse_success)
            class=("border-red-700", move || !parse_success())
            placeholder=placeholder
        />
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
