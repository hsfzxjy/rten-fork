//! Tools to run the generation loop for an auto-regressive model.

use std::error::Error;
use std::fmt;
use std::ops::Range;

use rten::{Dimension, Input, InputOrOutput, NodeId, Output};
use rten_tensor::prelude::*;
use rten_tensor::{NdTensor, Tensor};

#[cfg(feature = "text-decoder")]
use rten_text::tokenizers::{Tokenizer, TokenizerError};

use crate::metrics::Metrics;
use crate::model::Model;
use crate::sampler::{ArgMaxSampler, Sampler};

#[cfg(feature = "text-decoder")]
use crate::text_decoder::TextDecoder;

/// Errors that occur when creating or running a [`Generator`].
#[derive(Debug)]
pub enum GeneratorError {
    /// An expected model input was not found.
    InputNotFound(String),

    /// An expected model output was not found.
    OutputNotFound(String),

    /// An input or output did not have the expected shape.
    ShapeMismatch(String),

    /// An error occurred while generating the next token.
    GenerateError(Box<dyn Error>),

    /// An error occurred while decoding tokens.
    #[cfg(feature = "text-decoder")]
    DecodeError(TokenizerError),
}

/// Integer type used to represent token IDs.
pub type TokenId = u32;

impl fmt::Display for GeneratorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GeneratorError::InputNotFound(name) => write!(f, "model input not found: {}", name),
            GeneratorError::OutputNotFound(name) => write!(f, "model output not found: {}", name),
            GeneratorError::ShapeMismatch(err) => write!(f, "shape mismatch: {}", err),
            GeneratorError::GenerateError(err) => write!(f, "generation error: {}", err),
            #[cfg(feature = "text-decoder")]
            GeneratorError::DecodeError(err) => write!(f, "decode error: {}", err),
        }
    }
}

impl Error for GeneratorError {}

enum KvCacheData {
    /// Key-value cache with shape `[batch, seq_len, channels]`.
    ///
    /// In this configuration the channels for all heads are combined into the
    /// last dimension.
    BatchSeqChans(NdTensor<f32, 3>),
    /// Key-value cache with shape `[batch, heads, seq_len, channels]`.
    BatchHeadSeqChans(NdTensor<f32, 4>),
}

/// Key-value cache for a single layer of a transformer model.
struct KvCache {
    /// Input ID for this cache entry.
    input_id: NodeId,

    /// Output ID for this cache entry.
    output_id: NodeId,

    /// The cached keys and values. This is set to `None` during inference, as
    /// the model temporarily takes ownership of it.
    cache: Option<KvCacheData>,
}

/// Specifies a pattern for the name of a key-value cache input or output.
///
/// These inputs are expected to have the form `{prefix}{layer_number}{suffix}`,
/// with one input and output per layer for the key cache and the value cache.
pub struct KVCachePattern<'a> {
    pub prefix: &'a str,
    pub suffix: &'a str,
}

impl<'a> From<(&'a str, &'a str)> for KVCachePattern<'a> {
    /// Construct a [`KVCachePattern`] from a `(prefix, suffix)` tuple.
    fn from(value: (&'a str, &'a str)) -> Self {
        let (prefix, suffix) = value;
        KVCachePattern { prefix, suffix }
    }
}

/// Specifies the names of model inputs and outputs.
///
/// The [`Default`] impl for this struct returns an instance whose names
/// follow the configuration of Hugging Face's Optimum tool.
///
/// Any inputs that are not present in the model are ignored.
pub struct ModelInputsConfig<'a> {
    /// Model input that contains the token IDs of the prompt and output
    /// generated so far.
    pub input_ids: &'a str,

    /// Model output that contains logits.
    pub logits: &'a str,

    /// Model input that contains an attention mask.
    pub attention_mask: &'a str,

    /// Model input that contains position IDs for each position.
    pub position_ids: &'a str,

    /// Pattern for key cache inputs.
    pub key_cache: KVCachePattern<'a>,

    /// Pattern for key cache outputs.
    pub key_cache_output: KVCachePattern<'a>,

    /// Pattern for value cache inputs.
    pub value_cache: KVCachePattern<'a>,

    /// Pattern for value cache outputs.
    pub value_cache_output: KVCachePattern<'a>,
}

/// Contains essential configuration needed for a `Generator` to execute a
/// model, such as the roles of different inputs and outputs.
pub struct GeneratorConfig<'a> {
    /// Specifies names and roles of model inputs and outputs.
    pub model_inputs: ModelInputsConfig<'a>,
}

