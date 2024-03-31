use anyhow::Result;
use bitcoincash::{hashes::hex::ToHex, Network, Script, TxOut};
use leptos::{
    component, event_target_checked, event_target_value, view, IntoView, RwSignal, SignalDispose,
    SignalGet, SignalSet,
};

use super::token_data::TokenDataState;
use crate::{
    components::{token_data::TokenData, ParsedInput},
    util::{cash_addr_to_script, script_to_cash_addr},
};

#[derive(Clone)]
pub enum ScriptPubkeyData {
    Hex(String),
    Addr(String),
}

impl ScriptPubkeyData {
    pub fn empty_or_hex(&self) -> bool {
        match self {
            Self::Hex(_) => true,
            Self::Addr(s) if s.is_empty() => true,
            _ => false,
        }
    }

    pub fn empty_or_addr(&self) -> bool {
        match self {
            Self::Addr(_) => true,
            Self::Hex(s) if s.is_empty() => true,
            _ => false,
        }
    }

    pub fn inner(self) -> String {
        match self {
            Self::Addr(s) | Self::Hex(s) => s,
        }
    }
}

impl TryFrom<ScriptPubkeyData> for Script {
    type Error = anyhow::Error;
    fn try_from(s: ScriptPubkeyData) -> Result<Self, Self::Error> {
        match s {
            ScriptPubkeyData::Hex(mut s) => {
                s.retain(|c| !c.is_ascii_whitespace());
                Ok(s.parse::<Script>()?)
            }
            ScriptPubkeyData::Addr(s) => cash_addr_to_script(&s),
        }
    }
}

str_enum! {
    #[derive(Copy, Clone)]
    pub enum ScriptDisplayFormat {
        Addr = "addr",
        Asm = "asm",
        Hex = "hex",
    }
}

#[derive(Copy, Clone)]
pub struct TxOutputState {
    pub value: RwSignal<u64>,
    pub script_pubkey: RwSignal<ScriptPubkeyData>,
    pub script_display_format: RwSignal<ScriptDisplayFormat>,
    pub token_data_state: TokenDataState,
    pub key: usize,
}

impl TxOutputState {
    pub fn new(key: usize) -> Self {
        Self {
            value: RwSignal::new(0),
            script_pubkey: RwSignal::new(ScriptPubkeyData::Hex("".into())),
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
pub fn TxOutput(tx_output: TxOutputState) -> impl IntoView {
    let script_pubkey = tx_output.script_pubkey;
    let script_format = tx_output.script_display_format;
    let cashtoken_enabled = tx_output.token_data_state.cashtoken_enabled;

    let script_pubkey_enabled = RwSignal::new(true);
    let script_pubkey_error = RwSignal::new(false);

    let parsed_input_val_id = format!("tx-output-val-{}", tx_output.key);

    let render_script_pubkey = move || {
        let script_pubkey = script_pubkey();
        match script_format() {
            ScriptDisplayFormat::Hex => {
                if script_pubkey.empty_or_hex() {
                    script_pubkey_enabled.set(true);
                    script_pubkey_error.set(false);
                    return script_pubkey.inner();
                }
                match Script::try_from(script_pubkey) {
                    Ok(s) => {
                        script_pubkey_enabled.set(true);
                        script_pubkey_error.set(false);
                        s.to_hex()
                    }
                    Err(e) => {
                        script_pubkey_enabled.set(false);
                        script_pubkey_error.set(true);
                        e.to_string()
                    }
                }
            }
            ScriptDisplayFormat::Asm => {
                script_pubkey_enabled.set(false);
                let script: Result<Script> = script_pubkey.try_into();
                match script {
                    Ok(s) => {
                        script_pubkey_error.set(false);
                        s.asm()
                    }
                    Err(e) => {
                        script_pubkey_error.set(true);
                        e.to_string()
                    }
                }
            }
            ScriptDisplayFormat::Addr => {
                if script_pubkey.empty_or_addr() {
                    script_pubkey_error.set(false);
                    script_pubkey_enabled.set(true);
                    return script_pubkey.inner();
                }
                let script: Script = match script_pubkey.try_into() {
                    Ok(s) => s,
                    Err(e) => {
                        script_pubkey_error.set(true);
                        script_pubkey_enabled.set(false);
                        return e.to_string();
                    }
                };
                match script_to_cash_addr(&script, Network::Bitcoin) {
                    Ok(a) => {
                        script_pubkey_enabled.set(true);
                        script_pubkey_error.set(false);
                        a
                    }
                    Err(e) => {
                        script_pubkey_enabled.set(false);
                        script_pubkey_error.set(true);
                        e.to_string()
                    }
                }
            }
        }
    };

    view! {
        // Address
        <div class="mb-1 flex">
            <textarea
                spellcheck="false"
                rows=1
                on:change=move |e| {
                    match script_format() {
                        ScriptDisplayFormat::Hex => {
                            script_pubkey.set(ScriptPubkeyData::Hex(event_target_value(&e)));
                        }
                        ScriptDisplayFormat::Addr => {
                            script_pubkey.set(ScriptPubkeyData::Addr(event_target_value(&e)));
                        }
                        _ => unreachable!(),
                    }
                }
                class="border border-solid rounded border-stone-600 px-1 w-full bg-inherit placeholder:text-stone-600 font-mono grow bg-stone-900"
                placeholder=move || {
                    match script_format() {
                        ScriptDisplayFormat::Addr => "Address",
                        ScriptDisplayFormat::Hex | ScriptDisplayFormat::Asm => "Locking Script Hex",
                    }
                }
                prop:value=render_script_pubkey
                disabled=move || !script_pubkey_enabled()
                class=("text-red-700", script_pubkey_error)
            />
            <div>
                <select
                    class="bg-inherit border rounded ml-1 p-1"
                    on:input=move |e| {
                        script_format.set(ScriptDisplayFormat::from_str(&event_target_value(&e)).unwrap())
                    }
                    prop:value={move || script_format().to_str()}
                >
                    <option value={|| ScriptDisplayFormat::Addr.to_str()}>Address</option>
                    <option value={|| ScriptDisplayFormat::Asm.to_str()}>Asm</option>
                    <option value={|| ScriptDisplayFormat::Hex.to_str()}>Hex</option>
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
