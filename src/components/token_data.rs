use std::borrow::Cow;

use anyhow::Result;
use bitcoincash::{
    blockdata::token::{Capability, OutputData, Structure},
    hashes::hex::{FromHex, ToHex},
    TokenID,
};
use leptos::{
    component, event_target_checked, event_target_value, view, IntoView, RwSignal, Signal,
    SignalDispose, SignalGet, SignalSet, SignalUpdate, SignalWith,
};

use crate::components::ParsedInput;

#[derive(Copy, Clone, Default)]
pub enum NftCapability {
    #[default]
    Immutable,
    Mutable,
    Minting,
}

impl NftCapability {
    pub fn to_str(self) -> &'static str {
        match self {
            Self::Immutable => "immutable",
            Self::Mutable => "mutable",
            Self::Minting => "minting",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "immutable" => Some(Self::Immutable),
            "mutable" => Some(Self::Mutable),
            "minting" => Some(Self::Minting),
            _ => None,
        }
    }
}

impl From<NftCapability> for Capability {
    fn from(t: NftCapability) -> Self {
        use bitcoincash::blockdata::token::Capability as C;
        match t {
            NftCapability::Immutable => C::None,
            NftCapability::Mutable => C::Mutable,
            NftCapability::Minting => C::Minting,
        }
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub enum NftCommitmentFormat {
    #[default]
    Hex,
    Plaintext,
}

impl NftCommitmentFormat {
    pub fn to_str(self) -> &'static str {
        match self {
            Self::Hex => "hex",
            Self::Plaintext => "plaintext",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "hex" => Some(Self::Hex),
            "plaintext" => Some(Self::Plaintext),
            _ => None,
        }
    }
}

#[derive(Copy, Clone)]
pub struct TokenDataState {
    pub cashtoken_enabled: RwSignal<bool>,
    pub category_id: RwSignal<String>,
    pub has_ft_amount: RwSignal<bool>,
    pub ft_amount: RwSignal<u64>,
    pub has_nft: RwSignal<bool>,
    pub nft_capability: RwSignal<NftCapability>,
    pub nft_commitment_hex: RwSignal<String>,
    pub nft_commitment_format: RwSignal<NftCommitmentFormat>,
    pub key: usize,
}

impl TokenDataState {
    pub fn new(key: usize) -> Self {
        Self {
            cashtoken_enabled: RwSignal::new(false),
            category_id: RwSignal::default(),
            has_ft_amount: RwSignal::new(false),
            ft_amount: RwSignal::new(0),
            has_nft: RwSignal::new(false),
            nft_capability: RwSignal::default(),
            nft_commitment_hex: RwSignal::default(),
            nft_commitment_format: RwSignal::default(),
            key,
        }
    }

    pub fn dispose(self) {
        self.cashtoken_enabled.dispose();
        self.category_id.dispose();
        self.has_ft_amount.dispose();
        self.ft_amount.dispose();
        self.has_nft.dispose();
        self.nft_capability.dispose();
        self.nft_commitment_hex.dispose();
        self.nft_commitment_format.dispose();
    }

    pub fn token_data(self) -> Result<Option<OutputData>> {
        Ok(match self.cashtoken_enabled.get() {
            false => None,
            true => {
                let ft_amount = if self.has_ft_amount.get() {
                    if self.ft_amount.get() == 0 {
                        anyhow::bail!("FT amount must be nonzero");
                    }
                    i64::try_from(self.ft_amount.get())?
                } else {
                    0
                };
                let has_nft = self.has_nft.get();
                let capability = match has_nft {
                    true => self.nft_capability.get().into(),
                    false => Capability::None,
                };
                let commitment: Vec<u8> = match has_nft {
                    true => self.nft_commitment_hex.with(|h| <_>::from_hex(h))?,
                    false => vec![],
                };
                let mut structure = 0;
                if ft_amount != 0 {
                    structure |= Structure::HasAmount as u8;
                }
                if has_nft {
                    structure |= Structure::HasNFT as u8;
                }
                if !commitment.is_empty() {
                    structure |= Structure::HasCommitmentLength as u8;
                }
                Some(OutputData {
                    id: self.category_id.with(|h| TokenID::from_hex(h))?,
                    bitfield: structure | capability as u8,
                    amount: ft_amount,
                    commitment,
                })
            }
        })
    }

