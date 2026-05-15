use std::fmt::Write as _;

use bitcoincash::{
    blockdata::{
        opcodes::{Class, ClassifyContext},
        script::Instruction,
    },
    hashes::hex::ToHex,
    Network, Script,
};
use leptos::{
    component,
    either::Either,
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

// copied from https://docs.rs/bitcoincash/0.29.2/src/bitcoincash/blockdata/script.rs.html#224-253
/// Helper to encode an integer in script format.
/// Writes bytes into the buffer and returns the number of bytes written.
fn write_scriptint(out: &mut [u8; 8], n: i64) -> usize {
    let mut len = 0;
    if n == 0 {
        return len;
    }

    let neg = n < 0;

    let mut abs = if neg { -n } else { n } as usize;
    while abs > 0xFF {
        out[len] = (abs & 0xFF) as u8;
        len += 1;
        abs >>= 8;
    }
    // If the number's value causes the sign bit to be set, we need an extra
    // byte to get the correct value and correct sign bit
    if abs & 0x80 != 0 {
        out[len] = abs as u8;
        len += 1;
        out[len] = if neg { 0x80u8 } else { 0u8 };
        len += 1;
    }
    // Otherwise we just set the sign bit ourselves
    else {
        abs |= if neg { 0x80 } else { 0 };
        out[len] = abs as u8;
        len += 1;
    }
    len
}

/// Puts each opcode on its a separate line, and adds indentation.
fn format_cashassembly(src: &str, base_level: usize) -> String {
    let mut r = String::new();
    let mut level = base_level;
    for word in src.split_ascii_whitespace() {
        if matches!(word, "OP_ELSE" | "OP_ENDIF" | "OP_UNTIL") {
            level -= 1;
        }
        _ = writeln!(r, "{:1$}{word}", "", level * 4);
        if matches!(word, "OP_IF" | "OP_ELSE" | "OP_BEGIN") {
            level += 1;
        }
    }
    r.pop();
    r
}

fn disassemble_p2sh_script_sig(s: Script) -> Result<String, String> {
    let mut r = String::new();
    let mut instructions = s.instructions_minimal().peekable();
    while let Some(ins) = instructions.next() {
        let is_last = instructions.peek().is_none();
        let data = match ins {
            Ok(Instruction::PushBytes([])) => Either::Right(0),
            Ok(Instruction::PushBytes(x)) => Either::Left(x),
            Ok(Instruction::Op(op)) => {
                if let Class::PushNum(n) = op.classify(ClassifyContext::Legacy) {
                    Either::Right(n)
                } else {
                    return Err("encountered non-push opcode".into());
                }
            }
            Err(e) => return Err(e.to_string()),
        };
        if is_last {
            let mut buf = [0; 8];
            let d = match data {
                Either::Left(d) => d,
                Either::Right(n) => {
                    let b = write_scriptint(&mut buf, n.into());
                    &buf[..b]
                }
            };
            let redeem_script = bin_to_cash_assembly(d.into());
            let redeem_script = format_cashassembly(&redeem_script, 1);
            if !r.is_empty() {
                r.push('\n');
            }
            r += "<\n";
            r += &redeem_script;
            r += "\n>";
        } else {
            match data {
                Either::Left(d) => {
                    r.push_str("<0x");
                    r.push_str(&d.to_hex());
                }
                Either::Right(n) => {
                    _ = write!(r, "<{}", n);
                }
            }
            r.push_str(">\n");
        }
    }
    Ok(r)
}

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

    pub fn needs_conversion(&self, format: ScriptDisplayFormat) -> bool {
        use ScriptDisplayFormat as Sdf;
        use ScriptInputValue as Siv;
        if self.inner().is_empty() {
            return false;
        }
        !matches!(
            (self, format),
            (Siv::Hex(_), Sdf::Hex)
                | (Siv::Addr(_), Sdf::Addr)
                | (Siv::Asm(_), Sdf::Asm | Sdf::P2sh)
        )
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
        P2sh = "p2sh",
    }
}

#[component]
pub fn ScriptInput(
    value: RwSignal<ScriptInputValue>,
    format: RwSignal<ScriptDisplayFormat>,
    network: ReadSignal<Network>,
    #[prop(into, default=Default::default())] disabled: MaybeProp<bool>,
    oneline: bool,
) -> impl IntoView {
    let error = RwSignal::new(false);
    let disabled = move || disabled().unwrap_or(false);

    let render_value = move || {
        let value = value();
        let format = format();
        if !value.needs_conversion(format) {
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
                    let mut s = bin_to_cash_assembly(s.as_bytes().into());
                    if !oneline {
                        s = format_cashassembly(&s, 0);
                    }
                    s
                }
                Err(e) => {
                    error.set(true);
                    e.to_string()
                }
            },
            ScriptDisplayFormat::P2sh => match Script::try_from(value)
                .map_err(|e| e.to_string())
                .and_then(disassemble_p2sh_script_sig)
            {
                Ok(s) => {
                    error.set(false);
                    s
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
                    ScriptDisplayFormat::Asm | ScriptDisplayFormat::P2sh => {
                        value.set(ScriptInputValue::Asm(event_target_value(&e)));
                    }
                }
            }
            class="border border-solid rounded border-stone-600 px-1 w-full placeholder:text-stone-600 font-mono grow bg-stone-900"
            prop:value=render_value
            disabled=move || error() || disabled()
            class=("text-red-700", error)
            class=("opacity-30", disabled)
        />
    }
}
