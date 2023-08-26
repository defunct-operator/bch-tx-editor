use bitcoincash::{hashes::hex::ToHex, Address, Network, Script, TxOut};
use leptos::{
    component, create_rw_signal, create_signal, event_target_value, view, IntoView, RwSignal,
    SignalGet,
};

use crate::components::ParsedInput;

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
    type Error = String;
    fn try_from(s: ScriptPubkeyData) -> Result<Self, Self::Error> {
        match s {
            ScriptPubkeyData::Hex(s) => s.parse::<Script>().map_err(|e| e.to_string()),
            ScriptPubkeyData::Addr(s) => s
                .parse::<Address>()
                .map(|a| a.script_pubkey())
                .map_err(|e| e.to_string()),
        }
    }
}

#[derive(Copy, Clone)]
pub struct TxOutputState {
    pub value: RwSignal<u64>,
    pub script_pubkey: RwSignal<ScriptPubkeyData>,
    pub key: usize,
}

impl TxOutputState {
    pub fn new(key: usize) -> Self {
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
pub fn TxOutput(tx_output: TxOutputState) -> impl IntoView {
    let (script_pubkey, set_script_pubkey) = tx_output.script_pubkey.split();
    let (script_format, set_script_format) = create_signal(String::from("hex"));
    let (script_pubkey_enabled, set_script_pubkey_enabled) = create_signal(true);
    let (script_pubkey_error, set_script_pubkey_error) = create_signal(false);

    let render_script_pubkey = move || {
        let script_pubkey = script_pubkey();
        match &*script_format() {
            "hex" => {
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
                        e
                    }
                }
            }
            "asm" => {
                set_script_pubkey_enabled(false);
                let script: Result<Script, String> = script_pubkey.try_into();
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
                if script_pubkey.empty_or_addr() {
                    set_script_pubkey_error(false);
                    set_script_pubkey_enabled(true);
                    return script_pubkey.inner();
                }
                let script = match script_pubkey.try_into() {
                    Ok(s) => s,
                    Err(e) => {
                        set_script_pubkey_error(true);
                        set_script_pubkey_enabled(false);
                        return e;
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
