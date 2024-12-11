use bitcoincash::{hashes::hex::ToHex, Network, Script};
use leptos::{
    component,
    prelude::{
        event_target_value, ClassAttribute, Get, GlobalAttributes, MaybeProp, OnAttribute,
        PropAttribute, ReadSignal, RwSignal, Set,
    },
    view, IntoView,
};

use crate::{
    js_reexport::{bin_to_cash_assembly, cash_assembly_to_bin},
    util::{cash_addr_to_script, script_to_cash_addr},
};

#[derive(Clone)]
pub enum ScriptInputValue {
    Hex(String),
    Addr(String),
    Asm(String),
}

impl ScriptInputValue {
    pub fn is_empty(&self) -> bool {
        self.inner().is_empty()
    }

    pub fn format(&self) -> ScriptDisplayFormat {
        match self {
            ScriptInputValue::Hex(_) => ScriptDisplayFormat::Hex,
            ScriptInputValue::Addr(_) => ScriptDisplayFormat::Addr,
            ScriptInputValue::Asm(_) => ScriptDisplayFormat::Asm,
        }
    }

    pub fn inner(&self) -> &String {
        match self {
            Self::Addr(s) | Self::Hex(s) | Self::Asm(s) => s,
        }
    }

    pub fn inner_mut(&mut self) -> &mut String {
        match self {
            Self::Addr(s) | Self::Hex(s) | Self::Asm(s) => s,
        }
    }

    pub fn clear(&mut self) {
        self.inner_mut().clear()
    }
}

impl TryFrom<ScriptInputValue> for Script {
    type Error = anyhow::Error;
    fn try_from(s: ScriptInputValue) -> Result<Self, Self::Error> {
        match s {
            ScriptInputValue::Hex(mut s) => {
                s.retain(|c| !c.is_ascii_whitespace());
                Ok(s.parse::<Script>()?)
            }
            ScriptInputValue::Addr(s) => cash_addr_to_script(&s),
            ScriptInputValue::Asm(s) => Ok(Script::from(cash_assembly_to_bin(&s)?.into_vec())),
        }
    }
}

impl Default for ScriptInputValue {
    fn default() -> Self {
        Self::Hex(String::new())
    }
}

str_enum! {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub enum ScriptDisplayFormat {
        Addr = "addr",
        Asm = "asm",
        Hex = "hex",
    }
}

#[component]
pub fn ScriptInput(
    value: RwSignal<ScriptInputValue>,
    format: RwSignal<ScriptDisplayFormat>,
    network: ReadSignal<Network>,
    #[prop(into, default=Default::default())] disabled: MaybeProp<bool>,
) -> impl IntoView {
    let error = RwSignal::new(false);
    let disabled = move || disabled().unwrap_or(false);

    let render_value = move || {
        let value = value();
        let format = format();
        if value.format() == format || value.is_empty() {
            error.set(false);
            return value.inner().into();
        }
        match format {
            ScriptDisplayFormat::Hex => match Script::try_from(value) {
                Ok(s) => {
                    error.set(false);
                    s.to_hex()
                }
                Err(e) => {
                    error.set(true);
                    e.to_string()
                }
            },
            ScriptDisplayFormat::Asm => match Script::try_from(value) {
                Ok(s) => {
                    error.set(false);
                    bin_to_cash_assembly(s.as_bytes().into())
                }
                Err(e) => {
                    error.set(true);
                    e.to_string()
                }
            },
            ScriptDisplayFormat::Addr => {
                let script: Script = match value.try_into() {
                    Ok(s) => s,
                    Err(e) => {
                        error.set(true);
                        return e.to_string();
                    }
                };
                match script_to_cash_addr(&script, network.get()) {
                    Ok(a) => {
                        error.set(false);
                        a
                    }
                    Err(e) => {
                        error.set(true);
                        e.to_string()
                    }
                }
            }
        }
    };

    view! {
        <textarea
            spellcheck="false"
            on:change=move |e| {
                match format() {
                    ScriptDisplayFormat::Hex => {
                        value.set(ScriptInputValue::Hex(event_target_value(&e)));
                    }
                    ScriptDisplayFormat::Addr => {
                        value.set(ScriptInputValue::Addr(event_target_value(&e)));
                    }
                    ScriptDisplayFormat::Asm => {
                        value.set(ScriptInputValue::Asm(event_target_value(&e)));
                    }
                }
            }
            class="border border-solid rounded border-stone-600 px-1 w-full bg-inherit placeholder:text-stone-600 font-mono grow bg-stone-900"
            prop:value=render_value
            disabled=move || error() || disabled()
            class=("text-red-700", error)
            class=("opacity-30", disabled)
        />
    }
}