impl<'a> Default for ModelInputsConfig<'a> {
    /// Return default model input names.
    ///
    /// These are based on [Hugging Face's
    /// Optimum](https://huggingface.co/docs/optimum/en/index) model exporter.
    fn default() -> Self {
        ModelInputsConfig {
            input_ids: "input_ids",
            logits: "logits",
            attention_mask: "attention_mask",
            position_ids: "position_ids",
            key_cache: ("past_key_values.", ".key").into(),
            key_cache_output: ("present.", ".key").into(),
            value_cache: ("past_key_values.", ".value").into(),
            value_cache_output: ("present.", ".value").into(),
        }
    }
}

/// Generates a token ID sequence using a transformer decoder model.
///
/// This is an iterator that runs the model on each call to [`Iterator::next`]
/// and yields a result containing the next token ID or an error.
///
/// The token ID sequence can be converted to text using the
/// [`decode`](GeneratorUtils::decode) method of the [`GeneratorUtils`] trait.
///
/// The `GeneratorUtils` trait also provides useful wrappers for the output,
/// such as stopping generation when an end-of-text token is reached. You can
/// also use all of the standard iterator adapters. For example
/// `generator.take(30)` will return an iterator that stops generation after 30
/// tokens have been produced).
///
/// ## Sampling
///
/// The token ID is sampled from the outputs of the model (the "logits") using
/// a [`Sampler`]. By default this is an [`ArgMaxSampler`] which simply chooses
/// the token with the highest probability. The sampler can be configured using
/// [`with_sampler`](Self::with_sampler).
///
/// ## Key-value caches and generation performance
///
/// To enable efficient decoding, the model should have inputs and outputs for
/// the [key-value
/// cache](https://peterchng.com/blog/2024/06/11/what-is-the-transformer-kv-cache/).
/// The generator will work with models that do not have cache inputs, but
/// decoding of long output sequences will be much slower.
pub struct Generator<'a> {
    model: &'a dyn Model,

    /// Additional constant model inputs (eg. encoder outputs) passed to the
    /// model at each step.
    constant_inputs: Vec<(NodeId, InputOrOutput<'a>)>,

    /// Additional model inputs computed using constant propagation. This
    /// effectively caches parts of the graph that don't change in each
    /// generation step. This is `None` if the cache is out of date.
    constant_prop_inputs: Option<Vec<(NodeId, Output)>>,

    /// Additional varying model inputs computed and passed to the model at
    /// each step. The functions receive `(batch_size, sequence_positions)` as inputs.
    #[allow(clippy::type_complexity)]
    varying_inputs: Vec<(NodeId, &'a dyn Fn(usize, Range<usize>) -> InputOrOutput<'a>)>,

    /// Input token IDs for the next run of the model.
    input_ids: Vec<TokenId>,

    // Input node IDs
    input_ids_input: NodeId,

    // Output node IDs
    logits_output: NodeId,

    // Sampler used to get the next token ID from the output logits.
    sampler: Box<dyn Sampler>,

    /// Length of the sequence generated so far.
    seq_len: u32,

    /// Key-value cache.
    kv_cache: Vec<KvCache>,
}

