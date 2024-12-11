#![deny(rust_2018_idioms)]
#[macro_use]
mod macros;
mod components;
mod electrum_client;
pub mod js_reexport;
pub mod partially_signed;
pub mod util;

use anyhow::Result;
use bitcoincash::consensus::encode;
use bitcoincash::hashes::hex::{FromHex, ToHex};
use bitcoincash::psbt::serialize::{Deserialize, Serialize};
use bitcoincash::secp256k1::Secp256k1;
use bitcoincash::{Network, PackedLockTime, Transaction};
use components::script_input::{ScriptDisplayFormat, ScriptInputValue};
use components::ParsedInput;
use leptos::prelude::{
    event_target_value, mount_to_body, AddAnyAttr, ClassAttribute, ElementChild, For, Get,
    GlobalAttributes, OnAttribute, PropAttribute, Read, ReadSignal, RwSignal, Set, StoredValue,
    Write,
};
use leptos::{component, logging::log, view, IntoView};
use macros::StrEnum;

use crate::components::tx_input::{TxInput, TxInputState};
use crate::components::tx_output::{TxOutput, TxOutputState};
use crate::partially_signed::PartiallySignedTransaction;

impl StrEnum for Network {
    fn to_str(self) -> &'static str {
        match self {
            Network::Bitcoin => "mainnet",
            Network::Testnet => "testnet3",
            Network::Regtest => "regtest",
            Network::Testnet4 => "testnet4",
            Network::Scalenet => "scalenet",
            Network::Chipnet => "chipnet",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "mainnet" => Some(Network::Bitcoin),
            "testnet3" => Some(Network::Testnet),
            "regtest" => Some(Network::Regtest),
            "testnet4" => Some(Network::Testnet4),
            "scalenet" => Some(Network::Scalenet),
            "chipnet" => Some(Network::Chipnet),
            _ => None,
        }
    }
}

fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    mount_to_body(|| view! { <App/> });
}

