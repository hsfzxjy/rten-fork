//! This crate provides tokenizers for encoding text into token IDs
//! for model inputs and decoding output token IDs back into text.
//!
//! The tokenization process follows the
//! [pipeline](https://huggingface.co/docs/tokenizers/en/pipeline) used by the
//! Hugging Face [Tokenizers](https://huggingface.co/docs/tokenizers/en/)
//! library.  Tokenizers can either be constructed manually or loaded from
//! Hugging Face `tokenizer.json` files.
//!
//! ## Comparison to _tokenizers_ crate
//!
//! The canonical implementation of this tokenization pipeline is the
//! [`tokenizers`](https://github.com/huggingface/tokenizers) crate. The main
//! differences compared to that crate are:
//!
//! - rten-text focuses on inference only and does not support training
//!   tokenizers.
//! - rten-text is a pure Rust library with no dependencies written in C/C++.
//!   This means it is easy to build for WebAssembly and other targets where
//!   non-Rust dependencies may cause difficulties.
//! - rten-text is integrated with the
//! [rten-generate](https://docs.rs/rten-generate/) library which handles
//!   running the complete inference loop for auto-regressive transformer
//!   models. Note that you can use rten-generate's outputs with other tokenizer
//!   libraries if rten-text is not suitable.
//! - Not all tokenizer features are currently implemented in rten-text. Please
//!   file an issue if you find that rten-text is missing a feature needed for a
//!   particular model's tokenizer.
//!
//! ## Loading a pre-trained tokenizer
//!
//! The main entry point is the [`Tokenizer`] type. Use [`Tokenizer::from_file`]
//! or [`Tokenizer::from_json`] to construct a tokenizer from a `tokenizer.json`
//! file.
//!
//! ## Encoding text
//!
//! The [`Tokenizer::encode`] method is used to encode text into token IDs.
//! This can be used for example to encode a model's prompt:
//!
//! ```no_run
//! use rten_text::Tokenizer;
//!
//! let tokenizer = Tokenizer::from_file("gpt2/tokenizer.json")?;
//! let encoded = tokenizer.encode("some text to tokenize", None)?;
//! let token_ids = encoded.token_ids(); // Sequence of token IDs
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Decoding text
//!
//! Given token IDs generated by a model, you can decode them back into text
//! using the [`Tokenizer::decode`] method:
//!
//! ```no_run
//! use rten_text::Tokenizer;
//!
//! let tokenizer = Tokenizer::from_file("gpt2/tokenizer.json")?;
//! // Run model and get token IDs from outputs...
//! let token_ids = [101, 4256, 300];
//! let text = tokenizer.decode(&token_ids)?;
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```
//!
//! ## More examples
//!
//! See the
//! [rten-examples](https://github.com/robertknight/rten/tree/main/rten-examples)
//! crate for various examples showing how to use this crate as part of an
//! end-to-end pipeline.

pub mod models;
pub mod normalizers;
pub mod pre_tokenizers;
pub mod tokenizer;

mod split;

pub use tokenizer::{TokenId, Tokenizer, TokenizerError};
