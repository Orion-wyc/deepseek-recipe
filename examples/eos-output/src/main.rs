//! Demonstrate internal EOS accounting versus public response content.
//!
//! The adapter layer owns the decision to retain EOS for accounting while
//! filtering it before sending token IDs to `StreamProcessor`.

use deepseek_recipe::openai::ChatCompletionRequest;
use deepseek_recipe::request::{ConversionOptions, ProtocolRequest};
use deepseek_recipe::stream::{
    InferenceChunk, InferenceFinishReason, PromptUsage, StreamProcessor,
};
use serde_json::json;
use tokenizers::Tokenizer;
use tokio_stream::{StreamExt, iter};

// Rust guideline compliant 2026-10-09

const TOKENIZER_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../static/tokenizers/v41/tokenizer.json"
);
const EOS: &str = "<｜end▁of▁sentence｜>";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tokenizer = Tokenizer::from_file(TOKENIZER_PATH)?;
    let eos_id = tokenizer
        .token_to_id(EOS)
        .ok_or("the V4.1 tokenizer has no EOS token")?;
    let hello_ids = tokenizer.encode("hello", false)?.get_ids().to_vec();

    println!("EOS token : {EOS}");
    println!("EOS ID    : {eos_id}");
    println!("hello IDs : {hello_ids:?}\n");

    let mut eos_then_hello = vec![eos_id];
    eos_then_hello.extend(&hello_ids);
    run_case(
        "current behavior: [EOS, hello], stop",
        eos_then_hello,
        InferenceFinishReason::Stop,
        false,
        eos_id,
    )
    .await?;

    run_case(
        "current behavior: [EOS], length",
        vec![eos_id],
        InferenceFinishReason::Length,
        false,
        eos_id,
    )
    .await?;

    let mut filtered = vec![eos_id];
    filtered.extend(&hello_ids);
    run_case(
        "recommended behavior: count EOS, filter before Recipe",
        filtered,
        InferenceFinishReason::Stop,
        true,
        eos_id,
    )
    .await?;

    println!("Tail-delay demonstration:");
    println!("  current: frame(token_1) -> no token emitted");
    println!("           frame(token_2) -> token_1 emitted");
    println!("  recommended: emit non-EOS tokens immediately; retain only EOS if needed");

    Ok(())
}

async fn run_case(
    label: &str,
    backend_ids: Vec<u32>,
    finish_reason: InferenceFinishReason,
    filter_eos: bool,
    eos_id: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let body = json!({
        "model": "DeepSeek-V4.1-Flash",
        "messages": [{"role": "user", "content": "Say hello"}],
        "thinking": {"type": "disabled"},
        "stream": true
    });
    let request: ChatCompletionRequest = serde_json::from_value(body)?;
    let converted = request.convert(ConversionOptions::default())?;
    let generator = ChatCompletionRequest::chunk_generator(
        &converted,
        "eos-example".into(),
        "DeepSeek-V4.1-Flash".into(),
    );
    let processor = StreamProcessor::new(generator, converted.parsing_options)
        .with_tokenizer(Tokenizer::from_file(TOKENIZER_PATH)?);

    let generated_count = backend_ids.len();
    let recipe_ids: Vec<u32> = if filter_eos {
        backend_ids
            .iter()
            .copied()
            .filter(|id| *id != eos_id)
            .collect()
    } else {
        backend_ids
    };
    let inference = iter([
        InferenceChunk::Ready {
            system_fingerprint: None,
            prompt_usage: PromptUsage {
                prompt_tokens: 1,
                prompt_cache_hit_tokens: 0,
            },
        },
        InferenceChunk::Tokens {
            token_ids: recipe_ids.clone(),
            logprobs: None,
        },
        InferenceChunk::Finish { finish_reason },
    ]);

    let mut output = String::new();
    let mut stream = std::pin::pin!(processor.into_protocol_stream(inference));
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        output.push_str(&serde_json::to_string(&chunk)?);
        output.push('\n');
    }

    println!("=== {label} ===");
    println!("backend token count : {generated_count}");
    println!("Recipe input IDs    : {recipe_ids:?}");
    println!("public chunks:\n{output}");
    println!("adapter must override usage with backend count: {generated_count}\n");

    Ok(())
}
