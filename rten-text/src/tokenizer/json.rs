//! Types for the supported subset of the `tokenizer.json` pre-trained tokenizer
//! format.

use std::collections::HashMap;

use super::TokenId;
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct AddedToken {
    pub content: String,
    pub id: TokenId,
}

#[derive(Deserialize)]
pub(crate) enum Pattern {
    Regex(String),
    String(String),
}

pub mod normalizers {
    use serde::Deserialize;

    use super::{Normalizer, Pattern};

    #[derive(Deserialize)]
    pub(crate) struct Bert {
        pub lowercase: bool,
        pub strip_accents: Option<bool>,
    }

    #[derive(Deserialize)]
    pub(crate) struct Replace {
        pub pattern: Pattern,
        pub content: String,
    }

    #[derive(Deserialize)]
    pub(crate) struct Sequence {
        pub normalizers: Vec<Normalizer>,
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub(crate) enum Normalizer {
    #[serde(rename = "BertNormalizer")]
    Bert(normalizers::Bert),
    Lowercase,
    #[serde(rename = "NFC")]
    Nfc,
    #[serde(rename = "NFD")]
    Nfd,
    #[serde(rename = "NFKC")]
    Nfkc,
    #[serde(rename = "NFKD")]
    Nfkd,
    Replace(normalizers::Replace),
    Sequence(normalizers::Sequence),
}

pub mod pre_tokenizers {
    use serde::Deserialize;

    use super::{Pattern, PreTokenizer};

    #[derive(Deserialize)]
    pub(crate) struct ByteLevel {
        pub use_regex: bool,
    }

    #[derive(Deserialize)]
    pub(crate) struct Digits {
        pub individual_digits: bool,
    }

    #[derive(Deserialize)]
    pub(crate) struct Sequence {
        pub pretokenizers: Vec<PreTokenizer>,
    }

    #[derive(Deserialize)]
    pub(crate) enum SplitDelimiter {
        Removed,
        Isolated,
    }

    #[derive(Deserialize)]
    pub(crate) struct Split {
        pub pattern: Pattern,
        pub behavior: SplitDelimiter,
        pub invert: bool,
    }
}

/// Configuration for pre-tokenization.
///
/// See https://huggingface.co/docs/tokenizers/en/api/pre-tokenizers.
#[derive(Deserialize)]
#[serde(tag = "type")]
pub(crate) enum PreTokenizer {
    #[serde(rename = "BertPreTokenizer")]
    Bert,
    #[serde(rename = "ByteLevel")]
    ByteLevel(pre_tokenizers::ByteLevel),
    Digits(pre_tokenizers::Digits),
    Sequence(pre_tokenizers::Sequence),
    Split(pre_tokenizers::Split),
}

#[derive(Deserialize)]
pub(crate) struct WordPieceModel {
    /// Mapping from token text to token ID.
    pub vocab: HashMap<String, TokenId>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum MergeList {
    /// Pairs represented as a JSON array.
    Tuple(Vec<(String, String)>),
    /// Pairs represented as `<token_a> [SPACE] <token_b>`.
    Legacy(Vec<String>),
}

#[derive(Deserialize)]
pub(crate) struct BpeModel {
    /// Mapping from token text to token ID.
    pub vocab: HashMap<String, TokenId>,

    /// List of pairs of tokens to merge.
    pub merges: MergeList,

    /// A string which is implicitly appended to each substring after
    /// pre-tokenization before it is tokenized using BPE.
    ///
    /// This originated from CLIP's tokenizer.
    /// See https://github.com/openai/CLIP/blob/main/clip/simple_tokenizer.py.
    pub end_of_word_suffix: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub(crate) enum Model {
    #[serde(rename = "BPE")]
    Bpe(BpeModel),
    WordPiece(WordPieceModel),
}

/// Structure of the `tokenizers.json` files generated by Hugging Face
/// tokenizers [^1].
///
/// [^1]: https://github.com/huggingface/tokenizers
#[derive(Deserialize)]
pub(crate) struct TokenizerJson {
    pub added_tokens: Option<Vec<AddedToken>>,
    pub normalizer: Option<Normalizer>,
    pub pre_tokenizer: Option<PreTokenizer>,
    pub model: Model,
}

/// Deserialize a `tokenizer.json` file.
pub fn from_json(json: &str) -> Result<TokenizerJson, serde_json::Error> {
    serde_json::from_str(json)
}
