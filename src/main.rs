#[macro_use]
mod macros;
mod components;
mod electrum_client;
pub mod js_reexport;
pub mod leptos_drag_reorder;
pub mod partially_signed;
pub mod spv;
pub mod unbounded_rx_mut_stream;
pub mod util;

use anyhow::Result;
use bitcoincash::consensus::encode;
use bitcoincash::hashes::hex::{FromHex, ToHex};
use bitcoincash::psbt::serialize::{Deserialize, Serialize};
use bitcoincash::secp256k1::Secp256k1;
use bitcoincash::{Network, PackedLockTime, Transaction, Txid};
use components::ParsedInput;
use components::script_input::{ScriptDisplayFormat, ScriptInputValue};
use leptos::prelude::{
    AddAnyAttr, ClassAttribute, ElementChild, ForEnumerate, Get, GlobalAttributes,
    NodeRefAttribute, OnAttribute, PropAttribute, Read, ReadSignal, RwSignal, Set, Show,
    StoredValue, Write, event_target_value, mount_to_body, untrack,
};
use leptos::reactive::effect::Effect;
use leptos::{IntoView, component, logging::log, view};
use leptos_use::use_element_visibility;
use macros::StrEnum;
use tracing::Level;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{filter::Targets, layer::SubscriberExt};
use wasm_tracing::{WasmLayer, WasmLayerConfig};

use crate::components::tx_input::{TxInput, TxInputState};
use crate::components::tx_output::{TxOutput, TxOutputState};
use crate::leptos_drag_reorder::{
    HoverPosition, UseDragReorderReturn, provide_drag_reorder, use_drag_reorder,
};
use crate::partially_signed::PartiallySignedTransaction;
use crate::spv::{SpvConnStatus, SpvModal, provide_spv, use_spv};
use crate::util::script_to_cash_addr;

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
    console_error_panic_hook::set_once();
    tracing_subscriber::registry()
        .with(WasmLayer::new(WasmLayerConfig::default()))
        .with(
            Targets::new()
                // .with_target("bch_tx_editor", Level::TRACE)
                .with_default(Level::INFO),
        )
        .init();
    mount_to_body(|| view! { <App /> });
}