#[component]
fn App() -> impl IntoView {
    let secp = StoredValue::new(Secp256k1::new());
    let network = RwSignal::new(Network::Bitcoin);
    let tx_inputs = RwSignal::new(vec![TxInputState::new(0, 0)]);
    let tx_outputs = RwSignal::new(vec![TxOutputState::new(0, 0)]);
    let tx_version = RwSignal::new(2i32);
    let tx_locktime = RwSignal::new(0u32);
    let tx_hex = RwSignal::new(String::new());
    let tx_hex_errored = RwSignal::new(false);
    let tx_input_id = RwSignal::new(1);
    let tx_output_id = RwSignal::new(1);
    let serialize_message = RwSignal::new(String::new());

    let ctx = Context {
        network: network.read_only(),
    };

    let new_tx_input = move |t: &mut Vec<TxInputState>| {
        let id = tx_input_id();
        tx_input_id.set(id + 1);
        t.push(TxInputState::new(id, t.len()));
    };
    let new_tx_output = move |t: &mut Vec<TxOutputState>| {
        let id = tx_output_id();
        tx_output_id.set(id + 1);
        t.push(TxOutputState::new(id, t.len()));
    };
    let delete_tx_input = move |key_to_remove| {
        let mut tx_inputs = tx_inputs.write();
        let index_to_remove = tx_inputs
            .iter()
            .enumerate()
            .find(|(_, t)| t.key == key_to_remove)
            .unwrap()
            .0;
        let removed = tx_inputs.remove(index_to_remove);
        removed.dispose();
        for (i, tx) in tx_inputs.iter().enumerate().skip(index_to_remove) {
            tx.index.set(i);
        }
    };
    let delete_tx_output = move |key_to_remove| {
        let mut tx_outputs = tx_outputs.write();
        let index_to_remove = tx_outputs
            .iter()
            .enumerate()
            .find(|(_, t)| t.key == key_to_remove)
            .unwrap()
            .0;
        let removed = tx_outputs.remove(index_to_remove);
        removed.dispose();
        for (i, tx) in tx_outputs.iter().enumerate().skip(index_to_remove) {
            tx.index.set(i);
        }
    };
    let serialize_tx = move || -> Result<String> {
        let input = tx_inputs
            .read()
            .iter()
            .map(|&tx_input| tx_input.try_into())
            .collect::<Result<_, _>>()?;
        let output = tx_outputs
            .read()
            .iter()
            .map(|&tx_output| tx_output.try_into())
            .collect::<Result<_, _>>()?;
        let tx = PartiallySignedTransaction {
            version: tx_version.get(),
            lock_time: PackedLockTime(tx_locktime.get()),
            input,
            output,
        };
        let tx_serialized = tx.serialize();
        let mut sm = serialize_message.write();
        if sm.is_empty() || sm.ends_with('.') {
            *sm = format!("{} bytes", tx_serialized.len());
        } else {
            *sm = format!("{} bytes.", tx_serialized.len());
        }
        Ok(tx_serialized.to_hex())
    };
    let deserialize_tx = move || -> Result<()> {
        serialize_message.set(String::new());
        let hex = Vec::from_hex(&tx_hex.read())?;
        let tx = PartiallySignedTransaction::deserialize(&hex)
            .or_else::<encode::Error, _>(|_| Ok(Transaction::deserialize(&hex)?.into()))?;
        let mut tx_inputs = tx_inputs.write();
        let mut tx_outputs = tx_outputs.write();

        if tx_inputs.len() > tx.input.len() {
            for tx_input in tx_inputs.drain(tx.input.len()..) {
                tx_input.dispose();
            }
        }

        if tx_outputs.len() > tx.output.len() {
            for tx_output in tx_outputs.drain(tx.output.len()..) {
                tx_output.dispose();
            }
        }

        for _ in tx_inputs.len()..tx.input.len() {
            new_tx_input(&mut tx_inputs);
        }
        for _ in tx_outputs.len()..tx.output.len() {
            new_tx_output(&mut tx_outputs);
        }

        tx_version.set(tx.version);
        tx_locktime.set(tx.lock_time.0);

        for (i, input) in tx.input.iter().enumerate() {
            tx_inputs[i].update_from_txin(input);
        }

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
                .set(ScriptInputValue::Hex(script_pubkey_hex));
            tx_outputs[i].value.set(output.value);

            tx_outputs[i]
                .token_data_state
                .update_from_token_data(output.token.as_ref());
        }
        Ok(())
    };
    let reset = move |_| {
        let tx_inputs = &mut *tx_inputs.write();
        let tx_outputs = &mut *tx_outputs.write();

        for tx_input in tx_inputs.drain(..) {
            tx_input.dispose();
        }
        for tx_output in tx_outputs.drain(..) {
            tx_output.dispose();
        }
        new_tx_input(tx_inputs);
        new_tx_output(tx_outputs);
        tx_version.set(2);
        tx_locktime.set(0);
    };

    view! {
        <div class="flex gap-3 justify-between">
            <div class="table">
                <div class="table-row">
                    <div class="table-cell pr-1 pb-1">
                        <label for="tx_version">TX version:</label>
                    </div>
                    <div class="table-cell pb-1">
                        <ParsedInput value={tx_version} {..} id="tx_version" placeholder="2"/>
                    </div>
                </div>
                <div class="table-row">
                    <div class="table-cell pr-1">
                        <label for="tx_locktime">Locktime:</label>
                    </div>
                    <div class="table-cell">
                        <ParsedInput value={tx_locktime} {..} id="tx_locktime" placeholder="0"/>
                    </div>
                </div>
            </div>
            <div class="table">
                <div class="table-row">
                    <div class="table-cell pr-1">
                        <label for="tx_locktime">Network:</label>
                    </div>
                    <div class="table-cell">
                        <select
                            class="bg-inherit border rounded ml-1 p-1 disabled:opacity-30"
                            on:input=move |e| {
                                network.set(Network::from_str(&event_target_value(&e)).unwrap())
                            }
                            prop:value={move || network().to_str()}
                        >
                            <option value={Network::Bitcoin.to_str()}>mainnet</option>
                            <option value={Network::Testnet.to_str()}>testnet3</option>
                            <option value={Network::Regtest.to_str()}>regtest</option>
                            <option value={Network::Testnet4.to_str()}>testnet4</option>
                            <option value={Network::Scalenet.to_str()}>scalenet</option>
                            <option value={Network::Chipnet.to_str()}>chipnet</option>
                        </select>
                    </div>
                </div>
            </div>
        </div>
        <div class="flex flex-wrap gap-3 mt-3">
            <div class="basis-[32rem] grow">
                <p class="mb-1">Inputs</p>
                <ol start="0">
                    <For
                        each=move || 0..tx_inputs.read().len()
                        key=move |i| tx_inputs.read()[*i].key
                        let:i
                    >
                        {
                            let tx_input = tx_inputs.read()[i];
                            view! {
                                <li class="border border-solid rounded-md border-stone-600 p-1 mb-2 bg-stone-800">
                                    <TxInput tx_input secp ctx/>
                                    <div class="flex justify-between">
                                        <button
                                            on:click=move |_| delete_tx_input(tx_input.key)
                                            class="border border-solid rounded border-stone-600 px-2 bg-red-950"
                                        >
                                            "−"
                                        </button>
                                        <span class="text-sm mr-4">"#"{tx_input.index}</span>
                                    </div>
                                </li>
                            }
                        }
                    </For>
                </ol>
                <button
                    on:click=move |_| new_tx_input(&mut tx_inputs.write())
                    class="border border-solid rounded border-stone-600 px-2"
                >
                    "+"
                </button>
            </div>
            <div class="basis-[32rem] grow">
                <p class="mb-1">Outputs</p>
                <ol start="0">
                    <For
                        each=move || 0..tx_outputs.read().len()
                        key=move |i| tx_outputs.read()[*i].key
                        let:i
                    >
                        {
                            let tx_output = tx_outputs.read()[i];
                            view! {
                                <li class="border border-solid rounded border-stone-600 p-1 bg-stone-800 mb-2">
                                    <TxOutput tx_output ctx/>
                                    <div class="flex justify-between">
                                        <button
                                            on:click=move |_| delete_tx_output(tx_output.key)
                                            class="border border-solid rounded border-stone-600 px-2 bg-red-950"
                                        >"−"</button>
                                        <span class="text-sm mr-4">"#"{tx_output.index}</span>
                                    </div>
                                </li>
                            }
                        }
                    </For>
                </ol>
                <button
                    on:click=move |_| new_tx_output(&mut tx_outputs.write())
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
                            tx_hex_errored.set(false);
                            tx_hex.set(tx);
                        }
                        Err(e) => {
                            tx_hex_errored.set(true);
                            tx_hex.set(e.to_string());
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
                            tx_hex_errored.set(true);
                        }
                    }
                }
            >
                "Deserialize"
            </button>
            <button
                class="border border-solid rounded border-stone-600 px-1 mx-1 ml-3 bg-red-950"
                on:click=reset
            >
                "Reset"
            </button>
            <span>{serialize_message}</span>
            <textarea
                spellcheck="false"
                class="border border-solid rounded border-stone-600 px-1 w-full placeholder:text-stone-600 font-mono grow my-1"
                class=("bg-stone-900", move || !tx_hex_errored())
                class=("bg-red-950", tx_hex_errored)
                on:input=move |_| tx_hex_errored.set(false)
                on:change=move |e| tx_hex.set(event_target_value(&e))
                prop:value={tx_hex}
            />
        </div>
    }
}

