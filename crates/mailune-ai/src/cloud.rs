//! Bring-your-own-key request and response shapes. No HTTP.
//!
//! [`build_request`] asks the privacy rule first, so encrypted or
//! local-only mail never becomes a request, and redacts quoted text and the
//! signature. The key is not part of the shape: [`CloudRequest::auth`] names
//! the header and the HTTP adapter fills it from the keychain.
//! The shared llm-* crates are not published yet, so the two wire formats
//! live here until they are.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::{Completion, Error, Prompt, allow_cloud, redact_for_cloud};

/// Which wire format a bring-your-own-key provider speaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloudApi {
    /// `POST {base}/chat/completions` (OpenAI and compatible servers).
    OpenAiCompatible {
        /// Base URL, such as `https://api.openai.com/v1`.
        base_url: String,
    },
    /// `POST https://api.anthropic.com/v1/messages`.
    Anthropic,
}

/// How the adapter attaches the person's key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyHeader {
    /// `Authorization: Bearer <key>`.
    Bearer,
    /// `x-api-key: <key>`.
    XApiKey,
}

/// One request, ready for an HTTP adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct CloudRequest {
    /// Endpoint.
    pub url: String,
    /// Non-secret headers.
    pub headers: Vec<(&'static str, &'static str)>,
    /// Where the key goes.
    pub auth: KeyHeader,
    /// JSON body.
    pub body: Value,
    /// Bytes of mail text in the body, for the ledger.
    pub bytes: u64,
}

const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Builds a request for `prompt` on `model`.
///
/// # Errors
///
/// [`Error::CloudForbidden`] unless the prompt's effective class is
/// cloud-allowed; encrypted mail never passes.
pub fn build_request(
    api: &CloudApi,
    model: &str,
    prompt: &Prompt,
    max_tokens: u32,
) -> Result<CloudRequest, Error> {
    allow_cloud(prompt)?;
    let text = redact_for_cloud(&prompt.text);
    let bytes = u64::try_from(text.len()).unwrap_or(u64::MAX);
    let message = json!([{ "role": "user", "content": text }]);
    Ok(match api {
        CloudApi::OpenAiCompatible { base_url } => CloudRequest {
            url: format!("{}/chat/completions", base_url.trim_end_matches('/')),
            headers: vec![("content-type", "application/json")],
            auth: KeyHeader::Bearer,
            body: json!({ "model": model, "max_tokens": max_tokens, "messages": message }),
            bytes,
        },
        CloudApi::Anthropic => CloudRequest {
            url: ANTHROPIC_URL.into(),
            headers: vec![
                ("content-type", "application/json"),
                ("anthropic-version", ANTHROPIC_VERSION),
            ],
            auth: KeyHeader::XApiKey,
            body: json!({ "model": model, "max_tokens": max_tokens, "messages": message }),
            bytes,
        },
    })
}

#[derive(Deserialize)]
struct OpenAiReply {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicReply {
    content: Vec<AnthropicBlock>,
}

#[derive(Deserialize)]
struct AnthropicBlock {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct ErrorReply {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    #[serde(rename = "type")]
    kind: Option<String>,
    code: Option<Value>,
}

/// Turns a response body into a typed completion. The text stays untrusted.
///
/// # Errors
///
/// [`Error::Cloud`] with the provider's error type for an error body, and
/// [`Error::BadOutput`] for a body that is neither shape or has no text.
pub fn parse_response(api: &CloudApi, body: &str) -> Result<Completion, Error> {
    if let Ok(reply) = serde_json::from_str::<ErrorReply>(body) {
        // Only the error type is kept: the message may quote the prompt back.
        let kind = reply
            .error
            .kind
            .or_else(|| reply.error.code.map(|code| code.to_string()))
            .unwrap_or_else(|| "unknown".into());
        return Err(Error::Cloud(kind));
    }
    let text = match api {
        CloudApi::OpenAiCompatible { .. } => serde_json::from_str::<OpenAiReply>(body)
            .ok()
            .and_then(|reply| reply.choices.into_iter().next())
            .and_then(|choice| choice.message.content),
        CloudApi::Anthropic => serde_json::from_str::<AnthropicReply>(body)
            .ok()
            .map(|reply| {
                reply
                    .content
                    .into_iter()
                    .filter(|block| block.kind == "text")
                    .filter_map(|block| block.text)
                    .collect::<String>()
            })
            .filter(|text| !text.is_empty()),
    };
    text.map(Completion::Text).ok_or(Error::BadOutput)
}

#[cfg(test)]
mod tests {
    use super::{CloudApi, KeyHeader, build_request, parse_response};
    use crate::{Completion, Error, Feature, PrivacyClass, Prompt};

