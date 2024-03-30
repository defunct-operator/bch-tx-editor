#![deny(rust_2018_idioms)]
mod components;
mod electrum_client;
pub mod partially_signed;
pub mod util;

use std::time::Duration;

use anyhow::Result;
use bitcoincash::consensus::encode;
use bitcoincash::hashes::hex::{FromHex, ToHex};
use bitcoincash::hashes::sha256;
use bitcoincash::psbt::serialize::{Deserialize, Serialize};
use bitcoincash::secp256k1::{rand, Message, Secp256k1};
use bitcoincash::{KeyPair, PackedLockTime, Transaction};
use components::tx_output::ScriptDisplayFormat;
use components::ParsedInput;
use futures::future::FutureExt;
use futures::StreamExt;
use leptos::{
    component, create_rw_signal, create_signal, event_target_value, logging::log, mount_to_body,
    on_cleanup, view, For, IntoView, SignalGet, SignalSet, SignalUpdate,
    SignalWith,
};

use crate::components::tx_input::{TxInput, TxInputState};
use crate::components::tx_output::{ScriptPubkeyData, TxOutput, TxOutputState};
use crate::electrum_client::ElectrumClient;
use crate::partially_signed::PartiallySignedTransaction;

fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    mount_to_body(|| view! { <App/> });
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
        let tx = PartiallySignedTransaction::deserialize(&hex).or_else::<encode::Error, _>(|_| Ok(Transaction::deserialize(&hex)?.into()))?;

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
                    .set(input.previous_output().txid.to_string());
                tx_inputs[i].vout.set(input.previous_output().vout);
                tx_inputs[i].script_sig.set(input.script_sig().unwrap().to_hex());
                tx_inputs[i].sequence.set(input.sequence().0);
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
                } else {
                    tx_outputs[i]
                        .script_display_format
                        .set(ScriptDisplayFormat::Addr);
                }
                tx_outputs[i]
                    .script_pubkey
                    .set(ScriptPubkeyData::Hex(script_pubkey_hex));
                tx_outputs[i].value.set(output.value);

                tx_outputs[i]
                    .token_data_state
                    .update_from_token_data(output.token.as_ref());
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
