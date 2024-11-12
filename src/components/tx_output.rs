use anyhow::Result;
use bitcoincash::TxOut;
use leptos::{
    component, event_target_checked, event_target_value, view, IntoView, RwSignal, SignalDispose,
    SignalGet, SignalSet,
};

use crate::{
    components::{
        script_input::{ScriptDisplayFormat, ScriptInput, ScriptInputValue},
        token_data::{TokenData, TokenDataState},
        ParsedInput,
    },
    macros::StrEnum,
    Context,
};

#[derive(Copy, Clone)]
pub struct TxOutputState {
    pub value: RwSignal<u64>,
    pub script_pubkey: RwSignal<ScriptInputValue>,
    pub script_display_format: RwSignal<ScriptDisplayFormat>,
    pub token_data_state: TokenDataState,
    pub key: usize,
}

impl TxOutputState {
    pub fn new(key: usize) -> Self {
        Self {
            value: RwSignal::new(0),
            script_pubkey: RwSignal::default(),
            script_display_format: RwSignal::new(ScriptDisplayFormat::Addr),
            token_data_state: TokenDataState::new(key),
            key,
        }
    }

    pub fn dispose(self) {
        self.value.dispose();
        self.script_pubkey.dispose();
        self.script_display_format.dispose();
        self.token_data_state.dispose();
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
pub fn TxOutput(tx_output: TxOutputState, ctx: Context) -> impl IntoView {
    let script_pubkey = tx_output.script_pubkey;
    let script_format = tx_output.script_display_format;
    let cashtoken_enabled = tx_output.token_data_state.cashtoken_enabled;

    let parsed_input_val_id = format!("tx-output-val-{}", tx_output.key);

    view! {
        // Address
        <div class="mb-1 flex">
            <ScriptInput
                rows=1
                value=script_pubkey
                format=script_format
                network=ctx.network
                placeholder=move || {
                    match script_format() {
                        ScriptDisplayFormat::Addr => "Address",
                        ScriptDisplayFormat::Hex => "Locking Script Hex",
                        ScriptDisplayFormat::Asm => "Locking Script Asm",
                    }
                }
            />
            <div>
                <select
                    class="bg-inherit border rounded ml-1 p-1"
                    on:input=move |e| {
                        script_format.set(ScriptDisplayFormat::from_str(&event_target_value(&e)).unwrap())
                    }
                    prop:value={move || script_format().to_str()}
                >
                    <option value={ScriptDisplayFormat::Addr.to_str()}>Address</option>
                    <option value={ScriptDisplayFormat::Asm.to_str()}>Asm</option>
                    <option value={ScriptDisplayFormat::Hex.to_str()}>Hex</option>
                </select>
            </div>
        </div>

        // Amount
        <div class="my-1">
            <label class="mr-1" for=parsed_input_val_id.clone()>Sats:</label>
            <ParsedInput id=parsed_input_val_id value=tx_output.value placeholder="Sats" class="w-52"/>
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
