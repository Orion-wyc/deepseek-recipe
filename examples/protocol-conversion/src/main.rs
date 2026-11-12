//! Convert the same conversation from the three supported API protocols into
//! DeepSeek V4/V4.1 prompts.
//!
//! The example covers the basic conversion path:
//!
//! 1. Deserialize a protocol request (`ChatCompletionRequest`,
//!    `ResponsesRequest`, or `MessagesRequest`).
//! 2. Call `ProtocolRequest::convert` to get a protocol-independent
//!    `ConversationRequest`.
//! 3. Select the prompt encoding from `ConversationRequest::model_version` and
//!    render the conversation.
//! 4. Resolve `max_tokens` from a sample prompt length.
//!
//! It needs no model, API key, tokenizer, or OpenCV. Run it from the repository
//! root:
//!
//! ```sh
//! cargo run -p protocol-conversion-example --locked
//! cargo run -p protocol-conversion-example --locked -- messages
//! ```

use deepseek_recipe::anthropic::MessagesRequest;
use deepseek_recipe::openai::{ChatCompletionRequest, ResponsesRequest};
use deepseek_recipe::request::{
    ConversationRequest, ConversionOptions, ModelVersion, ProtocolRequest,
};
use deepseek_recipe_encoding::PromptEncoding;
use deepseek_recipe_encoding::v4::dsv4::DeepseekV4Encoding;
use deepseek_recipe_encoding::v4::dsv41::DeepseekV41Encoding;

/// Prompt tokens used for the `resolve_max_tokens` demonstration. A real
/// caller gets this number from the tokenizer that encoded the prompt.
const SAMPLE_PROMPT_TOKENS: u32 = 16;

/// Context limit passed to `resolve_max_tokens`.
const CONTEXT_LIMIT: u32 = 131_072;

const CHAT_COMPLETIONS: &str = r#"
{
  "model": "DeepSeek-V4.1-Flash",
  "messages": [
    { "role": "system", "content": "You are a helpful assistant." },
    { "role": "user", "content": "What's the weather in Beijing today?" }
  ]
}
"#;

const RESPONSES: &str = r#"
{
  "model": "DeepSeek-V4.1-Flash",
  "instructions": "You are a helpful assistant.",
  "input": "What's the weather in Beijing today?"
}
"#;

const MESSAGES: &str = r#"
{
  "model": "DeepSeek-V4.1-Flash",
  "system": "You are a helpful assistant.",
  "messages": [
    { "role": "user", "content": "What's the weather in Beijing today?" }
  ]
}
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let selection = std::env::args().nth(1);
    let selection = selection.as_deref().unwrap_or("all");

    match selection {
        "all" => {
            run::<ChatCompletionRequest>("Chat Completions", CHAT_COMPLETIONS)?;
            run::<ResponsesRequest>("Responses", RESPONSES)?;
            run::<MessagesRequest>("Messages", MESSAGES)?;
        }
        "chat" | "chat_completions" => {
            run::<ChatCompletionRequest>("Chat Completions", CHAT_COMPLETIONS)?;
        }
        "responses" => run::<ResponsesRequest>("Responses", RESPONSES)?,
        "messages" => run::<MessagesRequest>("Messages", MESSAGES)?,
        other => {
            eprintln!(
                "unknown protocol: {other}\n\
                 usage: protocol-conversion-example [all|chat|responses|messages]"
            );
            std::process::exit(2);
        }
    }

    Ok(())
}

/// Deserialize one protocol request, convert it, and print the result.
fn run<T>(label: &str, body: &str) -> Result<(), Box<dyn std::error::Error>>
where
    T: ProtocolRequest + serde::de::DeserializeOwned,
{
    let request: T = serde_json::from_str(body)?;
    let converted = request.convert(ConversionOptions::default())?;
    report(label, &converted);
    Ok(())
}

/// Print the resolved conversation and the prompt rendered from it.
fn report(label: &str, converted: &ConversationRequest) {
    println!("=== {label} ===");
    println!("model version : {:?}", converted.model_version);
    println!("thinking mode : {}", converted.conversation.thinking_mode);
    println!("messages      : {}", converted.conversation.messages.len());
    println!("tools         : {}", converted.conversation.tools.len());
    println!(
        "response fmt  : {:?}",
        converted.conversation.response_format
    );
    println!("stream        : {}", converted.stream);
    println!(
        "temperature   : {:?}",
        converted.inference_options.temperature
    );
    println!("top_p         : {:?}", converted.inference_options.top_p);
    let max_tokens = match converted.inference_options.max_tokens {
        Some(value) => value.to_string(),
        None => "unset".to_owned(),
    };
    println!("max_tokens    : {max_tokens}");

    // The prompt encoding follows the model version resolved from the request.
    let rendered = match converted.model_version {
        ModelVersion::DeepseekV4Pro => {
            DeepseekV4Encoding::new().render_conversation(&converted.conversation)
        }
        _ => DeepseekV41Encoding::new().render_conversation(&converted.conversation),
    };

    println!(
        "prompt        : {} bytes, {} special tokens, {} images",
        rendered.prompt.len(),
        rendered.special_ranges.len(),
        rendered.image_sources.len()
    );

    // An unset max_tokens needs the encoded prompt length. This example uses a
    // sample count instead of attaching a tokenizer.
    match converted.resolve_max_tokens(SAMPLE_PROMPT_TOKENS, CONTEXT_LIMIT) {
        Ok(max_tokens) => println!(
            "resolved      : max_tokens = {max_tokens} at {SAMPLE_PROMPT_TOKENS} prompt tokens"
        ),
        Err(error) => println!("resolved      : {error}"),
    }

    println!("\n--- rendered prompt ---");
    println!("{}", rendered.prompt);
    println!("-----------------------\n");
}
