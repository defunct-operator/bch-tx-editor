use anyhow::Result;
use bitcoincash::{
    blockdata::{opcodes, script::Builder},
    hashes::hex::ToHex,
    Address, Network, Script, TxOut,
};
use cashaddr::CashEnc;
use leptos::{
    component, create_rw_signal, create_signal, event_target_checked, event_target_value, view,
    IntoView, RwSignal, SignalDispose, SignalGet, SignalSet,
};

fn cash_addr_to_script(addr: &str) -> Result<Script> {
    match addr.parse::<cashaddr::Payload>() {
        Ok(addr) => match addr.hash_type().numeric_value() {
            0 | 2 => {
                // p2pkh, token-aware p2pkh
                Ok(Builder::new()
                    .push_opcode(opcodes::all::OP_DUP)
                    .push_opcode(opcodes::all::OP_HASH160)
                    .push_slice(&addr)
                    .push_opcode(opcodes::all::OP_EQUALVERIFY)
                    .push_opcode(opcodes::all::OP_CHECKSIG)
                    .into_script())
            }
            1 | 3 => match addr.len() {
                // p2sh, token-aware p2sh
                20 => Ok(Builder::new()
                    .push_opcode(opcodes::all::OP_HASH160)
                    .push_slice(&addr)
                    .push_opcode(opcodes::all::OP_EQUAL)
                    .into_script()),
                32 => Ok(Builder::new()
                    .push_opcode(opcodes::all::OP_HASH256)
                    .push_slice(&addr)
                    .push_opcode(opcodes::all::OP_EQUAL)
                    .into_script()),
                _ => anyhow::bail!("unknown CashAddress type"),
            },
            _ => anyhow::bail!("unknown CashAddress type"),
        },
        Err(e) => {
            let Ok(addr) = addr.parse::<Address>() else {
                Err(e)?
            };
            Ok(addr.script_pubkey())
        }
    }
}

fn is_p2sh32(s: &Script) -> bool {
    let s = s.as_bytes();
    s.len() == 35
        && s[0] == opcodes::all::OP_HASH256.to_u8()
        && s[1] == opcodes::all::OP_PUSHBYTES_32.to_u8()
        && s[34] == opcodes::all::OP_EQUAL.to_u8()
}

fn script_to_cash_addr(s: &Script, network: Network) -> Result<String> {
    let prefix = match network {
        Network::Bitcoin => "bitcoincash",
        Network::Regtest => "bchreg",
        Network::Testnet | Network::Testnet4 | Network::Scalenet | Network::Chipnet => "bchtest",
    };
    if is_p2sh32(s) {
        let hash = &s.as_bytes()[2..34];
        Ok(hash.encode_p2sh(prefix)?)
    } else if s.is_p2sh() {
        let hash = &s.as_bytes()[2..22];
        Ok(hash.encode_p2sh(prefix)?)
    } else if s.is_p2pkh() {
        let hash = &s.as_bytes()[3..23];
        Ok(hash.encode_p2pkh(prefix)?)
    } else {
        anyhow::bail!("Unknown script type");
    }
}

use super::token_data::TokenDataState;
use crate::components::{token_data::TokenData, ParsedInput};

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

#[derive(Copy, Clone)]
pub enum ScriptDisplayFormat {
    Addr,
    Asm,
    Hex,
}

impl ScriptDisplayFormat {
    pub fn to_str(self) -> &'static str {
        match self {
            Self::Addr => "addr",
            Self::Asm => "asm",
            Self::Hex => "hex",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "addr" => Some(Self::Addr),
            "asm" => Some(Self::Asm),
            "hex" => Some(Self::Hex),
            _ => None,
        }
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
            value: create_rw_signal(0),
            script_pubkey: create_rw_signal(ScriptPubkeyData::Hex("".into())),
            script_display_format: create_rw_signal(ScriptDisplayFormat::Addr),
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
    let (script_pubkey, set_script_pubkey) = tx_output.script_pubkey.split();
    let (script_format, set_script_format) = tx_output.script_display_format.split();
    let cashtoken_enabled = tx_output.token_data_state.cashtoken_enabled;

    let (script_pubkey_enabled, set_script_pubkey_enabled) = create_signal(true);
    let (script_pubkey_error, set_script_pubkey_error) = create_signal(false);

    let parsed_input_val_id = format!("tx-output-val-{}", tx_output.key);

    let render_script_pubkey = move || {
        let script_pubkey = script_pubkey();
        match script_format() {
            ScriptDisplayFormat::Hex => {
                if script_pubkey.empty_or_hex() {
                    set_script_pubkey_enabled(true);
                    set_script_pubkey_error(false);
                    return script_pubkey.inner();
                }
                match Script::try_from(script_pubkey) {
                    Ok(s) => {
                        set_script_pubkey_enabled(true);
                        set_script_pubkey_error(false);
                        s.to_hex()
                    }
                    Err(e) => {
                        set_script_pubkey_enabled(false);
                        set_script_pubkey_error(true);
                        e.to_string()
                    }
                }
            }
            ScriptDisplayFormat::Asm => {
                set_script_pubkey_enabled(false);
                let script: Result<Script> = script_pubkey.try_into();
                match script {
                    Ok(s) => {
                        set_script_pubkey_error(false);
                        s.asm()
                    }
                    Err(e) => {
                        set_script_pubkey_error(true);
                        e.to_string()
                    }
                }
            }
            ScriptDisplayFormat::Addr => {
                if script_pubkey.empty_or_addr() {
                    set_script_pubkey_error(false);
                    set_script_pubkey_enabled(true);
                    return script_pubkey.inner();
                }
                let script: Script = match script_pubkey.try_into() {
                    Ok(s) => s,
                    Err(e) => {
                        set_script_pubkey_error(true);
                        set_script_pubkey_enabled(false);
                        return e.to_string();
                    }
                };
                match script_to_cash_addr(&script, Network::Bitcoin) {
                    Ok(a) => {
                        set_script_pubkey_enabled(true);
                        set_script_pubkey_error(false);
                        a
                    }
                    Err(e) => {
                        set_script_pubkey_enabled(false);
                        set_script_pubkey_error(true);
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
                            set_script_pubkey(ScriptPubkeyData::Hex(event_target_value(&e)));
                        }
                        ScriptDisplayFormat::Addr => {
                            set_script_pubkey(ScriptPubkeyData::Addr(event_target_value(&e)));
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
                        set_script_format(ScriptDisplayFormat::from_str(&event_target_value(&e)).unwrap())
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
