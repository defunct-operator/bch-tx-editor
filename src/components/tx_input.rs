use anyhow::Result;
use bitcoincash::hashes::hex::ToHex;
use bitcoincash::secp256k1::{Secp256k1, Verification};
use bitcoincash::{OutPoint, Script, Sequence, TxIn};
use leptos::prelude::{
    AddAnyAttr, ArcRwSignal, ClassAttribute, ElementChild, Get, GlobalAttributes, OnAttribute,
    PropAttribute, ReadValue, RwSignal, Set, Show, StoredValue, Write, event_target_checked,
    event_target_value,
};
use leptos::reactive::signal::ReadSignal;
use leptos::{IntoView, component, view};

use super::script_input::ScriptInputValue;
use crate::Context;
use crate::components::drag_handle::DragHandle;
use crate::components::script_input::{ScriptDisplayFormat, ScriptInput};
use crate::components::{
    ParsedInput,
    token_data::{TokenData, TokenDataState},
};
use crate::js_reexport::bin_to_cash_assembly;
use crate::macros::StrEnum;
use crate::partially_signed::{MaybeUnsignedTxIn, UnsignedScriptSig, UnsignedTxIn};
use crate::spv::{SpvConnStatus, use_spv};
use crate::util::{cash_addr_to_script, script_to_cash_addr};

str_enum! {
    #[derive(Copy, Clone, Default)]
    pub enum PubkeyDisplayFormat {
        #[default]
        Addr = "addr",
        Asm = "asm",
        Hex = "hex",
    }
}

#[derive(Clone)]
pub enum UtxoPubkeyData {
    Hex(String),
    Addr(String),
}

impl Default for UtxoPubkeyData {
    fn default() -> Self {
        Self::Hex(String::new())
    }
}

impl UtxoPubkeyData {
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

impl TryFrom<UtxoPubkeyData> for UnsignedScriptSig {
    type Error = anyhow::Error;
    fn try_from(s: UtxoPubkeyData) -> Result<Self, Self::Error> {
        match s {
            UtxoPubkeyData::Hex(mut s) => {
                s.retain(|c| !c.is_ascii_whitespace());
                Ok(UnsignedScriptSig::from_raw_script(s.parse::<Script>()?))
            }
            UtxoPubkeyData::Addr(s) => Ok(UnsignedScriptSig::from_script_pubkey(
                cash_addr_to_script(&s)?,
            )),
        }
    }
}

#[derive(Clone)]
pub struct TxInputState {
    pub txid: ArcRwSignal<String>,
    pub vout: ArcRwSignal<u32>,
    pub sequence: ArcRwSignal<u32>,
    pub script_sig: ArcRwSignal<ScriptInputValue>,
    pub script_sig_format: ArcRwSignal<ScriptDisplayFormat>,
    pub unsigned: ArcRwSignal<bool>,
    /// The raw data that Electron Cash shoves into the scriptSig section in hex, typically
    /// the extended public key.
    pub utxo_pubkey: ArcRwSignal<UtxoPubkeyData>,
    pub utxo_amount: ArcRwSignal<u64>,
    pub token_data_state: TokenDataState,
    pub key: usize,
}

impl TxInputState {
    pub fn new(key: usize) -> Self {
        Self {
            txid: ArcRwSignal::default(),
            vout: ArcRwSignal::new(0),
            sequence: ArcRwSignal::new(4294967294),
            script_sig: ArcRwSignal::default(),
            script_sig_format: ArcRwSignal::new(ScriptDisplayFormat::Asm),
            unsigned: ArcRwSignal::new(false),
            utxo_pubkey: ArcRwSignal::default(),
            utxo_amount: ArcRwSignal::new(0),
            token_data_state: TokenDataState::new(key),
            key,
        }
    }

