use bitcoincash::{
    blockdata::{opcodes, script::Builder},
    Address, Network, Script,
};
use cashaddr::CashEnc;

pub fn is_p2sh32(s: &Script) -> bool {
    let s = s.as_bytes();
    s.len() == 35
        && s[0] == opcodes::all::OP_HASH256.to_u8()
        && s[1] == opcodes::all::OP_PUSHBYTES_32.to_u8()
        && s[34] == opcodes::all::OP_EQUAL.to_u8()
}

pub fn cash_addr_to_script(addr: &str) -> anyhow::Result<Script> {
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

pub fn script_to_cash_addr(s: &Script, network: Network) -> anyhow::Result<String> {
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
