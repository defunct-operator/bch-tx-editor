use std::fmt::LowerHex;

use bitcoincash::{
    blockdata::{
        opcodes::{
            all::{OP_CHECKMULTISIG, OP_SPECIAL_TOKEN_PREFIX},
            Class, ClassifyContext,
        },
        script::{self, Instruction},
        token::OutputData,
    },
    consensus::{
        encode::{self, MAX_VEC_SIZE},
        Decodable, Encodable,
    },
    psbt::serialize::{Deserialize, Serialize},
    secp256k1::{Secp256k1, Verification},
    util::bip32::{ChildNumber, ExtendedPubKey},
    Address, Network, OutPoint, PackedLockTime, Script, Sequence, Transaction, TxIn, TxOut, VarInt,
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

impl LowerHex for UnsignedScriptSig {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl UnsignedScriptSig {
    /// 0xFD: unknown pubkey, but we know the Bitcoin address, i.e. the script pubkey.
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
    pub fn script_pubkey<C: Verification>(&self, secp: &Secp256k1<C>) -> Option<Script> {
        let mut iter = self.0.instructions();
        let Instruction::PushBytes(first_push) = iter.next()?.ok()? else {
            return None;
        };
        if first_push.is_empty() {
            // multisig
            let Instruction::PushBytes(fake_redeem_script) = iter.last()?.ok()? else {
                return None;
            };
            let mut redeem_script = script::Builder::new();
            for ins in Script::from(fake_redeem_script.to_vec()).instructions() {
                match ins.ok()? {
                    Instruction::Op(op) => redeem_script = redeem_script.push_opcode(op),
                    Instruction::PushBytes(xpubkey) => {
                        redeem_script =
                            redeem_script.push_key(&ec_ff_parse_xpubkey(secp, xpubkey)?.to_pub())
                    }
                }
            }
            return Some(redeem_script.into_script().to_p2sh());
        } else if first_push != [0xff] {
            return None;
        }
        match iter.next()?.ok()? {
            Instruction::PushBytes([0xfd, ref spk @ ..]) => Some(spk.to_vec().into()),
            Instruction::PushBytes(bytes @ [0xff, ..]) => {
                let xpubkey = ec_ff_parse_xpubkey(secp, bytes)?;
                Some(Script::new_p2pkh(&xpubkey.to_pub().pubkey_hash()))
            }
            _ => None,
        }
    }

    /// The bare script as it would appear in an Electron Cash unsigned transaction.
    pub fn raw_script(&self) -> &Script {
        &self.0
    }

    pub fn into_raw_script(self) -> Script {
        self.0
    }

    pub fn from_raw_script(s: Script) -> Self {
        Self(s)
    }
}

/// Parse the 0xFF prefixed extended public key, which consists of the bip32 xpub and the
/// derivation.
fn ec_ff_parse_xpubkey<C: Verification>(
    secp: &Secp256k1<C>,
    bytes: &[u8],
) -> Option<ExtendedPubKey> {
    let [0xff, xpub_bytes @ ..] = bytes else {
        return None;
    };

    let mut xpub = ExtendedPubKey::decode(&xpub_bytes[..78]).ok()?;
    let mut path_bytes = &xpub_bytes[78..];
    while !path_bytes.is_empty() {
        let mut n = u32::from(u16::consensus_decode(&mut path_bytes).ok()?);
        if n == 0xffff {
            n = u32::consensus_decode(&mut path_bytes).ok()?;
        }
        xpub = xpub.ckd_pub(secp, ChildNumber::Normal { index: n }).ok()?;
    }
    Some(xpub)
}

fn is_unsigned_p2pkh_payload(s: &[u8]) -> bool {
    match s {
        [0xfd, spk @ ..] => {
            Address::from_script(&Script::from(spk.to_vec()), Network::Bitcoin).is_ok()
        }
        [0xfe | 0xff | 0x02..=0x04, ..] => true, // TODO actually parse?
        _ => false,
    }
}

fn is_multisig(script: &[u8], num_sigs: usize) -> bool {
    let script = Script::from(script.to_vec());
    let Ok(instructions): Result<Vec<_>, _> = script.instructions().collect() else {
        return false;
    };
    // Electron Cash only seems to recognize m and n up to 16
    let [Instruction::Op(m), pubkeys @ .., Instruction::Op(n), checkmultisig] = &instructions[..]
    else {
        return false;
    };
    let Class::PushNum(m) = m.classify(ClassifyContext::Legacy) else {
        return false;
    };
    let Class::PushNum(n) = n.classify(ClassifyContext::Legacy) else {
        return false;
    };
    let Ok(m) = usize::try_from(m) else {
        return false;
    };
    let Ok(n) = usize::try_from(n) else {
        return false;
    };

    *checkmultisig == Instruction::Op(OP_CHECKMULTISIG) && m == num_sigs && n == pubkeys.len()
}

fn is_unsigned_script_sig(s: &Script) -> bool {
    let mut ins = s.instructions();
    match ins.next() {
        // Possibly unsigned p2pkh or unsigned p2sh without xpubkeys
        Some(Ok(Instruction::PushBytes(&[0xff]))) => match ins.next() {
            Some(Ok(Instruction::PushBytes(payload))) => {
                is_unsigned_p2pkh_payload(payload) && ins.next().is_none()
            }
            _ => false,
        },
        // Possibly multisig with xpubkeys
        Some(Ok(Instruction::PushBytes(&[]))) => {
            let mut num_pushes = 0;
            let mut last = None;
            for x in ins {
                match x {
                    Ok(Instruction::PushBytes(b)) => {
                        num_pushes += 1;
                        last = Some(b);
                    }
                    _ => return false,
                }
            }
            let Some(last) = last else { return false };
            is_multisig(last, num_pushes - 1)
        }
        _ => false,
    }
}

/// Unsigned transaction input. Compatible with Electron Cash.
///
/// This only recognizes the 0xFD and the 0xFF public key, that is, the unknown pubkey but known
/// address form, and the bip32 xpub + derivation form.
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

impl MaybeUnsignedTxIn {
    pub fn previous_output(&self) -> &OutPoint {
        match self {
            Self::Unsigned(t) => &t.previous_output,
            Self::Signed(t) => &t.previous_output,
        }
    }

    pub fn previous_output_mut(&mut self) -> &mut OutPoint {
        match self {
            Self::Unsigned(t) => &mut t.previous_output,
            Self::Signed(t) => &mut t.previous_output,
        }
    }

    pub fn sequence(&self) -> Sequence {
        match self {
            Self::Unsigned(t) => t.sequence,
            Self::Signed(t) => t.sequence,
        }
    }

    pub fn sequence_mut(&mut self) -> &mut Sequence {
        match self {
            Self::Unsigned(t) => &mut t.sequence,
            Self::Signed(t) => &mut t.sequence,
        }
    }

    pub fn script_sig(&self) -> Option<&Script> {
        match self {
            Self::Unsigned(_) => None,
            Self::Signed(t) => Some(&t.script_sig),
        }
    }
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
                let mut wrapped_script_pubkey_slice = &wrapped_script_pubkey[1..];
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

impl From<Transaction> for PartiallySignedTransaction {
    fn from(t: Transaction) -> Self {
        Self {
            version: t.version,
            lock_time: t.lock_time,
            input: t.input.into_iter().map(MaybeUnsignedTxIn::Signed).collect(),
            output: t.output,
        }
    }
}

impl Deserialize for PartiallySignedTransaction {
    fn deserialize(bytes: &[u8]) -> Result<Self, encode::Error> {
        bitcoincash::consensus::deserialize(bytes)
    }
}

impl Serialize for PartiallySignedTransaction {
    fn serialize(&self) -> Vec<u8> {
        bitcoincash::consensus::serialize(self)
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
    fn test_unsigned_token_transaction() {
        let tx_bytes = Vec::<u8>::from_hex(concat!(
            "01000000022a4f73d341cb70ef826a2d1942f0acda9bb059536da7be352d54bc45a8c0f1040000000057",
            "01ff4c53ff0488b21e0000000000000000005d2f27f71323296d52bf8475ad8dad79d6239fcd640629fd",
            "dc8ef9a7229258a4023f72ac51c65717e8d44e8d86afacff3eed27ce00cea7b5a6fd1e6297fcbd4df900",
            "003c00feffffffdd73e9020000000062b76b5bb69fa5f572cf1de7c0972e12cd9584128b14cb03317e45",
            "4011ca9a6c000000005701ff4c53ff0488b21e0000000000000000005d2f27f71323296d52bf8475ad8d",
            "ad79d6239fcd640629fddc8ef9a7229258a4023f72ac51c65717e8d44e8d86afacff3eed27ce00cea7b5",
            "a6fd1e6297fcbd4df900003800fefffffffffffffffffffffffde80325efc44ce628940675b075d0e005",
            "9b9ddd165499a0656831f31f4f0adddb3bdd557910fd8c050320030000000000003eefc44ce628940675",
            "b075d0e0059b9ddd165499a0656831f31f4f0adddb3bdd557910fd8b0576a91403266ab5b02f4eebee6c",
            "43bf9fb9d4421cb67d5588ac20030000000000003cefc44ce628940675b075d0e0059b9ddd165499a065",
            "6831f31f4f0adddb3bdd5579100176a914795b6a18d92f888df281f85373288a6834a7d31a88acb36fe9",
            "02000000001976a91403266ab5b02f4eebee6c43bf9fb9d4421cb67d5588ac23cf0c00"
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

    #[test]
    fn test_unsigned_multisig() {
        let tx_bytes = Vec::<u8>::from_hex(concat!(
            "0100000001e504e5e7a9f8de239466eb56fb11f35a7f6abb9fdcf5f880cf7d33ca61f59e2002000000b4",
            "0001ff01ff4cad524c53ff0488b21e038a4e0085800000004a79f36002d5586864107032ba0ef24ed69c",
            "c4443a10c1d83ac3fab997887dda02410a7028fb543bce27b28c41a4e1ce254201d74af75ce0ceeaac13",
            "aaf77f3771000000004c53ff0488b21e03ffe004bd8000000026bbc9039eb31c596735ff6974c27ba089",
            "f3f2978cc0b792d62887c0f60c67b102a928d855d5a997fbc719c8c304122377106222c1fe67576282bc",
            "ceba6afc033d0000000052aefeffffff98a003000000000002011c01000000000017a914c3d5594a1a02",
            "b005e15fa5ce14ea8cb45d668bba87478302000000000017a914616cc2c9da3f60caf6abd9500576984e",
            "4fa484748765d00c00",
        ))
        .unwrap();
        let tx: PartiallySignedTransaction = deserialize(&tx_bytes).unwrap();
        assert_eq!(tx_bytes, serialize(&tx));
    }
}