    pub fn update_from_txin(&self, input: &MaybeUnsignedTxIn) {
        self.txid.set(input.previous_output().txid.to_string());
        self.vout.set(input.previous_output().vout);
        self.sequence.set(input.sequence().0);

        match input {
            MaybeUnsignedTxIn::Signed(txin) => {
                self.script_sig
                    .set(ScriptInputValue::Hex(txin.script_sig.to_hex()));
                self.script_sig_format.set(ScriptDisplayFormat::Asm);
                self.unsigned.set(false);
                self.utxo_pubkey.set(Default::default());
                self.utxo_amount.set(0);
                self.token_data_state.update_from_token_data(None);
            }
            MaybeUnsignedTxIn::Unsigned(txin) => {
                self.script_sig.write().clear();
                self.script_sig_format.set(ScriptDisplayFormat::Hex);
                self.unsigned.set(true);
                self.utxo_pubkey.set(UtxoPubkeyData::Hex(
                    txin.unsigned_script_sig.raw_script().to_hex(),
                ));
                self.utxo_amount.set(txin.value);
                self.token_data_state
                    .update_from_token_data(txin.token.as_ref());
            }
        }
    }
}

impl TryFrom<TxInputState> for TxIn {
    type Error = anyhow::Error;
    fn try_from(tx_input: TxInputState) -> Result<Self, Self::Error> {
        Ok(TxIn {
            previous_output: OutPoint {
                txid: tx_input.txid.get().parse()?,
                vout: tx_input.vout.get(),
            },
            script_sig: tx_input.script_sig.get().try_into()?,
            sequence: Sequence(tx_input.sequence.get()),
            witness: Default::default(),
        })
    }
}

impl TryFrom<TxInputState> for UnsignedTxIn {
    type Error = anyhow::Error;
    fn try_from(tx_input: TxInputState) -> Result<Self, Self::Error> {
        Ok(UnsignedTxIn {
            previous_output: OutPoint {
                txid: tx_input.txid.get().parse()?,
                vout: tx_input.vout.get(),
            },
            sequence: Sequence(tx_input.sequence.get()),
            unsigned_script_sig: tx_input.utxo_pubkey.get().try_into()?,
            value: tx_input.utxo_amount.get(),
            token: tx_input.token_data_state.token_data()?,
        })
    }
}

impl TryFrom<TxInputState> for MaybeUnsignedTxIn {
    type Error = anyhow::Error;
    fn try_from(tx_input: TxInputState) -> Result<Self, Self::Error> {
        if tx_input.unsigned.get() {
            Ok(MaybeUnsignedTxIn::Unsigned(tx_input.try_into()?))
        } else {
            Ok(MaybeUnsignedTxIn::Signed(tx_input.try_into()?))
        }
    }
}

#[component]
pub fn TxInput<C: Verification + 'static>(
    tx_input: TxInputState,
    secp: StoredValue<Secp256k1<C>>,
    ctx: Context,
    set_draggable: impl Fn(bool) + 'static,
) -> impl IntoView {
    let txid = RwSignal::from(tx_input.txid);
    let script_sig = RwSignal::from(tx_input.script_sig);
    let script_sig_format = RwSignal::from(tx_input.script_sig_format);
    let cashtoken_enabled = RwSignal::from(tx_input.token_data_state.cashtoken_enabled.clone());
    let unsigned = RwSignal::from(tx_input.unsigned);
    let utxo_amount = RwSignal::from(tx_input.utxo_amount);
    let utxo_pubkey = RwSignal::from(tx_input.utxo_pubkey);

    let pubkey_format = RwSignal::new(PubkeyDisplayFormat::default());
    let utxo_pubkey_enabled = RwSignal::new(true);
    let utxo_pubkey_error = RwSignal::new(false);

    let parsed_input_seq_id = format!("tx-input-sn-{}", tx_input.key);
    let parsed_input_val_id = format!("tx-input-val-{}", tx_input.key);

    let render_utxo_pubkey = move || {
        let utxo_pubkey = utxo_pubkey();
        match pubkey_format() {
            PubkeyDisplayFormat::Hex => {
                if utxo_pubkey.empty_or_hex() {
                    utxo_pubkey_enabled.set(true);
                    utxo_pubkey_error.set(false);
                    return utxo_pubkey.inner();
                }
                match UnsignedScriptSig::try_from(utxo_pubkey) {
                    Ok(s) => {
                        utxo_pubkey_enabled.set(true);
                        utxo_pubkey_error.set(false);
                        s.to_hex()
                    }
                    Err(e) => {
                        utxo_pubkey_enabled.set(false);
                        utxo_pubkey_error.set(true);
                        e.to_string()
                    }
                }
            }
            PubkeyDisplayFormat::Asm => {
                utxo_pubkey_enabled.set(false);
                let script: Result<UnsignedScriptSig> = utxo_pubkey.try_into();
                match script {
                    Ok(s) => {
                        utxo_pubkey_error.set(false);
                        bin_to_cash_assembly(s.raw_script().as_bytes().into())
                    }
                    Err(e) => {
                        utxo_pubkey_error.set(true);
                        e.to_string()
                    }
                }
            }
            PubkeyDisplayFormat::Addr => {
                if utxo_pubkey.empty_or_addr() {
                    utxo_pubkey_error.set(false);
                    utxo_pubkey_enabled.set(true);
                    return utxo_pubkey.inner();
                }
                let script: UnsignedScriptSig = match utxo_pubkey.try_into() {
                    Ok(s) => s,
                    Err(e) => {
                        utxo_pubkey_error.set(true);
                        utxo_pubkey_enabled.set(false);
                        return e.to_string();
                    }
                };
                let Some(script) = script.script_pubkey(&secp.read_value()) else {
                    utxo_pubkey_enabled.set(false);
                    utxo_pubkey_error.set(true);
                    return "Unknown address".into();
                };
                match script_to_cash_addr(&script, ctx.network.get()) {
                    Ok(a) => {
                        utxo_pubkey_enabled.set(true);
                        utxo_pubkey_error.set(false);
                        a
                    }
                    Err(e) => {
                        utxo_pubkey_enabled.set(false);
                        utxo_pubkey_error.set(true);
                        e.to_string()
                    }
                }
            }
        }
    };

    let spv = use_spv();
    let spv_status = ReadSignal::from(spv.status);
    // let source_tx = move || {
    //     if spv_status() == SpvConnStatus::Connected {
    //         spv.tx_fetcher.get("d58a0b1ce6a263259e94f895d2cf8c066159a603161b5294b308b114511b88e7".parse().unwrap()).get()
    //     } else {
    //         None
    //     }
    // };
    view! {
        // <div class="text-xs">
        //     <Show
        //         when=move || spv_status() == SpvConnStatus::Connected
        //         fallback=|| "not connected"
        //     >
        //         // show is implicitly reactive???
        //         {
        //             if let Some(source_tx) = source_tx() {
        //                 format!("{:?}", source_tx)
        //             } else {
        //                 String::from("loading")
        //             }
        //         }
        //     </Show>
        // </div>
        <div class="mb-1 flex">
            <input
                on:change=move |e| txid.set(event_target_value(&e))
                class=concat!(
                    "border border-solid rounded border-stone-600 px-1 w-full bg-stone-900 ",
                    "placeholder:text-stone-600 font-mono grow",
                )
                prop:value=txid
                placeholder="Transaction ID"
            />
            <span>:</span>
            <ParsedInput value=tx_input.vout.into() {..} placeholder="Index" class=("w-14", true)/>
            <div class=("cursor-grab", true) on:mousedown=move |_| set_draggable(true) >
                <DragHandle />
            </div>
        </div>
        <div class="mb-1 flex">
            <ScriptInput
                value=script_sig
                format=script_sig_format
                network=ctx.network
                disabled=unsigned
                oneline=false
                attr:placeholder=move || {
                    match script_sig_format() {
                        ScriptDisplayFormat::Addr => "How did you make this happen?",
                        ScriptDisplayFormat::Hex => "Unlocking Script Hex",
                        ScriptDisplayFormat::Asm => "Unlocking Script Asm",
                        ScriptDisplayFormat::P2sh => "Unlocking Script Asm",
                    }
                }
                {..}
                class=("text-xs", true)
                class=("pt-px", true)
            />
            <div>
                <select
                    class="bg-stone-900 border border-stone-600 rounded ml-1 p-1 disabled:opacity-30"
                    on:input=move |e| {
                        script_sig_format.set(ScriptDisplayFormat::from_str(&event_target_value(&e)).unwrap())
                    }
                    prop:value={move || script_sig_format().to_str()}
                    disabled=unsigned
                >
                    <option value={ScriptDisplayFormat::Asm.to_str()} selected>Asm</option>
                    <option value={ScriptDisplayFormat::Hex.to_str()}>Hex</option>
                    <option value={ScriptDisplayFormat::P2sh.to_str()}>P2SH</option>
                </select>
            </div>
        </div>
        <div class="my-1">
            <label class="mr-1" for=parsed_input_seq_id.clone()>Sequence Number:</label>
            <ParsedInput value=tx_input.sequence.into() {..} id=parsed_input_seq_id placeholder="Sequence"/>
            <label>
                <input
                    type="checkbox"
                    class="ml-5"
                    on:change=move |e| {
                        let c = event_target_checked(&e);
                        unsigned.set(c);
                        if !c {
                            cashtoken_enabled.set(false);
                        }
                    }
                    prop:checked=unsigned
                />
                Unsigned
            </label>
        </div>

        <Show when=unsigned>
            // UTXO Address
            <div class="mt-3 mb-1 flex">
                <textarea
                    spellcheck="false"
                    rows=1
                    on:change=move |e| {
                        match pubkey_format() {
                            PubkeyDisplayFormat::Hex => {
                                utxo_pubkey.set(UtxoPubkeyData::Hex(event_target_value(&e)));
                            }
                            PubkeyDisplayFormat::Addr => {
                                utxo_pubkey.set(UtxoPubkeyData::Addr(event_target_value(&e)));
                            }
                            _ => unreachable!(),
                        }
                    }
                    class="border border-solid rounded border-stone-600 px-1 w-full placeholder:text-stone-600 font-mono grow bg-stone-900"
                    placeholder=move || {
                        match pubkey_format() {
                            PubkeyDisplayFormat::Addr => "Previous Address",
                            PubkeyDisplayFormat::Hex | PubkeyDisplayFormat::Asm => "Serialized Data",
                        }
                    }
                    prop:value=render_utxo_pubkey
                    disabled=move || !utxo_pubkey_enabled()
                    class=("text-red-700", utxo_pubkey_error)
                />
                <div>
                    <select
                        class="bg-stone-900 border border-stone-600 rounded ml-1 p-1"
                        on:input=move |e| {
                            pubkey_format.set(PubkeyDisplayFormat::from_str(&event_target_value(&e)).unwrap())
                        }
                        prop:value={move || pubkey_format().to_str()}
                    >
                        <option value=PubkeyDisplayFormat::Addr.to_str()>Address</option>
                        <option value=PubkeyDisplayFormat::Asm.to_str()>Asm</option>
                        <option value=PubkeyDisplayFormat::Hex.to_str()>Hex</option>
                    </select>
                </div>
            </div>

            // Amount
            <div class="my-1">
                <label class="mr-1" for=parsed_input_val_id.clone()>Sats:</label>
                <ParsedInput value={utxo_amount} {..} placeholder="Sats" class=("w-52", true) id=parsed_input_val_id.clone()/>
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
        </Show>

        <TokenData token_data=tx_input.token_data_state />
    }
}
