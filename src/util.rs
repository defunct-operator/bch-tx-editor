use bitcoincash::{
    Address, CashAddress, Network, Script, ScriptBuf, address::NetworkUnchecked, blockdata::opcodes,
};

pub fn is_p2sh32(s: &Script) -> bool {
    let s = s.as_bytes();
    s.len() == 35
        && s[0] == opcodes::all::OP_HASH256.to_u8()
        && s[1] == opcodes::all::OP_PUSHBYTES_32.to_u8()
        && s[34] == opcodes::all::OP_EQUAL.to_u8()
}

pub fn addr_to_script(addr: &str) -> anyhow::Result<ScriptBuf> {
    match addr.parse::<CashAddress<_>>() {
        Ok(addr) => Ok(addr.assume_checked().script_pubkey()),
        Err(e) => {
            let Ok(addr) = addr.parse::<Address<NetworkUnchecked>>() else {
                Err(e)?
            };
            Ok(addr.assume_checked().script_pubkey())
        }
    }
}

pub fn script_to_cash_addr(s: &ScriptBuf, network: Network) -> anyhow::Result<String> {
    if let Ok(a) = CashAddress::from_script(s, network) {
        Ok(a.to_string())
    } else {
        anyhow::bail!("Unknown script type");
    }
}
