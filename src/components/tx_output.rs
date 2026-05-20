use anyhow::Result;
use bitcoincash::TxOut;
use leptos::{
    IntoView, component,
    prelude::{
        AddAnyAttr, ClassAttribute, ElementChild, Get, OnAttribute, PropAttribute, RwSignal, Set,
        event_target_checked, event_target_value,
    },
    reactive::signal::ArcRwSignal,
    view,
};

use crate::{
    Context,
    components::{
        ParsedInput,
        drag_handle::DragHandle,
        script_input::{ScriptDisplayFormat, ScriptInput, ScriptInputValue},
        token_data::{TokenData, TokenDataState},
    },
    macros::StrEnum,
};

#[derive(Clone)]
pub struct TxOutputState {
    pub value: ArcRwSignal<u64>,
    pub script_pubkey: ArcRwSignal<ScriptInputValue>,
    pub script_display_format: ArcRwSignal<ScriptDisplayFormat>,
    pub token_data_state: TokenDataState,
    pub key: usize,
}

impl TxOutputState {
    pub fn new(key: usize) -> Self {
        Self {
            value: ArcRwSignal::new(0),
            script_pubkey: ArcRwSignal::default(),
            script_display_format: ArcRwSignal::new(ScriptDisplayFormat::Addr),
            token_data_state: TokenDataState::new(key),
            key,
        }
    }
}

impl TryFrom<TxOutputState> for TxOut {
    type Error = anyhow::Error;
    fn try_from(tx_output: TxOutputState) -> Result<Self, Self::Error> {
        let script_pubkey = tx_output.script_pubkey.get().try_into()?;
        let token = tx_output.token_data_state.token_data()?;
        Ok(TxOut {
            value: tx_output.value.get(),
            script_pubkey,
            token,
        })
    }
}

#[component]
pub fn TxOutput(
    tx_output: TxOutputState,
    ctx: Context,
    set_draggable: impl Fn(bool) + 'static,
) -> impl IntoView {
    let script_pubkey = RwSignal::from(tx_output.script_pubkey);
    let script_format = RwSignal::from(tx_output.script_display_format);
    let cashtoken_enabled = RwSignal::from(tx_output.token_data_state.cashtoken_enabled.clone());
    let tx_output_value = RwSignal::from(tx_output.value);

    let parsed_input_val_id = format!("tx-output-val-{}", tx_output.key);

    view! {
        // Address
        <div class="mb-1 flex">
            <ScriptInput
                value=script_pubkey
                format=script_format
                network=ctx.network
                oneline=true
                {..}
                rows=1
                placeholder=move || {
                    match script_format() {
                        ScriptDisplayFormat::Addr => "Address",
                        ScriptDisplayFormat::Hex => "Locking Script Hex",
                        ScriptDisplayFormat::Asm => "Locking Script Asm",
                        ScriptDisplayFormat::P2sh => "this is not supposed to happen",
                    }
                }
            />
            <div>
                <select
                    autocomplete="off"
                    class="bg-stone-900 border border-stone-600 rounded ml-1 p-1"
                    on:input=move |e| {
                        script_format
                            .set(ScriptDisplayFormat::from_str(&event_target_value(&e)).unwrap())
                    }
                    prop:value=move || script_format().to_str()
                >
                    <option value=ScriptDisplayFormat::Addr.to_str()>Address</option>
                    <option value=ScriptDisplayFormat::Asm.to_str()>Asm</option>
                    <option value=ScriptDisplayFormat::Hex.to_str()>Hex</option>
                </select>
            </div>
            <div class=("cursor-grab", true) on:mousedown=move |_| set_draggable(true)>
                <DragHandle />
            </div>
        </div>

        // Amount
        <div class="my-1">
            <label class="mr-1" for=parsed_input_val_id.clone()>
                Sats:
            </label>
            <ParsedInput
                value={tx_output_value}
                {..}
                id=parsed_input_val_id
                placeholder="Sats"
                class=("w-52", true)
            />
            <label>
                <input
                    type="checkbox"
                    class="ml-5"
                    on:change=move |e| cashtoken_enabled.set(event_target_checked(&e))
                    prop:checked=cashtoken_enabled
                />
                CashToken
            </label>
        </div>

        <TokenData token_data=tx_output.token_data_state />
    }
}
