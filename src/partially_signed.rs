use bitcoincash::{
    blockdata::{
        opcodes::all::OP_SPECIAL_TOKEN_PREFIX,
        script::{self, Instruction},
        token::OutputData,
    },
    consensus::{
        encode::{self, MAX_VEC_SIZE},
        Decodable, Encodable,
    },
    OutPoint, PackedLockTime, Script, Sequence, TxIn, TxOut, VarInt,
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct UnsignedScriptSig(Script);

impl Encodable for UnsignedScriptSig {
    #[inline]
    fn consensus_encode<W: std::io::Write + ?Sized>(
        &self,
        w: &mut W,
    ) -> Result<usize, std::io::Error> {
        self.0.consensus_encode(w)
    }
}

impl Decodable for UnsignedScriptSig {
    #[inline]
    fn consensus_decode_from_finite_reader<R: std::io::Read + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, bitcoincash::consensus::encode::Error> {
        Ok(Self(Script::consensus_decode_from_finite_reader(reader)?))
    }

    #[inline]
    fn consensus_decode<R: std::io::Read + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, bitcoincash::consensus::encode::Error> {
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
        let Instruction::PushBytes(&[0xff]) = iter.next()?.ok()? else {
            return None;
        };
        let Instruction::PushBytes(&[0xfd, ref spk @ ..]) = iter.next()?.ok()? else {
            return None;
        };
        Some(spk.to_vec().into())
    }
}

fn is_unsigned_script_sig(s: &Script) -> bool {
    matches!(
        s.instructions().next(),
        Some(Ok(Instruction::PushBytes(&[0xff])))
    )
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
pub struct UnsignedTxIn {
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

impl Encodable for UnsignedTxIn {
    fn consensus_encode<W: std::io::Write + ?Sized>(
        &self,
        w: &mut W,
    ) -> Result<usize, std::io::Error> {
        let mut len = 0;
        len += self.previous_output.consensus_encode(w)?;
        len += self.unsigned_script_sig.consensus_encode(w)?;
        len += self.sequence.consensus_encode(w)?;

        match &self.token {
            None => len += self.value.consensus_encode(w)?,
            Some(token) => {
                len += 0xffff_ffff_ffff_ffff_u64.consensus_encode(w)?;
                len += VarInt(self.value).consensus_encode(w)?;
                len += VarInt(1 + token.consensus_encode(&mut std::io::empty())? as u64)
                    .consensus_encode(w)?;
                len += OP_SPECIAL_TOKEN_PREFIX.to_u8().consensus_encode(w)?;
                len += token.consensus_encode(w)?;
            }
        }
        Ok(len)
    }
}

impl Decodable for UnsignedTxIn {
    fn consensus_decode_from_finite_reader<R: std::io::Read + ?Sized>(
        r: &mut R,
    ) -> Result<Self, encode::Error> {
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
            if wrapped_script_pubkey.first() != Some(&OP_SPECIAL_TOKEN_PREFIX.to_u8()) {
                return Err(encode::Error::ParseFailed("Expected serialized token data"));
            }
            let mut wrapped_script_pubkey_slice = &*wrapped_script_pubkey;
            let token_data =
                OutputData::consensus_decode_from_finite_reader(&mut wrapped_script_pubkey_slice)?;
            if !wrapped_script_pubkey_slice.is_empty() {
                return Err(encode::Error::ParseFailed(
                    "Extra data after serialized token data",
                ));
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

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum MaybeUnsignedTxIn {
    Unsigned(UnsignedTxIn),
    Signed(TxIn),
}

impl Encodable for MaybeUnsignedTxIn {
    fn consensus_encode<W: std::io::Write + ?Sized>(
        &self,
        w: &mut W,
    ) -> Result<usize, std::io::Error> {
        match self {
            MaybeUnsignedTxIn::Unsigned(s) => s.consensus_encode(w),
            MaybeUnsignedTxIn::Signed(s) => s.consensus_encode(w),
        }
    }
}

impl Decodable for MaybeUnsignedTxIn {
    fn consensus_decode_from_finite_reader<R: std::io::Read + ?Sized>(
        r: &mut R,
    ) -> Result<Self, encode::Error> {
        let previous_output = OutPoint::consensus_decode_from_finite_reader(r)?;
        let script_sig = Script::consensus_decode_from_finite_reader(r)?;
        let sequence = Sequence::consensus_decode_from_finite_reader(r)?;
        if is_unsigned_script_sig(&script_sig) {
            let mut value = u64::consensus_decode_from_finite_reader(r)?;
            let mut token = None;
            if value >= 0xffff_ffff_ffff_fff0 {
                let ext_version = value & 0xf;
                if ext_version != 0xf {
                    return Err(encode::Error::ParseFailed("Unknown extension version"));
                }
                value = VarInt::consensus_decode_from_finite_reader(r)?.0;
                let wrapped_script_pubkey = Vec::<u8>::consensus_decode_from_finite_reader(r)?;
                if wrapped_script_pubkey.first() != Some(&OP_SPECIAL_TOKEN_PREFIX.to_u8()) {
                    return Err(encode::Error::ParseFailed("Expected serialized token data"));
                }
                let mut wrapped_script_pubkey_slice = &*wrapped_script_pubkey;
                let token_data = OutputData::consensus_decode_from_finite_reader(
                    &mut wrapped_script_pubkey_slice,
                )?;
                if !wrapped_script_pubkey_slice.is_empty() {
                    return Err(encode::Error::ParseFailed(
                        "Extra data after serialized token data",
                    ));
                }
                token = Some(token_data);
            }
            Ok(Self::Unsigned(UnsignedTxIn {
                previous_output,
                unsigned_script_sig: UnsignedScriptSig(script_sig),
                sequence,
                value,
                token,
            }))
        } else {
            Ok(Self::Signed(TxIn {
                previous_output,
                script_sig,
                sequence,
                witness: Default::default(),
            }))
        }
    }
}

trait MyEncodable {
    fn consensus_encode<W: std::io::Write + ?Sized>(
        &self,
        w: &mut W,
    ) -> Result<usize, std::io::Error>;
}

trait MyDecodable: Sized {
    fn consensus_decode_from_finite_reader<R: std::io::Read + ?Sized>(
        r: &mut R,
    ) -> Result<Self, encode::Error>;
}

impl MyEncodable for Vec<MaybeUnsignedTxIn> {
    #[inline]
    fn consensus_encode<W: std::io::Write + ?Sized>(
        &self,
        w: &mut W,
    ) -> Result<usize, std::io::Error> {
        let mut len = 0;
        len += VarInt(self.len() as u64).consensus_encode(w)?;
        for c in self.iter() {
            len += c.consensus_encode(w)?;
        }
        Ok(len)
    }
}

impl MyDecodable for Vec<MaybeUnsignedTxIn> {
    #[inline]
    fn consensus_decode_from_finite_reader<R: std::io::Read + ?Sized>(
        r: &mut R,
    ) -> Result<Self, encode::Error> {
        let len = VarInt::consensus_decode_from_finite_reader(r)?.0;
        // Do not allocate upfront more items than if the sequnce of type
        // occupied roughly quarter a block. This should never be the case
        // for normal data, but even if that's not true - `push` will just
        // reallocate.
        // Note: OOM protection relies on reader eventually running out of
        // data to feed us.
        let max_capacity = MAX_VEC_SIZE / 4 / std::mem::size_of::<MaybeUnsignedTxIn>();
        let mut ret = Vec::with_capacity(core::cmp::min(len as usize, max_capacity));
        for _ in 0..len {
            ret.push(Decodable::consensus_decode_from_finite_reader(r)?);
        }
        Ok(ret)
    }
}

/// Partially signed Bitcoin Cash transaction.
///
/// Compatible with Electron Cash.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct PartiallySignedTransaction {
    /// The protocol version, is currently expected to be 1 or 2 (BIP 68).
    pub version: i32,
    /// Block height or timestamp. Transaction cannot be included in a block until this height/time.
    ///
    /// ### Relevant BIPs
    ///
    /// * [BIP-65 OP_CHECKLOCKTIMEVERIFY](https://github.com/bitcoin/bips/blob/master/bip-0065.mediawiki)
    /// * [BIP-113 Median time-past as endpoint for lock-time calculations](https://github.com/bitcoin/bips/blob/master/bip-0113.mediawiki)
    pub lock_time: PackedLockTime,
    /// List of transaction inputs, possibly unsigned.
    pub input: Vec<MaybeUnsignedTxIn>,
    /// List of transaction outputs.
    pub output: Vec<TxOut>,
}

impl Encodable for PartiallySignedTransaction {
    fn consensus_encode<W: std::io::Write + ?Sized>(
        &self,
        w: &mut W,
    ) -> Result<usize, std::io::Error> {
        Ok(self.version.consensus_encode(w)?
            + self.input.consensus_encode(w)?
            + self.output.consensus_encode(w)?
            + self.lock_time.consensus_encode(w)?)
    }
}

impl Decodable for PartiallySignedTransaction {
    fn consensus_decode_from_finite_reader<R: std::io::Read + ?Sized>(
        r: &mut R,
    ) -> Result<Self, encode::Error> {
        Ok(Self {
            version: i32::consensus_decode_from_finite_reader(r)?,
            input: MyDecodable::consensus_decode_from_finite_reader(r)?,
            output: Decodable::consensus_decode_from_finite_reader(r)?,
            lock_time: Decodable::consensus_decode_from_finite_reader(r)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use bitcoincash::{
        consensus::{deserialize, serialize},
        hashes::hex::FromHex,
    };

    use super::PartiallySignedTransaction;

    #[test]
    fn test_unsigned_transaction() {
        let tx_bytes = Vec::<u8>::from_hex(concat!(
            "01000000013c3b636f926cb2c5a8f971d7e06e488aa3d10f42202b293f936bafdf63d7908a1800000057",
            "01ff4c53ff0488b21e0000000000000000005d2f27f71323296d52bf8475ad8dad79d6239fcd640629fd",
            "dc8ef9a7229258a4023f72ac51c65717e8d44e8d86afacff3eed27ce00cea7b5a6fd1e6297fcbd4df901",
            "00fe15feffffff20090600000000000262e80200000000001976a914c9226d620fe088b4d84a4ab0ca6b",
            "4fe6dfb3193488ace31f0300000000001976a914795b6a18d92f888df281f85373288a6834a7d31a88ac",
            "81cc0c00",
        ))
        .unwrap();
        let tx: PartiallySignedTransaction = deserialize(&tx_bytes).unwrap();
        assert_eq!(tx_bytes, serialize(&tx));
    }

    #[test]
    fn test_signed_transaction() {
        let tx_bytes = Vec::<u8>::from_hex(concat!(
            "010000000123da0881236aad5c493623ca2bbe82e1796119d8546c2dda7ecc7a1e4251c713000000006a",
            "473044022050343561f7a42de739ed32051cf50dace181ccd2e15d41bcae2b2b676a3f553f022050566f",
            "ea7ff2d122d0fad0b84a435927523697a0da8bd742a72fe55e3881b8f84121030a72c3eb8d023aa16385",
            "87293e427819265fd307db1d67de8e5c4129f654bf49ffffffff02dd73e902000000001976a914e22b94",
            "d8e2cb8030f6af8c09749ae10767acf0fd88ac65bad565000000001976a914235baf7ab8973f9a6afb81",
            "cdeda1f9a0ca10e82188ac00000000",
        ))
        .unwrap();
        let tx: PartiallySignedTransaction = deserialize(&tx_bytes).unwrap();
        assert_eq!(tx_bytes, serialize(&tx));
    }

    #[test]
    fn test_coinbase() {
        let tx_bytes = Vec::<u8>::from_hex(concat!(
            "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0e",
            "03e6cb0c2f4e696365486173682fffffffff0300000000000000000e6a0c17d8d7a62027d4b56b519d00",
            "dc26fa24000000001976a9145633aebf44152de83126acc6282c99f8b33422dc88ac219e5f0000000000",
            "1976a914f9bfd1340cce62f2ff7eaff4b751dc0ba90d3f6388ac00000000",
        ))
        .unwrap();
        let tx: PartiallySignedTransaction = deserialize(&tx_bytes).unwrap();
        assert_eq!(tx_bytes, serialize(&tx));
    }
}