#[component]
fn App() -> impl IntoView {
    provide_spv();

    let spv = use_spv();
    let spv_status = ReadSignal::from(spv.status.clone());
    let secp = StoredValue::new(Secp256k1::new());
    let network = RwSignal::new(Network::Bitcoin);
    let tx_inputs = RwSignal::new_local(vec![TxInputState::new(0)]);
    let tx_outputs = RwSignal::new_local(vec![TxOutputState::new(0)]);
    let tx_version = RwSignal::new(2i32);
    let tx_locktime = RwSignal::new(0u32);
    let tx_hex = RwSignal::new(String::new());
    let tx_hex_errored = RwSignal::new(false);
    let tx_input_id = RwSignal::new(1);
    let tx_output_id = RwSignal::new(1);
    let serialize_message = RwSignal::new(String::new());
    let show_spv_modal = RwSignal::new(false);
    let txid_to_load = RwSignal::new(String::new()); // read by an effect

    let ctx = Context {
        network: network.read_only(),
    };

    let new_tx_input = move |t: &mut Vec<TxInputState>| {
        let id = tx_input_id();
        tx_input_id.set(id + 1);
        t.push(TxInputState::new(id));
    };
    let new_tx_output = move |t: &mut Vec<TxOutputState>| {
        let id = tx_output_id();
        tx_output_id.set(id + 1);
        t.push(TxOutputState::new(id));
    };
    let delete_tx_input = move |key_to_remove| {
        let mut tx_inputs = tx_inputs.write();
        let index_to_remove = tx_inputs
            .iter()
            .enumerate()
            .find(|(_, t)| t.key == key_to_remove)
            .unwrap()
            .0;
        tx_inputs.remove(index_to_remove);
    };
    let delete_tx_output = move |key_to_remove| {
        let mut tx_outputs = tx_outputs.write();
        let index_to_remove = tx_outputs
            .iter()
            .enumerate()
            .find(|(_, t)| t.key == key_to_remove)
            .unwrap()
            .0;
        tx_outputs.remove(index_to_remove);
    };
    let serialize_tx = move || -> Result<String> {
        let input = tx_inputs
            .read()
            .iter()
            .map(|tx_input| tx_input.clone().try_into())
            .collect::<Result<_, _>>()?;
        let output = tx_outputs
            .read()
            .iter()
            .map(|tx_output| tx_output.clone().try_into())
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
        let hex = Vec::from_hex(
            &tx_hex
                .read()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>(),
        )?;
        let tx = PartiallySignedTransaction::deserialize(&hex)
            .or_else::<encode::Error, _>(|_| Ok(Transaction::deserialize(&hex)?.into()))?;
        let mut tx_inputs = tx_inputs.write();
        let mut tx_outputs = tx_outputs.write();

        if tx_inputs.len() > tx.input.len() {
            tx_inputs.drain(tx.input.len()..);
        }

        if tx_outputs.len() > tx.output.len() {
            tx_outputs.drain(tx.output.len()..);
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
            if script_to_cash_addr(&output.script_pubkey, network()).is_ok() {
                tx_outputs[i]
                    .script_display_format
                    .set(ScriptDisplayFormat::Addr);
            } else {
                tx_outputs[i]
                    .script_display_format
                    .set(ScriptDisplayFormat::Asm);
            }
            tx_outputs[i]
                .script_pubkey
                .set(ScriptInputValue::Hex(script_pubkey_hex));
            tx_outputs[i].value.set(output.value);

            tx_outputs[i]
                .token_data_state
                .update_from_token_data(output.token.as_ref());
        }
        tx_hex.write().clear();
        Ok(())
    };
    let reset = move |_| {
        let tx_inputs = &mut *tx_inputs.write();
        let tx_outputs = &mut *tx_outputs.write();

        tx_inputs.clear();
        tx_outputs.clear();
        new_tx_input(tx_inputs);
        new_tx_output(tx_outputs);
        tx_version.set(2);
        tx_locktime.set(0);
    };

    let tx_fetcher = spv.tx_fetcher.clone();
    Effect::new(move || {
        let txid_to_load = txid_to_load();
        let txid_to_load = txid_to_load.trim();
        if txid_to_load.is_empty() {
            return;
        }
        let txid: Txid = match txid_to_load.parse() {
            Ok(x) => x,
            Err(e) => {
                tx_hex_errored(true);
                serialize_message(e.to_string());
                return;
            }
        };
        let fetch_result = tx_fetcher.get(txid);
        match fetch_result() {
            None => (),
            Some(Ok(tx_raw)) => {
                serialize_message.write().clear();
                tx_hex(tx_raw.to_hex());
                tx_hex_errored(false);
                if let Err(e) = untrack(deserialize_tx) {
                    log!("Deserialization error: {e}");
                    tx_hex_errored.set(true);
                }
            }
            Some(Err(e)) => {
                tx_hex_errored(true);
                serialize_message(e.to_string())
            }
        }
    });

    let [txinput_column_ref] = provide_drag_reorder([tx_inputs], |p| p.key.to_string().into());
    let [txoutput_column_ref] = provide_drag_reorder([tx_outputs], |p| p.key.to_string().into());
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
                        <label for="network">Network:</label>
                    </div>
                    <div class="table-cell">
                        <select
                            class="bg-stone-900 border border-stone-600 rounded ml-1 p-1 disabled:opacity-30"
                            on:input=move |e| {
                                network.set(Network::from_str(&event_target_value(&e)).unwrap())
                            }
                            prop:value={move || network().to_str()}
                            id="network"
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
                <div class="table-row">
                    <div class="table-cell text-right">
                        <button
                            on:click=move |_| show_spv_modal(true)
                            class="border border-solid rounded border-transparent hover:border-stone-600 px-1 inline-flex gap-2 align-top items-center"
                        >
                            <div
                                class="w-[5px] h-[5px] rounded-full"
                                class=("bg-gray-600", move || spv_status() == SpvConnStatus::Disabled)
                                class=("bg-yellow-600", move || spv_status() == SpvConnStatus::Connecting)
                                class=("bg-red-600", move || spv_status() == SpvConnStatus::Disconnected)
                                class=("bg-green-600", move || spv_status() == SpvConnStatus::Connected)
                            ></div>
                            <div>"SPV:"</div>
                        </button>
                    </div>
                    <div class="table-cell">
                        <select
                            class="bg-stone-900 border border-stone-600 rounded ml-1 p-1"
                            on:input=move |e| {
                                spv.set_enabled(event_target_value(&e) == "enabled");
                            }
                            id="spv_enabled"
                        >
                            <option value={"disabled"}>Disabled</option>
                            <option value={"enabled"}>Enabled</option>
                        </select>
                    </div>
                </div>
            </div>
        </div>
        <div class="flex flex-wrap gap-x-3 gap-y-10 mt-3">

            // Inputs
            <div class="basis-lg grow">
                <p class="mb-1 text-xl">Inputs</p>
                <ol node_ref=txinput_column_ref start="0">
                    <ForEnumerate
                        each=tx_inputs
                        key=move |t| t.key
                        let(index, tx_input)
                    >
                        {
                            let UseDragReorderReturn {
                                node_ref,
                                draggable,
                                set_draggable,
                                hover_position,
                                on_dragstart,
                                on_dragend,
                                ..
                            } = use_drag_reorder::<_, TxInputState>(tx_input.key.to_string());
                            let is_visible = use_element_visibility(node_ref);
                            let tx_input_key = tx_input.key;

                            view! {
                                <li
                                    node_ref=node_ref
                                    class="border border-solid rounded-md border-stone-600 p-1 mb-2 bg-stone-800 panel"
                                    class=("panel--above", move || matches!(hover_position.get(), Some(HoverPosition::Above)))
                                    class=("panel--below", move || matches!(hover_position.get(), Some(HoverPosition::Below)))
                                    draggable=move || draggable.get().then_some("true")
                                    class=("opacity-50", draggable)
                                    on:dragstart=on_dragstart
                                    on:dragend=on_dragend
                                >
                                    <TxInput tx_input secp ctx set_draggable is_visible/>
                                    <div class="flex justify-between">
                                        <button
                                            on:click=move |_| delete_tx_input(tx_input_key)
                                            class="border border-solid rounded border-stone-600 px-2 bg-red-950"
                                        >
                                            "−"
                                        </button>
                                        <span class="text-sm mr-4">"#"{index}</span>
                                    </div>
                                </li>
                            }
                        }
                    </ForEnumerate>
                </ol>
                <button
                    on:click=move |_| new_tx_input(&mut tx_inputs.write())
                    class="border border-solid rounded border-stone-600 px-2"
                >
                    "+"
                </button>
            </div>

            // Outputs
            <div class="basis-lg grow">
                <p class="mb-1 text-xl">Outputs</p>
                <ol node_ref=txoutput_column_ref start="0">
                    <ForEnumerate
                        each=tx_outputs
                        key=move |t| t.key
                        let:(index, tx_output)
                    >
                        {
                            let UseDragReorderReturn {
                                node_ref,
                                draggable,
                                set_draggable,
                                hover_position,
                                on_dragstart,
                                on_dragend,
                                ..
                            } = use_drag_reorder::<_, TxOutputState>(tx_output.key.to_string());
                            let tx_output_key = tx_output.key;

                            view! {
                                <li
                                    node_ref=node_ref
                                    class="border border-solid rounded border-stone-600 p-1 bg-stone-800 mb-2 panel"
                                    class=("panel--above", move || matches!(hover_position.get(), Some(HoverPosition::Above)))
                                    class=("panel--below", move || matches!(hover_position.get(), Some(HoverPosition::Below)))
                                    draggable=move || draggable.get().then_some("true")
                                    class=("opacity-50", draggable)
                                    on:dragstart=on_dragstart
                                    on:dragend=on_dragend
                                >
                                    <TxOutput tx_output ctx set_draggable/>
                                    <div class="flex justify-between">
                                        <button
                                            on:click=move |_| delete_tx_output(tx_output_key)
                                            class="border border-solid rounded border-stone-600 px-2 bg-red-950"
                                        >"−"</button>
                                        <span class="text-sm mr-4">"#"{index}</span>
                                    </div>
                                </li>
                            }
                        }
                    </ForEnumerate>
                </ol>
                <button
                    on:click=move |_| new_tx_output(&mut tx_outputs.write())
                    class="border border-solid rounded border-stone-600 px-2"
                >
                    "+"
                </button>
            </div>
        </div>
        <div class="mt-10">
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
                    if let Err(e) = deserialize_tx() {
                        log!("Deserialization error: {e}");
                        tx_hex_errored.set(true);
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
            <button
                class="border border-solid rounded border-stone-600 px-1 mx-1 ml-3 disabled:opacity-30"
                on:click=move |_| txid_to_load(tx_hex())
                disabled=move || spv_status() != SpvConnStatus::Connected
                title=move ||
                    if spv_status() != SpvConnStatus::Connected {
                        "SPV not connected"
                    } else {
                        ""
                    }
            >
                "Load from network"
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
        <Show when=show_spv_modal>
            <SpvModal on_exit=move || show_spv_modal(false) />
        </Show>
    }
}

#[derive(Copy, Clone)]
struct Context {
    network: ReadSignal<Network>,
}

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