    fn prompt(privacy: PrivacyClass, encrypted: bool) -> Prompt {
        Prompt {
            feature: Feature::Summarize,
            text: "Summarize.\n\nShip Friday.\n> quoted secret\n-- \nAda".into(),
            privacy,
            encrypted,
        }
    }

    #[test]
    fn openai_and_anthropic_requests_are_built_from_a_prompt() {
        let openai = CloudApi::OpenAiCompatible {
            base_url: "https://api.example/v1/".into(),
        };
        let request = build_request(
            &openai,
            "gpt-x",
            &prompt(PrivacyClass::CloudAllowed, false),
            200,
        )
        .unwrap();
        assert_eq!(request.url, "https://api.example/v1/chat/completions");
        assert_eq!(request.auth, KeyHeader::Bearer);
        assert_eq!(request.body["model"], "gpt-x");
        assert_eq!(request.body["max_tokens"], 200);
        let sent = request.body["messages"][0]["content"].as_str().unwrap();
        assert_eq!(sent, "Summarize.\n\nShip Friday.");
        assert_eq!(request.bytes, sent.len() as u64);

        let request = build_request(
            &CloudApi::Anthropic,
            "claude-x",
            &prompt(PrivacyClass::CloudAllowed, false),
            200,
        )
        .unwrap();
        assert_eq!(request.url, "https://api.anthropic.com/v1/messages");
        assert_eq!(request.auth, KeyHeader::XApiKey);
        assert!(
            request
                .headers
                .contains(&("anthropic-version", "2023-06-01"))
        );
        assert!(!request.body.to_string().contains("quoted secret"));
    }

    #[test]
    fn local_and_encrypted_mail_never_becomes_a_request() {
        for (class, encrypted) in [
            (PrivacyClass::CloudAllowed, true),
            (PrivacyClass::LocalPreferred, false),
            (PrivacyClass::LocalOnly, false),
        ] {
            assert!(matches!(
                build_request(&CloudApi::Anthropic, "m", &prompt(class, encrypted), 10),
                Err(Error::CloudForbidden)
            ));
        }
    }

    #[test]
    fn scripted_responses_become_typed_results() {
        let openai = CloudApi::OpenAiCompatible {
            base_url: "https://api.example/v1".into(),
        };
        let reply = r#"{"id":"x","choices":[{"index":0,"message":{"role":"assistant","content":"Ship Friday."}}]}"#;
        assert_eq!(
            parse_response(&openai, reply).unwrap(),
            Completion::Text("Ship Friday.".into())
        );
        let reply = r#"{"content":[{"type":"thinking","thinking":"..."},{"type":"text","text":"Ship "},{"type":"text","text":"Friday."}],"stop_reason":"end_turn"}"#;
        assert_eq!(
            parse_response(&CloudApi::Anthropic, reply).unwrap(),
            Completion::Text("Ship Friday.".into())
        );
        let error =
            r#"{"type":"error","error":{"type":"rate_limit_error","message":"echo of prompt"}}"#;
        let err = parse_response(&CloudApi::Anthropic, error).unwrap_err();
        assert!(matches!(&err, Error::Cloud(kind) if kind == "rate_limit_error"));
        assert!(!err.to_string().contains("echo"));
        assert!(matches!(
            parse_response(&openai, r#"{"choices":[]}"#),
            Err(Error::BadOutput)
        ));
        assert!(matches!(
            parse_response(&CloudApi::Anthropic, "not json"),
            Err(Error::BadOutput)
        ));
    }
}