    pub fn update_from_token_data(self, token_data: Option<&OutputData>) {
        match token_data {
            None => {
                self.cashtoken_enabled.set(false);
                self.category_id.update(String::clear);
                self.has_ft_amount.set(false);
                self.ft_amount.set(0);
                self.has_nft.set(false);
                self.nft_capability.set(NftCapability::default());
                self.nft_commitment_hex.update(String::clear);
                self.nft_commitment_format
                    .set(NftCommitmentFormat::default());
            }
            Some(token_data) => {
                self.cashtoken_enabled.set(true);
                self.category_id.set(token_data.id.to_hex());
                self.has_ft_amount.set(token_data.amount != 0);
                self.ft_amount
                    .set(u64::try_from(token_data.amount).unwrap());
                let has_nft = token_data.has_nft();
                self.has_nft.set(has_nft);
                if has_nft {
                    self.nft_capability.set(
                        if (token_data.capability() & Capability::Mutable as u8) != 0 {
                            NftCapability::Mutable
                        } else if (token_data.capability() & Capability::Minting as u8) != 0 {
                            NftCapability::Minting
                        } else {
                            NftCapability::Immutable
                        },
                    );
                    if token_data.has_commitment_length() {
                        self.nft_commitment_hex.set(token_data.commitment.to_hex());
                    } else {
                        self.nft_commitment_hex.update(String::clear);
                    }
                    self.nft_commitment_format
                        .set(NftCommitmentFormat::default());
                } else {
                    self.nft_capability.set(NftCapability::default());
                    self.nft_commitment_hex.update(String::clear);
                    self.nft_commitment_format
                        .set(NftCommitmentFormat::default());
                }
            }
        }
    }
}

#[component]
pub fn TokenData(token_data: TokenDataState) -> impl IntoView {
    let cashtoken_enabled = token_data.cashtoken_enabled;
    let has_ft_amount = token_data.has_ft_amount;
    let has_nft = token_data.has_nft;
    let nft_capability = token_data.nft_capability;
    let nft_commitment_hex = token_data.nft_commitment_hex;
    let nft_commitment_format = token_data.nft_commitment_format;

    let nft_commitment_error = RwSignal::new(false);
    let nft_commitment_lossy = RwSignal::new(false);

    let parsed_input_ft_id = format!("tx-output-ft-{}", token_data.key);
    let input_category_id = format!("tx-output-cat-{}", token_data.key);

    view! {
        // CashToken category
        <div class="mt-3 mb-1 flex" class=("hidden", move || !cashtoken_enabled())>
            <label for=input_category_id.clone() class="mr-1">Category:</label>
            <input
                id=input_category_id
                on:change=move |e| token_data.category_id.set(event_target_value(&e))
                class=concat!(
                    "border border-solid rounded border-stone-600 px-1 bg-stone-900 ",
                    "font-mono grow placeholder:text-stone-600",
                )
                prop:value=token_data.category_id
                placeholder="Category ID"
            />
        </div>

        // CashToken fungible amount
        <div class="my-1 ml-1" class=("hidden", move || !cashtoken_enabled())>
            <label>
                <input
                    type="checkbox"
                    on:change=move |e| has_ft_amount.set(event_target_checked(&e))
                    prop:checked=has_ft_amount
                />
                FT
            </label>
            <label
                class="mr-1"
                class=("opacity-30", move || !has_ft_amount())
                for=parsed_input_ft_id.clone()
            >
                Amount:
            </label>
            <ParsedInput
                id=parsed_input_ft_id
                value=token_data.ft_amount
                class="w-52 disabled:opacity-30"
                disabled=Signal::derive(move || !has_ft_amount())
            />
        </div>

        // CashToken NFT
        <div class="my-1 ml-1 flex" class=("hidden", move || !cashtoken_enabled())>
            <label class="whitespace-nowrap mr-1">
                <input
                    type="checkbox"
                    on:change=move |e| has_nft.set(event_target_checked(&e))
                    prop:checked=has_nft
                />
                NFT
            </label>

            // NFT Capability
            <div class="grow">
                <select
                    class="bg-inherit border rounded p-1 disabled:opacity-30"
                    disabled=move || !has_nft()
                    on:input=move |e| {
                        nft_capability.set(
                            NftCapability::from_str(&event_target_value(&e)).unwrap()
                        )
                    }
                    prop:value={move || nft_capability().to_str()}
                >
                    <option value={|| NftCapability::Immutable.to_str()}>Immutable</option>
                    <option value={|| NftCapability::Mutable.to_str()}>Mutable</option>
                    <option value={|| NftCapability::Minting.to_str()}>Minting</option>
                </select>

                // NFT commitment
                <div class="my-1 flex" class=("hidden", move || !cashtoken_enabled())>
                    <textarea
                        spellcheck="false"
                        rows=1
                        on:change=move |e| {
                            match nft_commitment_format() {
                                NftCommitmentFormat::Hex => {
                                    nft_commitment_hex.set(event_target_value(&e));
                                }
                                NftCommitmentFormat::Plaintext => {
                                    nft_commitment_hex.set(event_target_value(&e).as_bytes().to_hex());
                                }
                            }
                        }
                        class=concat!(
                            "border border-solid rounded border-stone-600 px-1 w-full bg-inherit ",
                            "placeholder:text-stone-600 font-mono grow bg-stone-900 ",
                        )
                        placeholder="Commitment"
                        prop:value=move || {
                            match nft_commitment_format() {
                                NftCommitmentFormat::Hex => {
                                    nft_commitment_error.set(false);
                                    nft_commitment_lossy.set(false);
                                    nft_commitment_hex()
                                }
                                NftCommitmentFormat::Plaintext => 'a: {
                                    let bytes = match nft_commitment_hex.with(|h| Vec::from_hex(h)) {
                                        Ok(b) => b,
                                        Err(e) => {
                                            nft_commitment_error.set(true);
                                            nft_commitment_lossy.set(false);
                                            break 'a e.to_string();
                                        }
                                    };
                                    nft_commitment_error.set(false);
                                    let text = String::from_utf8_lossy(&bytes);
                                    match text {
                                        Cow::Borrowed(s) => {
                                            nft_commitment_lossy.set(false);
                                            s.into()
                                        }
                                        Cow::Owned(s) => {
                                            nft_commitment_lossy.set(true);
                                            s
                                        }
                                    }
                                }
                            }
                        }
                        disabled=move || !has_nft()
                            || nft_commitment_error()
                            || nft_commitment_lossy()
                        class=("text-red-700", nft_commitment_error)
                        class=("text-yellow-700", nft_commitment_lossy)
                        class=("opacity-30", move || !has_nft())
                    />
                    <div>
                        <select
                            class="bg-inherit border rounded ml-1 p-1 disabled:opacity-30"
                            disabled=move || !has_nft()
                            on:input=move |e| {
                                nft_commitment_format.set(
                                    NftCommitmentFormat::from_str(&event_target_value(&e)).unwrap()
                                )
                            }
                            prop:value={move || nft_commitment_format().to_str()}
                        >
                            <option value={|| NftCommitmentFormat::Hex.to_str()}>Hex</option>
                            <option value={|| NftCommitmentFormat::Plaintext.to_str()}>Plaintext</option>
                        </select>
                    </div>
                </div>
            </div>
        </div>
    }
}