#[derive(Copy, Clone)]
struct Context {
    network: ReadSignal<Network>,
}

// #[component]
// fn ElectrumThingo() -> impl IntoView {
//     let (cancel_send, mut cancel_recv) = futures::channel::oneshot::channel::<()>();
//     on_cleanup(|| {
//         cancel_send.send(()).ok();
//     });
//
//     leptos::spawn_local(async move {
//         let client = jsonrpsee::wasm_client::WasmClientBuilder::new()
//             .build("wss://chipnet.imaginary.cash:50004")
//             .await
//             .unwrap();
//         log!("Connected");
//         let client = ElectrumClient::new(client);
//
//         // Protocol version negotiation
//         let version = client.server_version("").await.unwrap();
//         log!(
//             "Server version: {}, protocol version: {}",
//             version.server_software_version,
//             version.protocol_version
//         );
//
//         let (current_head, mut subscription) = client.blockchain_headers_subscribe().await.unwrap();
//         log!("\n{current_head:?}");
//
//         futures::select! {
//             _ = cancel_recv => (),
//             _ = client.ping_loop().fuse() => (),
//             _ = async move {
//                 loop {
//                     let result = subscription.next().await;
//                     log!("\n{result:?}");
//                     if result.is_none() {
//                         break;
//                     }
//                 }
//             }.fuse() => (),
//         }
//         log!("Disconnect");
//     });
// }

// #[component]
// fn AsyncCounter() -> impl IntoView {
//     let count = RwSignal::new(0);
//     let async_task = || async move {
//         loop {
//             gloo::timers::future::sleep(Duration::from_secs(1)).await;
//             count.update(|x| *x += 1);
//         }
//     };
//     leptos::spawn_local(async_task());
//     view! { <p>{count}</p> }
// }

// use bitcoincash::hashes::sha256;
// use bitcoincash::secp256k1::{rand, Message};
// use bitcoincash::KeyPair;
// #[component]
// fn SimpleWallet() -> impl IntoView {
//     let secp = Secp256k1::new();
//     let mut rng = rand::thread_rng();
//     let keypair = KeyPair::new(&secp, &mut rng);
//
//     let value = RwSignal::new(String::new());
//     let pubkey = keypair.public_key().to_string();
//
//     view! {
//         <p>"Public key: " {pubkey}</p>
//         <p>
//             "Message to sign: "
//             <input
//                 on:change=move |e| value.set(event_target_value(&e))
//             />
//         </p>
//         <p>
//             "Signature: "
//             {move || {
//                 let sig = secp.sign_ecdsa(&Message::from_hashed_data::<sha256::Hash>(value().as_bytes()), &keypair.secret_key());
//                 sig.to_string()
//             }}
//         </p>
//     }
// }
