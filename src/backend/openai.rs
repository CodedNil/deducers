use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{env, fs::OpenOptions, io::Write, time::Duration};

const GPT_MODEL: &str = "gpt-4-1106-preview";

// ---------- Request Payload ----------
// Represents the main structure for the API request payload.
#[derive(Deserialize, Serialize)]
struct RequestBody {
    model: String,
    max_tokens: usize,
    temperature: f32,
    messages: Vec<Message>,
    response_format: Option<ResponseFormat>,
}

// Represents individual messages in the request.
#[derive(Deserialize, Serialize)]
struct Message {
    role: String,
    content: String,
}

// Represents the response format in the request.
#[derive(Deserialize, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    response_type: String,
}

// ---------- API Response ----------
// Represents the expected response format from the API.
#[derive(Deserialize, Serialize, Debug)]
struct ApiResponse {
    choices: Vec<Choice>,
    usage: Usage,
}

// Represents individual messages in the request.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MessageResponse {
    content: String,
}

// Represents individual choices in the API response.
#[derive(Deserialize, Serialize, Debug)]
struct Choice {
    message: MessageResponse,
}

// Represents the token usage of a response.
#[derive(Deserialize, Serialize, Debug)]
struct Usage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

pub async fn query_ai(prompt: &String, max_tokens: usize, temperature: f32) -> Result<String> {
    let api_key = env::var("OPENAI_API_KEY").context("No OPENAI_API_KEY found in environment")?;

    let messages: Result<Vec<Message>> = prompt
        .split('|')
        .map(|message| {
            let parts: Vec<&str> = message.split(':').collect();
            let role: String = match parts.first() {
                Some(&"u") => Ok("user".into()),
                Some(&"s") => Ok("system".into()),
                Some(&"a") => Ok("assistant".into()),
                _ => Err(anyhow::anyhow!("Invalid role")),
            }?;
            let content: String = match parts.get(1) {
                Some(&content) => Ok(content.into()),
                _ => Err(anyhow::anyhow!("Invalid content")),
            }?;
            Ok(Message { role, content })
        })
        .collect();
    let messages = messages.context("Failed to parse messages")?;

    // Construct the request payload.
    let body = RequestBody {
        model: GPT_MODEL.into(),
        max_tokens,
        temperature,
        messages,
        response_format: Some(ResponseFormat {
            response_type: "json_object".into(),
        }),
    };
    let body_str = serde_json::to_string(&body).context("Failed to serialize the request body")?;

    // Execute the HTTP POST request to the OpenAI API.
    let client = reqwest::Client::new();
    let raw_response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {api_key}"))
        .timeout(Duration::from_secs(10))
        .body(body_str.to_string())
        .send()
        .await
        .context("Failed to make the HTTP request")?
        .text()
        .await
        .context("Failed to convert the response into a string")?;

    // Deserialize the response into our ApiResponse struct.
    let response = serde_json::from_str::<ApiResponse>(&raw_response).context("Failed to parse response into JSON")?;

    // Check if there's a choice in the response and extract the assistant's reply.
    if let Some(choice) = response.choices.first() {
        // Log the required details to a log file.
        log_details(prompt, &choice.message, &response.usage)?;

        return Ok(choice.message.content.clone());
    }
    Err(anyhow::anyhow!("Failed to extract message content from the response"))
}

#[allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]
fn log_details(prompt: &String, result: &MessageResponse, tokens: &Usage) -> Result<()> {
    // Pricing is input $0.0015 / 1K tokens output $0.002 / 1K tokens
    let price = ((tokens.prompt_tokens as f32 * 0.0015) + (tokens.completion_tokens as f32 * 0.002)) / 1000.0;

    // Format the log entry.
    let result = format!("{result:?}");
    let log_entry = format!(
        "Prompt: {:} | Result: {:} | Tokens: {}/{}/{} ${}\n",
        &prompt[..100.min(prompt.len())],
        &result[..100.min(result.len())],
        tokens.prompt_tokens,
        tokens.completion_tokens,
        tokens.total_tokens,
        price
    );

    // Open the log file in append mode.
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("gpt_log.txt")
        .context("Failed to open log file")?;

    // Write the log entry to the file.
    file.write_all(log_entry.as_bytes()).context("Failed to write to log file")?;

    Ok(())
}