impl<'a> Generator<'a> {
    /// Create a generator that iteratively produces tokens using a model.
    ///
    /// This function assumes default names for model inputs and outputs
    /// based on the conventions of Hugging Face's Optimum exporter. These
    /// can be customized using [`from_model_config`](Self::from_model_config).
    ///
    /// The model must have the required inputs:
    ///
    ///  - `input_ids` - (batch, sequence) tensor of token IDs
    ///
    /// The model may have the optional inputs:
    ///
    ///  - `attention_mask` - (batch, sequence) tensor of booleans
    ///  - `position_ids` - (batch, sequence) tensor of position indices
    ///  - `past_key_values.N.key` - (batch, head, past_seq_len, size) key vector cache
    ///    where `N` is the layer index
    ///  - `past_key_values.N.value` - (batch, head, past_key_values, size) value vector cache,
    ///    where `N` is the layer index
    ///
    /// **Warning:** Generation of long sequences will be much slower in models without
    /// key-value caches.
    ///
    /// The model must have the outputs:
    ///
    ///  - `logits` - output (batch, sequence, vocab) tensor of next token probabilities
    ///
    /// The model may have the optional outputs:
    ///
    ///  - `present.N.key` - (batch, head, past_seq_len + 1, size) updated key vector cache
    ///  - `present.N.value` - (batch, head, past_seq_len + 1, size) updated value vector cache
    pub fn from_model(model: &'a dyn Model) -> Result<Generator<'a>, GeneratorError> {
        let config = GeneratorConfig {
            model_inputs: ModelInputsConfig::default(),
        };
        Self::from_model_config(model, config)
    }

    /// Create a generator that iteratively produces tokens using a model.
    ///
    /// This is a variant of [`from_model`](Self::from_model) that allows
    /// specifying custom names for model inputs.
    pub fn from_model_config(
        model: &'a dyn Model,
        config: GeneratorConfig,
    ) -> Result<Generator<'a>, GeneratorError> {
        let model_inputs = &config.model_inputs;

        let input_ids_input =
            model
                .find_node(model_inputs.input_ids)
                .ok_or(GeneratorError::InputNotFound(
                    model_inputs.input_ids.to_string(),
                ))?;

        let logits_output =
            model
                .find_node(model_inputs.logits)
                .ok_or(GeneratorError::OutputNotFound(
                    model_inputs.logits.to_string(),
                ))?;

        // Find inputs and corresponding outputs for key-value cache.
        let batch_size = 1;
        let mut kv_cache = Vec::new();
        for &input_id in model.input_ids() {
            let input_info = model
                .node_info(input_id)
                .ok_or(GeneratorError::InputNotFound(format!(
                    "input ID {}",
                    input_id
                )))?;

            let name = input_info.name();
            let is_key_cache = name.starts_with(model_inputs.key_cache.prefix)
                && name.ends_with(model_inputs.key_cache.suffix);
            let is_value_cache = name.starts_with(model_inputs.value_cache.prefix)
                && name.ends_with(model_inputs.value_cache.suffix);

            if !is_key_cache && !is_value_cache {
                continue;
            }

            let (n_heads, size) = match *input_info.shape() {
                [_, Dimension::Fixed(n_heads), _, Dimension::Fixed(size)] => (Some(n_heads), size),
                [_, _, Dimension::Fixed(size)] => (None, size),
                _ => {
                    return Err(GeneratorError::ShapeMismatch(format!("input \"{}\" has unexpected shape. expected (batch, past_seq_len, chans) or (batch, heads, past_seq_len, chans) where `heads` and `size` are fixed", name)));
                }
            };

            let prefix = if is_key_cache {
                model_inputs.key_cache.prefix
            } else {
                model_inputs.value_cache.prefix
            };

            let layer_index_start = prefix.len();
            let layer_index_str: String = name[layer_index_start..]
                .chars()
                .take_while(|ch| ch.is_ascii_digit())
                .collect();
            let Ok(layer_index) = layer_index_str.parse::<u32>() else {
                continue;
            };

            let (output_prefix, output_suffix) = if is_key_cache {
                (
                    model_inputs.key_cache_output.prefix,
                    model_inputs.key_cache_output.suffix,
                )
            } else {
                (
                    model_inputs.value_cache_output.prefix,
                    model_inputs.value_cache_output.suffix,
                )
            };

            let output_name = format!("{}{}{}", output_prefix, layer_index, output_suffix);
            let output_id = model
                .find_node(&output_name)
                .ok_or(GeneratorError::OutputNotFound(output_name))?;

            // This value should be configurable.
            let max_seq_len = 512;

            kv_cache.push(KvCache {
                input_id,
                output_id,
                cache: if let Some(n_heads) = n_heads {
                    Some(KvCacheData::BatchHeadSeqChans(NdTensor::with_capacity(
                        [batch_size, n_heads, max_seq_len, size],
                        2, /* seq dim */
                    )))
                } else {
                    Some(KvCacheData::BatchSeqChans(NdTensor::with_capacity(
                        [batch_size, max_seq_len, size],
                        1, /* seq dim */
                    )))
                },
            });
        }

        let mut generator = Generator {
            model,
            constant_inputs: Vec::new(),
            varying_inputs: Vec::new(),

            // Constant propagation is performed as a graph optimization when
            // the model is loaded, so we only need to re-do it if additional
            // constant inputs are added.
            constant_prop_inputs: Some(Vec::new()),

            input_ids: vec![],
            input_ids_input,
            logits_output,
            kv_cache,
            seq_len: 0,
            sampler: Box::new(ArgMaxSampler {}),
        };

        let attention_mask_input = model.find_node(model_inputs.attention_mask);
        if let Some(attention_mask_input) = attention_mask_input {
            generator = generator
                .with_varying_input(attention_mask_input, &|batch_size, positions| {
                    NdTensor::full([batch_size, positions.end], 1i32).into()
                });
        }

        let position_ids_input = model.find_node(model_inputs.position_ids);
        if let Some(position_ids_input) = position_ids_input {
            generator =
                generator.with_varying_input(position_ids_input, &|batch_size, positions| {
                    NdTensor::from_fn([batch_size, positions.len()], |[_batch, pos]| {
                        (positions.start + pos) as i32
                    })
                    .into()
                });
        }

        Ok(generator)
    }

