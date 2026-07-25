use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::{
    LlmProvider, SHARE_RESPONSE_MAX_TOKENS, SUMMARY_SYSTEM_INSTRUCTION, SummaryError,
    SummaryRequest, build_summary_input, summary_output,
};

const OPENAI_URL: &str = "https://api.openai.com/v1/responses";
const DEEPSEEK_URL: &str = "https://api.deepseek.com/chat/completions";
const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const HTTP_TOTAL_TIMEOUT: Duration = Duration::from_secs(180);
const MAX_CONTINUATIONS: usize = 2;
const CONTINUE_PROMPT: &str = "Continue exactly where the previous response stopped. Do not repeat completed sections. Finish every remaining required report section.";

pub(super) fn summarize(request: &SummaryRequest) -> Result<String, SummaryError> {
    let client = Client::builder()
        .connect_timeout(HTTP_CONNECT_TIMEOUT)
        .timeout(HTTP_TOTAL_TIMEOUT)
        .build()?;
    match request.provider {
        LlmProvider::OpenAi => summarize_openai(&client, request),
        LlmProvider::DeepSeek => summarize_deepseek(&client, request),
    }
}

fn summarize_openai(client: &Client, request: &SummaryRequest) -> Result<String, SummaryError> {
    let response = client
        .post(OPENAI_URL)
        .bearer_auth(&request.api_key)
        .json(&OpenAiRequest {
            model: &request.model,
            instructions: SUMMARY_SYSTEM_INSTRUCTION,
            input: &build_summary_input(&request.report, request.audience),
        })
        .send()
        .map_err(SummaryError::Http)
        .and_then(error_for_status_with_body)?;
    let body = response.text()?;
    let response = serde_json::from_str::<OpenAiResponse>(&body)
        .map_err(|_| SummaryError::InvalidJson { body })?;
    response
        .output_text
        .map(summary_output)
        .transpose()?
        .ok_or(SummaryError::MissingOutput)
}

fn summarize_deepseek(client: &Client, request: &SummaryRequest) -> Result<String, SummaryError> {
    let input = build_summary_input(&request.report, request.audience);
    let mut output = String::new();
    let mut previous_response: Option<String> = None;
    for attempt in 0..=MAX_CONTINUATIONS {
        let continue_prompt = if attempt == 0 {
            None
        } else {
            Some(CONTINUE_PROMPT)
        };
        let choice = send_deepseek_chat(
            client,
            request,
            &input,
            previous_response.as_deref(),
            continue_prompt,
        )?;
        let content = summary_output(choice.message.content)?;
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        output.push_str(&content);
        if choice.finish_reason.as_deref() != Some("length") {
            return Ok(output);
        }
        previous_response = Some(content);
    }
    Err(SummaryError::IncompleteOutput)
}

fn send_deepseek_chat(
    client: &Client,
    request: &SummaryRequest,
    input: &str,
    previous_response: Option<&str>,
    continue_prompt: Option<&str>,
) -> Result<ChatChoice, SummaryError> {
    let messages = chat_messages(input, previous_response, continue_prompt);
    let response = client
        .post(DEEPSEEK_URL)
        .bearer_auth(&request.api_key)
        .json(&ChatRequest {
            model: &request.model,
            messages,
            stream: false,
            max_tokens: SHARE_RESPONSE_MAX_TOKENS,
            temperature: 0.2,
        })
        .send()
        .map_err(SummaryError::Http)
        .and_then(error_for_status_with_body)?;
    let body = response.text()?;
    let response = serde_json::from_str::<ChatResponse>(&body)
        .map_err(|_| SummaryError::InvalidJson { body })?;
    response
        .choices
        .into_iter()
        .next()
        .ok_or(SummaryError::MissingOutput)
}

fn chat_messages<'a>(
    input: &'a str,
    previous_response: Option<&'a str>,
    continue_prompt: Option<&'a str>,
) -> Vec<ChatMessage<'a>> {
    let mut messages = vec![
        ChatMessage {
            role: "system",
            content: SUMMARY_SYSTEM_INSTRUCTION,
        },
        ChatMessage {
            role: "user",
            content: input,
        },
    ];
    if let (Some(previous_response), Some(continue_prompt)) = (previous_response, continue_prompt) {
        messages.push(ChatMessage {
            role: "assistant",
            content: previous_response,
        });
        messages.push(ChatMessage {
            role: "user",
            content: continue_prompt,
        });
    }
    messages
}

#[derive(Debug, Serialize)]
struct OpenAiRequest<'a> {
    model: &'a str,
    instructions: &'a str,
    input: &'a str,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    output_text: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
    max_tokens: u16,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceMessage {
    content: String,
}

fn error_for_status_with_body(response: Response) -> Result<Response, SummaryError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text()?;
    Err(SummaryError::ProviderStatus { status, body })
}

#[cfg(test)]
mod tests {
    use super::{CONTINUE_PROMPT, chat_messages};

    #[test]
    fn continuation_messages_include_previous_partial_answer() {
        let messages = chat_messages(
            "source context",
            Some("partial report"),
            Some(CONTINUE_PROMPT),
        );

        assert_eq!(messages.len(), 4);
        assert_eq!(messages[2].role, "assistant");
        assert_eq!(messages[2].content, "partial report");
        assert_eq!(messages[3].role, "user");
        assert!(messages[3].content.contains("Continue exactly where"));
    }
}
