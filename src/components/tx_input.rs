use bitcoincash::hashes::hex::FromHex;
use bitcoincash::{hashes, OutPoint, Script, Sequence, TxIn};
use leptos::{component, create_signal, event_target_checked, event_target_value, view, IntoView, RwSignal, SignalDispose, SignalGet, SignalSet};

use crate::components::{token_data::TokenData, ParsedInput};

use super::token_data::TokenDataState;
use anyhow::Result;


#[derive(Copy, Clone)]
pub struct TxInputState {
    pub txid: RwSignal<String>,
    pub vout: RwSignal<u32>,
    pub sequence: RwSignal<u32>,
    pub script_sig: RwSignal<String>,
    pub unsigned: RwSignal<bool>,
    pub token_data_state: TokenDataState,
    pub key: usize,
}

impl TxInputState {
    pub fn new(key: usize) -> Self {
        Self {
            txid: RwSignal::new("".into()),
            vout: RwSignal::new(0),
            sequence: RwSignal::new(4294967295),
            script_sig: RwSignal::new("".into()),
            unsigned: RwSignal::new(false),
            token_data_state: TokenDataState::new(key),
            key,
        }
    }

    pub fn dispose(self) {
        let Self { txid, vout, sequence, script_sig, unsigned, token_data_state, key: _ } = self;
        txid.dispose();
        vout.dispose();
        sequence.dispose();
        script_sig.dispose();
        unsigned.dispose();
        token_data_state.dispose();
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
pub fn TxInput(tx_input: TxInputState) -> impl IntoView {
    let (txid, set_txid) = tx_input.txid.split();
    let (script_sig, set_script_sig) = tx_input.script_sig.split();
    let (script_format, set_script_format) = create_signal(String::from("hex"));
    let (script_enabled, set_script_enabled) = create_signal(true);
    let (script_error, set_script_error) = create_signal(false);
    let cashtoken_enabled = tx_input.token_data_state.cashtoken_enabled;
    let unsigned = tx_input.unsigned;

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
                disabled=move || !script_enabled() || unsigned()
                class=("text-red-700", script_error)
                class=("opacity-30", unsigned)
            />
            <div>
                <select
                    class="bg-inherit border rounded ml-1 p-1 disabled:opacity-30"
                    on:input=move |e| set_script_format(event_target_value(&e))
                    prop:value={script_format}
                    disabled=unsigned
                >
                    <option value="hex">Hex</option>
                    <option value="asm">Asm</option>
                </select>
            </div>
        </div>
        <div class="my-1">
            <label class="mr-1" for=parsed_input_seq_id.clone()>Sequence Number:</label>
            <ParsedInput id=parsed_input_seq_id value=tx_input.sequence placeholder="Sequence"/>
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
            <label class=("opacity-30", move || !unsigned())>
                <input
                    type="checkbox"
                    class="ml-5"
                    on:change=move |e| cashtoken_enabled.set(event_target_checked(&e))
                    prop:checked=cashtoken_enabled
                    disabled=move || !unsigned()
                />
                CashToken
            </label>
        </div>

        <TokenData token_data=tx_input.token_data_state />
    }
}