    /// Set the initial sequence of tokens (aka. the prompt) passed to the model
    /// when it is first run.
    ///
    /// To add new inputs after the initial generation, use
    /// [`append_prompt`](Self::append_prompt) instead.
    pub fn with_prompt(mut self, prompt: &[TokenId]) -> Self {
        self.input_ids = prompt.to_vec();
        self
    }

    /// Add input tokens to be included in the next iteration of the model.
    ///
    /// This is useful in applications such as chat where the model's input
    /// alternates between encoded user input and model-generated output.
    pub fn append_prompt(&mut self, prompt: &[TokenId]) {
        self.input_ids.extend(prompt);
    }

    /// Add a constant input which is provided to the model at each iteration.
    ///
    /// A common use case is to pass the outputs of an encoder model to
    /// an auto-regressive decoder.
    pub fn with_constant_input(mut self, input_id: NodeId, value: Input<'a>) -> Self {
        self.constant_prop_inputs = None;
        self.constant_inputs.push((input_id, value.into()));
        self
    }

    /// Add an input which varies with the sequence position.
    ///
    /// `value_fn` receives `(batch_size, sequence_positions)` as input and
    /// computes the value for the input at the given positions.
    ///
    /// A common use case is to pass position embeddings, if they are not
    /// computed internally by the model.
    pub fn with_varying_input<F: Fn(usize, Range<usize>) -> InputOrOutput<'a>>(
        mut self,
        input_id: NodeId,
        value_fn: &'a F,
    ) -> Self {
        self.varying_inputs.push((input_id, value_fn));
        self
    }

    /// Set the sampler used to sample the next token ID from the output logits.
    pub fn with_sampler<S: Sampler + 'static>(mut self, sampler: S) -> Self {
        self.sampler = Box::new(sampler);
        self
    }

    /// Run the model and generate the next token.
    fn generate_next_token(&mut self) -> Result<TokenId, GeneratorError> {
        fn wrap_error<E>(e: E) -> GeneratorError
        where
            E: Into<Box<dyn Error>>,
        {
            GeneratorError::GenerateError(e.into())
        }

        let batch_size = 1;
        let input_ids: NdTensor<i32, 2> = self
            .input_ids
            .iter()
            .map(|id| *id as i32)
            .collect::<Tensor<_>>()
            .into_shape([batch_size, self.input_ids.len()]);

        let seq_range = (self.seq_len as usize)..(self.seq_len as usize + self.input_ids.len());

        let mut model_inputs: Vec<(NodeId, InputOrOutput)> =
            vec![(self.input_ids_input, input_ids.view().into())];

        // Propagate constants on the first run.
        if self.constant_prop_inputs.is_none() {
            let inputs = match self
                .model
                .partial_run(self.constant_inputs.clone(), &[self.logits_output])
            {
                Ok(inputs) => inputs,
                Err(err) => {
                    return Err(wrap_error(err));
                }
            };
            self.constant_prop_inputs = Some(inputs);
        }

        if let Some(constants) = self.constant_prop_inputs.as_ref() {
            model_inputs.extend(
                constants
                    .iter()
                    .map(|(node_id, output)| (*node_id, output.as_input().into())),
            );
        }

        if !self.varying_inputs.is_empty() {
            model_inputs.extend(
                self.varying_inputs
                    .iter()
                    .map(|(node_id, value_fn)| (*node_id, value_fn(batch_size, seq_range.clone()))),
            );
        }

        // Add key-value cache from previous run. The model takes ownership
        // of the KV-cache tensor during the run so it can efficiently append
        // the entry for the current step, without copying the existing buffer.
        for entry in self.kv_cache.iter_mut() {
            let cache = entry.cache.take();
            match cache {
                Some(KvCacheData::BatchSeqChans(cache)) => {
                    model_inputs.push((entry.input_id, cache.into()));
                }
                Some(KvCacheData::BatchHeadSeqChans(cache)) => {
                    model_inputs.push((entry.input_id, cache.into()));
                }
                None => {}
            }
        }

        // Run the model and collect outputs and updated KV cache.
        let model_outputs: Vec<NodeId> = [self.logits_output]
            .into_iter()
            .chain(self.kv_cache.iter().map(|entry| entry.output_id))
            .collect();

        let mut outputs = self
            .model
            .run(model_inputs, &model_outputs)
            .map_err(wrap_error)?;

        // Sample output token.
        let logits: NdTensor<f32, 3> = outputs.remove(0).try_into().map_err(wrap_error)?;
        let next_id = self.sampler.sample(logits.slice::<1, _>((0, -1)));

        // Update the key-value cache.
        //
        // The KV cache tensors returned from the model should be the same as
        // the passed in tensors, but extended by one element along the sequence
        // axis.
        for cache_entry in self.kv_cache.iter_mut() {
            let output = outputs.remove(0);
            let kv_cache = match output.ndim() {
                3 => KvCacheData::BatchSeqChans(output.try_into().map_err(wrap_error)?),
                4 => KvCacheData::BatchHeadSeqChans(output.try_into().map_err(wrap_error)?),
                _ => {
                    return Err(wrap_error("expected KV cache output to have 3 or 4 dims"));
                }
            };
            cache_entry.cache = Some(kv_cache);
        }

        // Update the token IDs and sequence offset for the next iteration.
        if !self.kv_cache.is_empty() {
            self.seq_len += self.input_ids.len() as u32;
            self.input_ids = vec![next_id];
        } else {
            self.input_ids.push(next_id);
        }

        Ok(next_id)
    }
}

