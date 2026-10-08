//! Maps a host [`PlatformModel`] into the router types.
//!
//! [`PlatformBridge::complete`] awaits the host future the caller already
//! polls. It does not open a socket and it does not know a cloud provider.

use mailune_protocol::{ModelCapability as PlatformCapability, ModelPrompt, PlatformModel};

use crate::{Completion, Error, Feature, ModelCapability, ModelKind, Prompt, Provider};

/// On-device model behind the [`Provider`] trait.
pub struct PlatformBridge<M> {
    model: M,
}

impl<M> PlatformBridge<M> {
    /// Wraps a host model. The bridge does not start it.
    pub fn new(model: M) -> Self {
        Self { model }
    }

    /// The host model, for a caller that still needs the trait object it passed in.
    pub fn model(&self) -> &M {
        &self.model
    }
}

/// Copies a host capability into a router capability.
///
/// The platform model is on the device, so the kind is [`ModelKind::Platform`].
/// Summarize and draft reply are the features this bridge will ask it for.
pub fn map_capability(capability: PlatformCapability) -> ModelCapability {
    ModelCapability {
        id: capability.id,
        kind: ModelKind::Platform,
        context_tokens: capability.context_tokens,
        features: vec![Feature::Summarize, Feature::DraftReply],
    }
}

impl<M> Provider for PlatformBridge<M>
where
    M: PlatformModel,
{
    fn name(&self) -> &str {
        "platform"
    }

    fn capabilities(&self) -> Vec<ModelCapability> {
        self.model
            .capabilities()
            .into_iter()
            .map(map_capability)
            .collect()
    }

    async fn complete(&self, prompt: &Prompt) -> Result<Completion, Error> {
        // A fixed cap: the host should not be asked for an unbounded completion.
        let request = ModelPrompt {
            text: prompt.text.clone(),
            max_output_tokens: 256,
        };
        let text = self.model.complete(&request).await?;
        Ok(Completion::Text(text))
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use mailune_protocol::{ModelCapability as PlatformCapability, ModelPrompt, PlatformModel};

    use super::{PlatformBridge, map_capability};
    use crate::{Completion, Feature, ModelKind, PrivacyClass, Prompt, Provider};

    struct Ready {
        reply: String,
    }

    impl PlatformModel for Ready {
        fn capabilities(&self) -> Vec<PlatformCapability> {
            vec![PlatformCapability {
                id: "foundation".into(),
                context_tokens: Some(4096),
            }]
        }

        async fn complete(&self, prompt: &ModelPrompt) -> Result<String, mailune_protocol::Error> {
            assert_eq!(prompt.max_output_tokens, 256);
            Ok(self.reply.clone())
        }
    }

    fn drive<T>(future: impl Future<Output = T>) -> T {
        let mut future = pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("platform model waited"),
        }
    }

    #[test]
    fn capabilities_and_completion_map_into_router_types() {
        let mapped = map_capability(PlatformCapability {
            id: "foundation".into(),
            context_tokens: Some(4096),
        });
        assert_eq!(mapped.kind, ModelKind::Platform);
        assert_eq!(mapped.context_tokens, Some(4096));
        assert!(mapped.features.contains(&Feature::Summarize));

        let bridge = PlatformBridge::new(Ready {
            reply: "done".into(),
        });
        assert_eq!(bridge.name(), "platform");
        assert_eq!(bridge.capabilities()[0].id, "foundation");
        let prompt = Prompt {
            feature: Feature::Summarize,
            text: "mail".into(),
            privacy: PrivacyClass::LocalOnly,
            encrypted: true,
        };
        let Completion::Text(text) = drive(bridge.complete(&prompt)).unwrap();
        assert_eq!(text, "done");
        assert_eq!(bridge.model().reply, "done");
    }
}
