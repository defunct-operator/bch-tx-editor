#![deny(rust_2018_idioms)]
mod components;
mod electrum_client;

use std::rc::Rc;
use std::time::Duration;

use anyhow::Result;
use bitcoincash::hashes::hex::{FromHex, ToHex};
use bitcoincash::hashes::{self, sha256};
use bitcoincash::psbt::serialize::{Deserialize, Serialize};
use bitcoincash::secp256k1::{rand, Message, Secp256k1};
use bitcoincash::{KeyPair, OutPoint, PackedLockTime, Script, Sequence, Transaction, TxIn};
use components::tx_output::ScriptDisplayFormat;
use components::ParsedInput;
use futures::future::FutureExt;
use futures::StreamExt;
use leptos::{
    component, create_rw_signal, create_signal, event_target_value, logging::log, mount_to_body,
    on_cleanup, view, For, IntoView, RwSignal, SignalDispose, SignalGet, SignalSet, SignalUpdate,
    SignalWith,
};

use crate::components::tx_output::{ScriptPubkeyData, TxOutput, TxOutputState};
use crate::electrum_client::ElectrumClient;

fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
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
    let (txid, set_txid) = tx_input.txid.split();
    let (script_sig, set_script_sig) = tx_input.script_sig.split();
    let (script_format, set_script_format) = create_signal(String::from("hex"));
    let (script_enabled, set_script_enabled) = create_signal(true);
    let (script_error, set_script_error) = create_signal(false);

    let parsed_input_seq_id = format!("tx-input-sn-{}", tx_input.key);

    let try_render_script = move || -> Result<String> {
        match &*script_format() {
            "hex" => {
                set_script_enabled(true);
                Ok(script_sig())
            }
            "asm" => {
                set_script_enabled(false);
                let mut s = script_sig();
                s.retain(|c| !c.is_ascii_whitespace());
                let s = Script::from_hex(&s)?;
                Ok(s.asm())
            }
            _ => unreachable!(),
        }
    };
    let render_script = move || match try_render_script() {
        Ok(s) => {
            set_script_error(false);
            s
        }
        Err(e) => {
            set_script_error(true);
            e.to_string()
        }
    };

    view! {
        <div class="mb-1 flex">
            <input
                on:change=move |e| set_txid(event_target_value(&e))
                class=concat!(
                    "border border-solid rounded border-stone-600 px-1 w-full bg-stone-900 ",
                    "placeholder:text-stone-600 font-mono grow",
                )
                prop:value=txid
                placeholder="Transaction ID"
            />
            <span>:</span>
            <ParsedInput value=tx_input.vout placeholder="Index" class="w-16" id=""/>
        </div>
        <div class="mb-1 flex">
            <textarea
                spellcheck="false"
                on:change=move |e| set_script_sig(event_target_value(&e))
                class=concat!(
                    "border border-solid rounded border-stone-600 px-1 w-full bg-inherit ",
                    "placeholder:text-stone-600 font-mono bg-stone-900 grow",
                )
                placeholder="Unlocking Script Hex"
                prop:value=render_script
                disabled=move || !script_enabled()
                class=("text-red-700", script_error)
            />
            <div>
                <select
                    class="bg-inherit border rounded ml-1 p-1"
                    on:input=move |e| set_script_format(event_target_value(&e))
                    prop:value={move || script_format()}
                >
                    <option value="hex">Hex</option>
                    <option value="asm">Asm</option>
                </select>
            </div>
        </div>
        <div class="my-1">
            <label class="mr-1" for=parsed_input_seq_id.clone()>Sequence Number:</label>
            <ParsedInput id=parsed_input_seq_id value=tx_input.sequence placeholder="Sequence"/>
        </div>
    }
}