/// Output items from a [`Generator`].
pub type GeneratorItem = Result<TokenId, GeneratorError>;

impl<'a> Iterator for Generator<'a> {
    type Item = Result<TokenId, GeneratorError>;

    /// Run the model and generate the next output token.
    fn next(&mut self) -> Option<Self::Item> {
        Some(self.generate_next_token())
    }
}

/// Iterator utilities that wrap a [`Generator`] to perform common tasks such
/// as stopping generation when an end-of-text token is encountered.
pub trait GeneratorUtils: Iterator<Item = GeneratorItem> + Sized {
    /// Stop the generator when any token in `eos_tokens` is encountered.
    fn stop_on_tokens<A: AsRef<[u32]>>(self, eos_tokens: A) -> impl Iterator<Item = GeneratorItem> {
        self.take_while(move |tok| match tok {
            Ok(tok_id) => !eos_tokens.as_ref().contains(tok_id),
            _ => true,
        })
    }

    /// Decode the tokens to text using a tokenizer.
    #[cfg(feature = "text-decoder")]
    fn decode(self, tokenizer: &Tokenizer) -> TextDecoder<Self> {
        TextDecoder::wrap(self, tokenizer)
    }

    /// Record timing metrics.
    ///
    /// Metrics such as the number of tokens generated per second will be
    /// available from `metrics` after generation has finished.
    fn profile(self, metrics: &mut Metrics) -> impl Iterator<Item = Self::Item> {
        Profiler::wrap(self, metrics)
    }
}

impl<I: Iterator<Item = GeneratorItem>> GeneratorUtils for I {}

/// Wraps a [`Generator`] to record timing metrics into a [`Metrics`] struct.
struct Profiler<'a, G: Iterator> {
    generator: G,
    metrics: &'a mut Metrics,
}

impl<'a, G: Iterator> Profiler<'a, G> {
    fn wrap(generator: G, metrics: &'a mut Metrics) -> Profiler<'a, G> {
        Profiler { generator, metrics }
    }
}

impl<'a, G: Iterator> Iterator for Profiler<'a, G> {
    type Item = G::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let start = std::time::Instant::now();
        let item = self.generator.next()?;
        self.metrics.add_step_duration(start.elapsed());
        Some(item)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::error::Error;

    use rten::{Dimension, InputOrOutput, NodeId, Output};
    use rten_tensor::prelude::*;
    use rten_tensor::NdTensor;

    use super::{Generator, GeneratorUtils};
    use crate::metrics::Metrics;
    use crate::model::{Model, NodeInfo};

    struct FakeModel {
        nodes: Vec<NodeInfo>,
        input_ids: Vec<NodeId>,
        output_ids: Vec<NodeId>,

        // Next inference step
        step: Cell<usize>,

        // Inference outputs for each step
        outputs: Vec<HashMap<NodeId, Output>>,

        // Inference inputs for each step
        inputs: RefCell<Vec<HashMap<NodeId, Output>>>,
    }

