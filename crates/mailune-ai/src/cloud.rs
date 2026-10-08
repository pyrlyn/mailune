//! Request bodies for a bring-your-own-key call.
//!
//! The shapes follow the chat APIs those providers publish. A scripted body
//! becomes a [`Completion`]. Nothing here opens a socket. Encrypted mail never
//! becomes a cloud request: [`Router`] drops cloud unless the effective class
//! is cloud-allowed.

use std::fmt;

use serde::Deserialize;
use serde::Serialize;

use crate::{
    Completion, Error, ModelCapability, ModelKind, Prompt, RouteRequest, Router, allow_cloud,
};

/// What the router needs in order to allow a cloud request.
pub struct CloudCall<'a> {
    /// Policies, including the encrypted-mail override.
    pub router: &'a Router,
    /// Models the host says are installed.
    pub models: &'a [ModelCapability],
    /// The prompt. Its text may be mail, so it is not printed.
    pub prompt: &'a Prompt,
    /// Tokens the prompt will spend. Compared with the policy and the window.
    pub needed_tokens: u32,
}

/// One chat turn. `content` may be mail, so [`Debug`] redacts it.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct ChatTurn {
    /// `user` for a prompt we built.
    pub role: String,
    /// The prompt text. May contain mail.
    pub content: String,
}

impl fmt::Debug for ChatTurn {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ChatTurn")
            .field("role", &self.role)
            .field("content", &"redacted")
            .finish()
    }
}

/// An OpenAI-compatible chat request. The body may be mail, so [`Debug`] hides it.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct OpenAiRequest {
    /// Model id the router selected.
    pub model: String,
    /// The prompt as a user turn.
    pub messages: Vec<ChatTurn>,
}

impl fmt::Debug for OpenAiRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenAiRequest")
            .field("model", &self.model)
            .field("messages", &self.messages.len())
            .finish()
    }
}

/// An Anthropic messages request. The body may be mail, so [`Debug`] hides it.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct AnthropicRequest {
    /// Model id the router selected.
    pub model: String,
    /// Output cap. Anthropic rejects a body that omits this.
    pub max_tokens: u32,
    /// The prompt as a user turn.
    pub messages: Vec<ChatTurn>,
}

impl fmt::Debug for AnthropicRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AnthropicRequest")
            .field("model", &self.model)
            .field("max_tokens", &self.max_tokens)
            .field("messages", &self.messages.len())
            .finish()
    }
}

fn selected_cloud<'a>(call: &CloudCall<'a>) -> Result<&'a ModelCapability, Error> {
    let request = RouteRequest {
        feature: call.prompt.feature,
        encrypted: call.prompt.encrypted,
        needed_tokens: call.needed_tokens,
        models: call.models,
    };
    let model = call.router.route(&request)?;
    if model.kind != ModelKind::Cloud {
        return Err(Error::CloudForbidden);
    }
    // The router used its policy row. The prompt's own class still wins, so a
    // local-only prompt cannot ride a cloud-allowed policy.
    allow_cloud(call.prompt)?;
    Ok(model)
}

fn user_turn(text: &str) -> ChatTurn {
    ChatTurn {
        role: "user".to_owned(),
        content: text.to_owned(),
    }
}

/// Builds an OpenAI-compatible chat request from `call`.
///
/// # Errors
///
/// [`Error::NoPolicy`], [`Error::OverBudget`], or [`Error::NoProvider`] from
/// the router. [`Error::CloudForbidden`] when the selected model is not cloud
/// or the prompt itself forbids cloud, including every encrypted message.
pub fn openai_request(call: &CloudCall<'_>) -> Result<OpenAiRequest, Error> {
    let model = selected_cloud(call)?;
    Ok(OpenAiRequest {
        model: model.id.clone(),
        messages: vec![user_turn(&call.prompt.text)],
    })
}

/// Builds an Anthropic messages request from `call`.
///
/// `max_tokens` is the feature budget. Anthropic requires a positive cap, and
/// the budget is the only size the router already stored.
///
/// # Errors
///
/// Same as [`openai_request`]. [`Error::NoPolicy`] when the feature row
/// disappeared between the route and the budget read.
pub fn anthropic_request(call: &CloudCall<'_>) -> Result<AnthropicRequest, Error> {
    let model = selected_cloud(call)?;
    let max_tokens = call
        .router
        .policy(call.prompt.feature)
        .map(|policy| policy.budget_tokens.max(1))
        .ok_or(Error::NoPolicy)?;
    Ok(AnthropicRequest {
        model: model.id.clone(),
        max_tokens,
        messages: vec![user_turn(&call.prompt.text)],
    })
}