#[component]
fn App() -> impl IntoView {
    let (tx_inputs, set_tx_inputs) = create_signal(vec![TxInputState::new(0)]);
    let (tx_outputs, set_tx_outputs) = create_signal(vec![TxOutputState::new(0)]);
    let tx_version_rw = create_rw_signal(2i32);
    let tx_locktime_rw = create_rw_signal(0u32);
    let (tx_hex, set_tx_hex) = create_signal(String::new());
    let (tx_hex_errored, set_tx_hex_errored) = create_signal(false);
    let (tx_input_id, set_tx_input_id) = create_signal(1);
    let (tx_output_id, set_tx_output_id) = create_signal(1);

    let new_tx_input = move || {
        let id = tx_input_id();
        set_tx_input_id(id + 1);
        set_tx_inputs.update(|tx_inputs| tx_inputs.push(TxInputState::new(id)));
    };
    let new_tx_output = move || {
        let id = tx_output_id();
        set_tx_output_id(id + 1);
        set_tx_outputs.update(|tx_outputs| tx_outputs.push(TxOutputState::new(id)));
    };
    let delete_tx_input = move |key_to_remove| {
        set_tx_inputs.update(|tx_inputs| {
            let index_to_remove = tx_inputs
                .iter()
                .enumerate()
                .find(|(_, t)| t.key == key_to_remove)
                .unwrap()
                .0;
            let removed = tx_inputs.remove(index_to_remove);
            removed.dispose();
        });
    };
    let delete_tx_output = move |key_to_remove| {
        set_tx_outputs.update(|tx_outputs| {
            let index_to_remove = tx_outputs
                .iter()
                .enumerate()
                .find(|(_, t)| t.key == key_to_remove)
                .unwrap()
                .0;
            let removed = tx_outputs.remove(index_to_remove);
            removed.dispose();
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
        let tx_serialized = tx.serialize();
        Ok(tx_serialized.to_hex())
    };
    let deserialize_tx = move || -> Result<()> {
        let hex = tx_hex.with(|t| Vec::from_hex(t))?;
        let tx = Transaction::deserialize(&hex)?;

        let mut current_input_len = 0;
        set_tx_inputs.update(|tx_inputs| {
            if tx_inputs.len() > tx.input.len() {
                for tx_input in tx_inputs.drain(tx.input.len()..) {
                    tx_input.dispose();
                }
            }
            current_input_len = tx_inputs.len();
        });
        let mut current_output_len = 0;
        set_tx_outputs.update(|tx_outputs| {
            if tx_outputs.len() > tx.output.len() {
                for tx_output in tx_outputs.drain(tx.output.len()..) {
                    tx_output.dispose();
                }
            }
            current_output_len = tx_outputs.len();
        });

        for _ in current_input_len..tx.input.len() {
            new_tx_input();
        }
        for _ in current_output_len..tx.output.len() {
            new_tx_output();
        }

        tx_version_rw.set(tx.version);
        tx_locktime_rw.set(tx.lock_time.0);

        tx_inputs.with(|tx_inputs| {
            for (i, input) in tx.input.iter().enumerate() {
                tx_inputs[i]
                    .txid
                    .set(input.previous_output.txid.to_string());
                tx_inputs[i].vout.set(input.previous_output.vout);
                tx_inputs[i].script_sig.set(input.script_sig.to_hex());
                tx_inputs[i].sequence.set(input.sequence.0);
            }
        });

        tx_outputs.with(|tx_outputs| {
            for (i, output) in tx.output.iter().enumerate() {
                let script_pubkey_hex = output.script_pubkey.to_hex();
                if script_pubkey_hex.starts_with("6a") {
                    // OP_RETURN script
                    tx_outputs[i]
                        .script_display_format
                        .set(ScriptDisplayFormat::Asm);
                }
                tx_outputs[i]
                    .script_pubkey
                    .set(ScriptPubkeyData::Hex(script_pubkey_hex));
                tx_outputs[i].value.set(output.value);
            }
        });
        Ok(())
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
                        let:i
                    >
                        {
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
                    </For>
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
                        let:i
                    >
                        {
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
                    </For>
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
                        Ok(tx) => {
                            set_tx_hex_errored(false);
                            set_tx_hex(tx);
                        }
                        Err(e) => {
                            set_tx_hex_errored(true);
                            set_tx_hex(e.to_string());
                        }
                    }
                }
            >
                "Serialize"
            </button>
            <button
                class="border border-solid rounded border-stone-600 px-1 mx-1"
                on:click=move |_| {
                    match deserialize_tx() {
                        Ok(_) => (),
                        Err(e) => {
                            log!("Deserialization error: {e}");
                            set_tx_hex_errored(true);
                        }
                    }
                }
            >
                "Deserialize"
            </button>
            <textarea
                spellcheck="false"
                class="border border-solid rounded border-stone-600 px-1 w-full placeholder:text-stone-600 font-mono grow my-1"
                class=("bg-stone-900", move || !tx_hex_errored())
                class=("bg-red-950", tx_hex_errored)
                on:input=move |_| set_tx_hex_errored(false)
                on:change=move |e| set_tx_hex(event_target_value(&e))
                prop:value={tx_hex}
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