    impl FakeModel {
        /// Return a model with a given set of inputs and outputs.
        fn with_inputs_and_outputs(inputs: &[NodeInfo], outputs: &[NodeInfo]) -> FakeModel {
            let node_infos = [inputs, outputs].concat();
            let input_ids = (0..inputs.len()).collect();
            let output_ids = (inputs.len()..(inputs.len() + outputs.len())).collect();

            FakeModel {
                input_ids,
                output_ids,
                nodes: node_infos,
                step: Cell::new(0),
                inputs: RefCell::new(vec![]),
                outputs: vec![],
            }
        }

        /// Add inference outputs for one run of the model.
        fn add_outputs(&mut self, outputs: HashMap<NodeId, Output>) {
            self.outputs.push(outputs)
        }

        /// Get an input for the `step`th run of the model.
        fn get_inputs(&self, step: usize, node_id: NodeId) -> Option<Output> {
            self.inputs
                .borrow()
                .get(step)
                .map(|step_inputs| step_inputs.get(&node_id))
                .flatten()
                .cloned()
        }
    }

    impl Model for FakeModel {
        fn find_node(&self, name: &str) -> Option<NodeId> {
            self.nodes.iter().position(|info| info.name() == name)
        }

        fn node_info(&self, id: NodeId) -> Option<NodeInfo> {
            self.nodes.get(id).cloned()
        }

        fn input_ids(&self) -> &[NodeId] {
            &self.input_ids
        }

        fn run(
            &self,
            inputs: Vec<(NodeId, InputOrOutput)>,
            outputs: &[NodeId],
        ) -> Result<Vec<Output>, Box<dyn Error>> {
            if let Some((input_id, _)) = inputs.iter().find(|(id, _)| !self.input_ids.contains(id))
            {
                return Err(format!("invalid input ID {}", input_id).into());
            }

            if let Some(output_id) = outputs.iter().find(|id| !self.output_ids.contains(id)) {
                return Err(format!("invalid output ID {}", output_id).into());
            }

            self.inputs.borrow_mut().push(
                inputs
                    .into_iter()
                    .map(|(id, input_or_output)| (id, input_or_output.to_output()))
                    .collect(),
            );

            let result = outputs
                .iter()
                .map(|id| {
                    let step_outputs = self
                        .outputs
                        .get(self.step.get())
                        .expect("outputs not specified for step");

                    step_outputs
                        .get(id)
                        .cloned()
                        .expect("invalid output node ID")
                })
                .collect();

            self.step.set(self.step.get() + 1);

            Ok(result)
        }

        fn partial_run(
            &self,
            _inputs: Vec<(NodeId, InputOrOutput)>,
            _outputs: &[NodeId],
        ) -> Result<Vec<(NodeId, Output)>, Box<dyn Error>> {
            Ok(Vec::new())
        }
    }

    /// Generate `[batch, sequence, n_vocab]` tensor for `logits` output.
    fn generate_logits(n_vocab: usize, token_ids: &[u32]) -> NdTensor<f32, 3> {
        let mut logits = NdTensor::zeros([1, token_ids.len(), n_vocab]);
        for (idx, id) in token_ids.iter().copied().enumerate() {
            logits[[0, idx, id as usize]] = 1.0;
        }
        logits
    }

    #[derive(Copy, Clone, PartialEq)]
    struct TransformerParams {
        /// Number of layers. This determines the number of KV-cache inputs
        /// and outputs.
        n_layers: usize,
        n_heads: usize,
        n_embed: usize,

        /// Vocabulary size. This is the size of the last dimension of the
        /// logits output.
        n_vocab: usize,
    }

    impl Default for TransformerParams {
        fn default() -> Self {
            Self {
                n_layers: 5,
                n_heads: 3,
                n_vocab: 5,
                n_embed: 8,
            }
        }
    }