#[derive(Deserialize)]
struct OpenAiBody {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiMessage {
    content: String,
}

/// Turns a scripted OpenAI chat body into text.
///
/// # Errors
///
/// [`Error::BadResponse`] when the body is not that shape, or the text is empty.
pub fn completion_from_openai(body: &str) -> Result<Completion, Error> {
    let parsed: OpenAiBody = serde_json::from_str(body).map_err(|_| Error::BadResponse)?;
    let text = parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .filter(|content| !content.is_empty())
        .ok_or(Error::BadResponse)?;
    Ok(Completion::Text(text))
}

#[derive(Deserialize)]
struct AnthropicBody {
    content: Vec<AnthropicBlock>,
}

#[derive(Deserialize)]
struct AnthropicBlock {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

/// Turns a scripted Anthropic messages body into text.
///
/// # Errors
///
/// [`Error::BadResponse`] when the body is not that shape, or no text block is present.
pub fn completion_from_anthropic(body: &str) -> Result<Completion, Error> {
    let parsed: AnthropicBody = serde_json::from_str(body).map_err(|_| Error::BadResponse)?;
    let text = parsed
        .content
        .into_iter()
        .find(|block| block.kind == "text")
        .and_then(|block| block.text)
        .filter(|content| !content.is_empty())
        .ok_or(Error::BadResponse)?;
    Ok(Completion::Text(text))
}

#[cfg(test)]
mod tests {
    use super::{
        CloudCall, anthropic_request, completion_from_anthropic, completion_from_openai,
        openai_request,
    };
    use crate::{
        Completion, Error, Feature, FeaturePolicy, ModelCapability, ModelKind, PrivacyClass,
        Prompt, Router,
    };

    fn model(id: &str, kind: ModelKind) -> ModelCapability {
        ModelCapability {
            id: id.into(),
            kind,
            context_tokens: None,
            features: vec![Feature::Summarize],
        }
    }

    fn router(class: PrivacyClass) -> Router {
        Router::new([FeaturePolicy {
            feature: Feature::Summarize,
            class,
            budget_tokens: 128,
        }])
    }

    fn make_prompt(privacy: PrivacyClass, encrypted: bool) -> Prompt {
        Prompt {
            feature: Feature::Summarize,
            text: "secret body".into(),
            privacy,
            encrypted,
        }
    }

    fn make_call<'a>(
        router: &'a Router,
        models: &'a [ModelCapability],
        prompt: &'a Prompt,
    ) -> CloudCall<'a> {
        CloudCall {
            router,
            models,
            prompt,
            needed_tokens: 8,
        }
    }

    #[test]
    fn encrypted_mail_does_not_select_a_cloud_model() {
        let cloud = router(PrivacyClass::CloudAllowed);
        let only_cloud = vec![model("cloud-model", ModelKind::Cloud)];
        let encrypted = make_prompt(PrivacyClass::CloudAllowed, true);
        let call = make_call(&cloud, &only_cloud, &encrypted);
        assert!(matches!(openai_request(&call), Err(Error::NoProvider)));
        assert!(matches!(anthropic_request(&call), Err(Error::NoProvider)));

        let both = vec![
            model("cloud-model", ModelKind::Cloud),
            model("local-model", ModelKind::Local),
        ];
        let call = make_call(&cloud, &both, &encrypted);
        assert!(matches!(openai_request(&call), Err(Error::CloudForbidden)));
        assert!(matches!(
            anthropic_request(&call),
            Err(Error::CloudForbidden)
        ));
    }

    #[test]
    fn a_cloud_prompt_becomes_both_request_shapes() {
        let cloud = router(PrivacyClass::CloudAllowed);
        let models = vec![model("cloud-model", ModelKind::Cloud)];
        let prompt = make_prompt(PrivacyClass::CloudAllowed, false);
        let call = make_call(&cloud, &models, &prompt);
        let openai = openai_request(&call).unwrap();
        assert_eq!(openai.model, "cloud-model");
        assert_eq!(openai.messages[0].role, "user");
        assert_eq!(openai.messages[0].content, "secret body");
        let value = serde_json::to_value(&openai).unwrap();
        assert_eq!(value["model"], "cloud-model");
        assert_eq!(value["messages"][0]["content"], "secret body");
        assert!(!format!("{openai:?}").contains("secret body"));

        let anthropic = anthropic_request(&call).unwrap();
        assert_eq!(anthropic.model, "cloud-model");
        assert_eq!(anthropic.max_tokens, 128);
        assert_eq!(anthropic.messages[0].content, "secret body");
        let value = serde_json::to_value(&anthropic).unwrap();
        assert_eq!(value["max_tokens"], 128);
        assert!(!format!("{anthropic:?}").contains("secret body"));

        let preferred = router(PrivacyClass::LocalPreferred);
        let call = make_call(&preferred, &models, &prompt);
        assert!(matches!(openai_request(&call), Err(Error::NoProvider)));

        let local_only = make_prompt(PrivacyClass::LocalOnly, false);
        let call = make_call(&cloud, &models, &local_only);
        assert!(matches!(openai_request(&call), Err(Error::CloudForbidden)));
    }

    #[test]
    fn a_scripted_response_becomes_text() {
        let openai = r#"{"id":"x","choices":[{"message":{"role":"assistant","content":"noted"}}]}"#;
        assert_eq!(
            completion_from_openai(openai).unwrap(),
            Completion::Text("noted".into())
        );
        let anthropic = r#"{"content":[{"type":"text","text":"noted"}]}"#;
        assert_eq!(
            completion_from_anthropic(anthropic).unwrap(),
            Completion::Text("noted".into())
        );
        assert!(matches!(
            completion_from_openai("[]"),
            Err(Error::BadResponse)
        ));
        assert!(matches!(
            completion_from_anthropic("{}"),
            Err(Error::BadResponse)
        ));
        assert!(matches!(
            completion_from_openai(r#"{"choices":[]}"#),
            Err(Error::BadResponse)
        ));
    }
}
