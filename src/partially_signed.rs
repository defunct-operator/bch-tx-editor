use bitcoincash::{blockdata::{opcodes::all::OP_SPECIAL_TOKEN_PREFIX, script::{self, Instruction}, token::{unwrap_scriptpubkey, OutputData}}, consensus::{Decodable, Encodable, encode}, OutPoint, Script, Sequence, VarInt};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct UnsignedScriptSig(Script);

impl Encodable for UnsignedScriptSig {
    #[inline]
    fn consensus_encode<W: std::io::Write + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        self.0.consensus_encode(w)
    }
}

impl Decodable for UnsignedScriptSig {
    #[inline]
    fn consensus_decode_from_finite_reader<R: std::io::Read + ?Sized>(reader: &mut R) -> Result<Self, bitcoincash::consensus::encode::Error> {
        Ok(Self(Script::consensus_decode_from_finite_reader(reader)?))
    }

    #[inline]
    fn consensus_decode<R: std::io::Read + ?Sized>(reader: &mut R) -> Result<Self, bitcoincash::consensus::encode::Error> {
        Ok(Self(Script::consensus_decode(reader)?))
    }
}

impl UnsignedScriptSig {
    /// 0xFD: unknown pubkey, but we know the Bitcoin address.
    pub fn from_script_pubkey(script_pubkey: Script) -> Self {
        let mut prefixed_pubkey = vec![0xfd];
        prefixed_pubkey.extend_from_slice(script_pubkey.as_bytes());
        Self(
            script::Builder::new()
                .push_slice(&[0xff])
                .push_slice(&prefixed_pubkey)
                .into_script(),
        )
    }

    /// Get the inner script pubkey.
    ///
    /// Returns `None` if the pubkey is not prefixed with 0xFD or if parsing fails for any reason.
    pub fn script_pubkey(&self) -> Option<Script> {
        let mut iter = self.0.instructions();
        let Instruction::PushBytes(&[0xff]) = iter.next()?.ok()? else { return None; };
        let Instruction::PushBytes(&[0xfd, ref spk @ ..]) = iter.next()?.ok()? else { return None; };
        Some(spk.to_vec().into())
    }
}

/// Unsigned transaction input. Compatible with Electron Cash.
///
/// This only implements the 0xFD public key, so it only contains the script pubkey of the previous
/// transaction's output.
///
/// * [Electrum documentation](https://electrum.readthedocs.io/en/latest/transactions.html)
/// * [Electron Cash source 1](https://github.com/Electron-Cash/Electron-Cash/blob/8e966d3c53fc1c394054a273ca2dc2be578b0abf/electroncash/keystore.py#L698)
/// * [Electron Cash source 2](https://github.com/Electron-Cash/Electron-Cash/blob/8e966d3c53fc1c394054a273ca2dc2be578b0abf/electroncash/transaction.py#L228)
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct TxIn {
    /// The reference to the previous output that is being used an an input.
    pub previous_output: OutPoint,
    /// The script that will get serialized as the scriptSig.
    pub unsigned_script_sig: UnsignedScriptSig,
    /// The sequence number, which suggests to miners which of two
    /// conflicting transactions should be preferred, or 0xFFFFFFFF
    /// to ignore this feature. This is generally never used since
    /// the miner behaviour cannot be enforced.
    pub sequence: Sequence,
    /// The value of the previous output, in satoshis.
    pub value: u64,
    /// Token output, optional. None if no token in this output.
    pub token: Option<OutputData>,
}

impl TxIn {
}

impl Encodable for TxIn {
    fn consensus_encode<W: std::io::Write + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        let mut len = 0;
        len += self.previous_output.consensus_encode(w)?;
        len += self.unsigned_script_sig.consensus_encode(w)?;
        len += self.sequence.consensus_encode(w)?;

        match &self.token {
            None => len += self.value.consensus_encode(w)?,
            Some(token) => {
                len += 0xffff_ffff_ffff_ffff_u64.consensus_encode(w)?;
                len += VarInt(self.value).consensus_encode(w)?;
                len += VarInt(1 + token.consensus_encode(&mut std::io::empty())? as u64).consensus_encode(w)?;
                len += OP_SPECIAL_TOKEN_PREFIX.to_u8().consensus_encode(w)?;
                len += token.consensus_encode(w)?;
            }
        }
        Ok(len)
    }
}

impl Decodable for TxIn {
    fn consensus_decode_from_finite_reader<R: std::io::Read + ?Sized>(r: &mut R) -> Result<Self, encode::Error> {
        let previous_output = OutPoint::consensus_decode_from_finite_reader(r)?;
        let unsigned_script_sig = UnsignedScriptSig::consensus_decode_from_finite_reader(r)?;
        let sequence = Sequence::consensus_decode_from_finite_reader(r)?;
        let mut value = u64::consensus_decode_from_finite_reader(r)?;
        let mut token = None;
        if value >= 0xffff_ffff_ffff_fff0 {
            let ext_version = value & 0xf;
            if ext_version != 0xf {
                return Err(encode::Error::ParseFailed("Unknown extension version"));
            }
            value = VarInt::consensus_decode_from_finite_reader(r)?.0;
            let wrapped_script_pubkey = Vec::<u8>::consensus_decode_from_finite_reader(r)?;
            if wrapped_script_pubkey.get(0) != Some(&OP_SPECIAL_TOKEN_PREFIX.to_u8()) {
                return Err(encode::Error::ParseFailed("Expected serialized token data"));
            }
            let mut wrapped_script_pubkey_slice = &*wrapped_script_pubkey;
            let token_data = OutputData::consensus_decode_from_finite_reader(&mut wrapped_script_pubkey_slice)?;
            if !wrapped_script_pubkey_slice.is_empty() {
                return Err(encode::Error::ParseFailed("Extra data after serialized token data"));
            }
            token = Some(token_data);
        }
        Ok(Self {
            previous_output,
            unsigned_script_sig,
            sequence,
            value,
            token,
        })
    }
}