    /// Create a fake transformer model using the default names for inputs and
    /// outputs.
    fn fake_transformer_model(
        params: TransformerParams,
        use_kv_cache: bool,
        prompt_len: usize,
        output_token_ids: &[u32],
    ) -> FakeModel {
        let TransformerParams {
            n_layers,
            n_heads,
            n_vocab,
            n_embed,
        } = params;

        // Add inputs and outputs using the standard names.
        let mut inputs = vec![
            NodeInfo::from_name_shape("input_ids", &[]),
            NodeInfo::from_name_shape("position_ids", &[]),
            NodeInfo::from_name_shape("attention_mask", &[]),
        ];
        let mut outputs = vec![NodeInfo::from_name_shape("logits", &[])];

        // Add KV-cache inputs and outputs.
        let mut kv_cache_output_names = Vec::new();
        if use_kv_cache {
            for layer in 0..n_layers {
                let dims = [
                    Dimension::Symbolic("batch".to_string()),
                    Dimension::Fixed(n_heads as usize),
                    Dimension::Symbolic("seq".to_string()),
                    Dimension::Fixed(n_embed),
                ];
                let past_key_name = format!("past_key_values.{}.key", layer);
                let past_value_name = format!("past_key_values.{}.value", layer);
                let present_key_name = format!("present.{}.key", layer);
                let present_value_name = format!("present.{}.value", layer);

                inputs.push(NodeInfo::from_name_shape(&past_key_name, &dims));
                inputs.push(NodeInfo::from_name_shape(&past_value_name, &dims));

                outputs.push(NodeInfo::from_name_shape(&present_key_name, &dims));
                outputs.push(NodeInfo::from_name_shape(&present_value_name, &dims));
                kv_cache_output_names.push(present_key_name);
                kv_cache_output_names.push(present_value_name);
            }
        }

        let mut model = FakeModel::with_inputs_and_outputs(&inputs, &outputs);
        let logits_id = model.find_node("logits").unwrap();

        for (step, output_token_id) in output_token_ids.iter().copied().enumerate() {
            assert!(
                output_token_id < n_vocab as u32,
                "token ID is invalid for vocab size"
            );

            let logits = if use_kv_cache {
                generate_logits(n_vocab, &[output_token_id])
            } else {
                generate_logits(n_vocab, &output_token_ids[..=step])
            };

            let mut outputs = HashMap::new();
            outputs.insert(logits_id, Output::FloatTensor(logits.into()));

            // Add KV cache outputs
            for kv_output in kv_cache_output_names.iter() {
                let kv_output_id = model.find_node(&kv_output).unwrap();
                let context_len = if step == 0 {
                    prompt_len
                } else {
                    prompt_len + step - 1
                };
                outputs.insert(
                    kv_output_id,
                    Output::FloatTensor(NdTensor::zeros([1, n_heads, context_len, n_embed]).into()),
                );
            }

            model.add_outputs(outputs);
        }

        model
    }

    fn test_generator_impl(use_kv_cache: bool) -> Result<(), Box<dyn Error>> {
        let params = TransformerParams::default();
        let expected_token_ids = [0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 0, 0];
        let prompt = [1, 2, 3, 1, 2, 3];
        let model = fake_transformer_model(params, use_kv_cache, prompt.len(), &expected_token_ids);

        let generator = Generator::from_model(&model)?;
        let generation_len = 10;

        let output_token_ids: Vec<_> = generator
            .with_prompt(&prompt)
            .take(generation_len)
            .map(|id| id.expect("generation failed"))
            .collect();

        // Check generator outputs
        assert_eq!(output_token_ids.len(), generation_len);
        assert_eq!(output_token_ids, &expected_token_ids[..generation_len]);

        // Check model inputs
        let input_id = model.find_node("input_ids").unwrap();
        let position_ids = model.find_node("position_ids").unwrap();
        let attention_mask = model.find_node("attention_mask").unwrap();

        for step in 0..generation_len {
            let step_inputs = model.get_inputs(step, input_id).unwrap();
            let step_inputs: NdTensor<i32, 2> = step_inputs.try_into().unwrap();

            let step_pos_ids = model.get_inputs(step, position_ids).unwrap();
            let step_pos_ids: NdTensor<i32, 2> = step_pos_ids.try_into().unwrap();

            let step_attn_mask = model.get_inputs(step, attention_mask).unwrap();
            let step_attn_mask: NdTensor<i32, 2> = step_attn_mask.try_into().unwrap();

            if step == 0 {
                assert_eq!(step_inputs.size(1), prompt.len());
                assert!(step_inputs
                    .iter()
                    .map(|x| *x as u32)
                    .eq(prompt.iter().copied()));

                assert_eq!(step_attn_mask.size(1), prompt.len());
                assert!(step_attn_mask.iter().all(|x| *x == 1));

                assert_eq!(step_pos_ids.size(1), prompt.len());
                assert!(step_pos_ids.iter().map(|x| *x as usize).eq(0..prompt.len()));
            } else if use_kv_cache {
                assert_eq!(step_inputs.size(1), 1);
                assert_eq!(step_inputs[[0, 0]] as u32, expected_token_ids[step - 1]);

                assert_eq!(step_attn_mask.size(1), prompt.len() + step);
                assert_eq!(step_attn_mask[[0, 0]], 1);

                assert_eq!(step_pos_ids.size(1), 1);
                assert_eq!(step_pos_ids[[0, 0]], (prompt.len() + step - 1) as i32);
            } else {
                let expected_inputs: Vec<i32> = prompt
                    .iter()
                    .copied()
                    .chain(expected_token_ids)
                    .take(prompt.len() + step)
                    .map(|x| x as i32)
                    .collect();
                assert_eq!(
                    step_inputs,
                    NdTensor::from_data([1, expected_inputs.len()], expected_inputs)
                );

                let expected_attn_mask = vec![1i32; prompt.len() + step];
                assert_eq!(
                    step_attn_mask,
                    NdTensor::from_data([1, expected_attn_mask.len()], expected_attn_mask)
                );

                let expected_pos_ids: Vec<i32> =
                    (0..prompt.len() + step).map(|x| x as i32).collect();
                assert_eq!(
                    step_pos_ids,
                    NdTensor::from_data([1, expected_pos_ids.len()], expected_pos_ids)
                );
            }
        }

        Ok(())
    }

    #[test]
    fn test_generator() -> Result<(), Box<dyn Error>> {
        test_generator_impl(true /* use_kv_cache */)
    }

    #[test]
    fn test_generator_without_kv_cache() -> Result<(), Box<dyn Error>> {
        test_generator_impl(false /* use_kv_cache */)
    }

    #[test]
    fn test_generator_append_prompt() -> Result<(), Box<dyn Error>> {
        let mut params = TransformerParams::default();
        params.n_vocab = 110;
        let output_token_ids = [0, 1, 2, 3, 4, 5, 6, 7, 8];
        let prompt = [99];
        let model = fake_transformer_model(
            params,
            true, /* use_kv_cache */
            prompt.len(),
            &output_token_ids,
        );

        let mut generator = Generator::from_model(&model)?.with_prompt(&prompt);

        generator.next();
        generator.append_prompt(&[100]);
        generator.next();
        generator.append_prompt(&[101, 102]);
        generator.next();

        let input_id = model.find_node("input_ids").unwrap();

        // The input to the first step is just the prompt.
        let inputs = model.get_inputs(0, input_id).unwrap();
        let inputs: NdTensor<i32, 2> = inputs.try_into().unwrap();
        assert_eq!(inputs, NdTensor::from([[99]]));

        // The inputs for the next steps are the output followed by the inputs
        // added with `append_prompt`.
        let inputs = model.get_inputs(1, input_id).unwrap();
        let inputs: NdTensor<i32, 2> = inputs.try_into().unwrap();
        assert_eq!(inputs, NdTensor::from([[0, 100]]));

        let inputs = model.get_inputs(2, input_id).unwrap();
        let inputs: NdTensor<i32, 2> = inputs.try_into().unwrap();
        assert_eq!(inputs, NdTensor::from([[1, 101, 102]]));

        Ok(())
    }

    #[test]
    fn test_stop_on_tokens() -> Result<(), Box<dyn Error>> {
        let params = TransformerParams::default();
        let expected_token_ids = [0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 0, 0];
        let prompt = [1, 2, 3, 1, 2, 3];
        let model = fake_transformer_model(
            params,
            true, /* use_kv_cache */
            prompt.len(),
            &expected_token_ids,
        );

        let generator = Generator::from_model(&model)?;

        let output_token_ids: Vec<_> = generator
            .with_prompt(&prompt)
            .stop_on_tokens([4])
            .map(|id| id.expect("generation failed"))
            .collect();

        assert_eq!(output_token_ids, &[0, 1, 2, 3]);

        Ok(())
    }

    #[test]
    fn test_profile() -> Result<(), Box<dyn Error>> {
        let params = TransformerParams::default();
        let expected_token_ids = [0, 1, 2, 3, 4];
        let prompt = [1, 2, 3, 1, 2, 3];
        let model = fake_transformer_model(
            params,
            true, /* use_kv_cache */
            prompt.len(),
            &expected_token_ids,
        );

        let generator = Generator::from_model(&model)?;
        let mut metrics = Metrics::new();

        let output_token_ids: Vec<_> = generator
            .with_prompt(&prompt)
            .profile(&mut metrics)
            .take(expected_token_ids.len())
            .map(|id| id.expect("generation failed"))
            .collect();

        assert_eq!(output_token_ids, expected_token_ids);
        assert!(metrics.warmup_duration().is_some());
        assert_eq!(metrics.step_durations().len(), output_token_ids.len() - 1);

        Ok(())
    }
}
