//! Privacy router for one feature at a time.
//!
//! The chain is platform, then a bundled local model, then cloud. Cloud is
//! skipped unless the feature's class is cloud-allowed, and encrypted mail
//! forces local-only before that check. Nothing here opens a socket.

use crate::{Error, Feature, ModelCapability, ModelKind, PrivacyClass, effective_privacy};

/// How large a feature may be, and where it may run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeaturePolicy {
    /// Feature this row governs.
    pub feature: Feature,
    /// Account or feature class, before the encrypted-mail override.
    pub class: PrivacyClass,
    /// Largest prompt, in tokens, this feature may send.
    pub budget_tokens: u32,
}

/// One routing decision's inputs. The model list is borrowed from the caller.
#[derive(Debug, Clone, Copy)]
pub struct RouteRequest<'a> {
    /// Feature being served.
    pub feature: Feature,
    /// The message is end-to-end encrypted.
    pub encrypted: bool,
    /// Tokens the prompt will spend. Compared with the policy and the window.
    pub needed_tokens: u32,
    /// Models the host says are installed. Order inside one kind does not matter
    /// beyond "first match".
    pub models: &'a [ModelCapability],
}

/// Per-feature policies. The first row for a feature wins so a caller can put
/// an override in front of the default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Router {
    policies: Vec<FeaturePolicy>,
}

impl Router {
    /// Builds a router from `policies`. An empty list fails closed on every call.
    pub fn new(policies: impl IntoIterator<Item = FeaturePolicy>) -> Self {
        Self {
            policies: policies.into_iter().collect(),
        }
    }

    /// The policy for `feature`, if one was supplied.
    pub fn policy(&self, feature: Feature) -> Option<&FeaturePolicy> {
        self.policies
            .iter()
            .find(|policy| policy.feature == feature)
    }

    /// Picks a model: budget, then the capability probe, then the fallback chain.
    ///
    /// # Errors
    ///
    /// [`Error::NoPolicy`] when `feature` has no row.
    /// [`Error::OverBudget`] when the prompt is larger than that row's budget.
    /// [`Error::NoProvider`] when no model on the chain can serve it. Encrypted
    /// mail never returns a cloud model; it returns [`Error::NoProvider`] instead.
    pub fn route<'a>(&self, request: &RouteRequest<'a>) -> Result<&'a ModelCapability, Error> {
        let policy = self.policy(request.feature).ok_or(Error::NoPolicy)?;
        if request.needed_tokens > policy.budget_tokens {
            return Err(Error::OverBudget);
        }
        let class = effective_privacy(policy.class, request.encrypted);
        for kind in [ModelKind::Platform, ModelKind::Local, ModelKind::Cloud] {
            // Cloud is the last step and only when the class still allows it.
            if kind == ModelKind::Cloud && class != PrivacyClass::CloudAllowed {
                continue;
            }
            if let Some(model) = request.models.iter().find(|model| {
                model.kind == kind && probe(model, request.feature, request.needed_tokens)
            }) {
                return Ok(model);
            }
        }
        Err(Error::NoProvider)
    }
}

/// A model can serve `feature` when it lists the feature and its window, if
/// known, covers `needed_tokens`. An unknown window is not treated as zero:
/// the host did not report a limit, and the budget already capped the prompt.
pub fn probe(model: &ModelCapability, feature: Feature, needed_tokens: u32) -> bool {
    model.features.contains(&feature)
        && model
            .context_tokens
            .is_none_or(|window| window >= needed_tokens)
}

#[cfg(test)]
mod tests {
    use super::{FeaturePolicy, RouteRequest, Router, probe};
    use crate::{Error, Feature, ModelCapability, ModelKind, PrivacyClass};

    fn model(id: &str, kind: ModelKind, window: Option<u32>) -> ModelCapability {
        ModelCapability {
            id: id.into(),
            kind,
            context_tokens: window,
            features: vec![Feature::Summarize],
        }
    }

    fn policy(class: PrivacyClass, budget_tokens: u32) -> FeaturePolicy {
        FeaturePolicy {
            feature: Feature::Summarize,
            class,
            budget_tokens,
        }
    }

    fn request<'a>(
        models: &'a [ModelCapability],
        encrypted: bool,
        needed_tokens: u32,
    ) -> RouteRequest<'a> {
        RouteRequest {
            feature: Feature::Summarize,
            encrypted,
            needed_tokens,
            models,
        }
    }

    #[test]
    fn encrypted_mail_never_selects_a_cloud_model() {
        let models = vec![
            model("cloud", ModelKind::Cloud, Some(8192)),
            model("local", ModelKind::Local, Some(2048)),
        ];
        let router = Router::new([policy(PrivacyClass::CloudAllowed, 4096)]);
        let chosen = router.route(&request(&models, true, 100)).unwrap();
        assert_eq!(chosen.id, "local");
        assert_ne!(chosen.kind, ModelKind::Cloud);

        let cloud_only = vec![model("cloud", ModelKind::Cloud, Some(8192))];
        assert!(matches!(
            router.route(&request(&cloud_only, true, 100)),
            Err(Error::NoProvider)
        ));
    }

    #[test]
    fn fallback_prefers_platform_then_local_then_cloud() {
        let models = vec![
            model("cloud", ModelKind::Cloud, Some(8192)),
            model("local", ModelKind::Local, Some(2048)),
            model("platform", ModelKind::Platform, Some(1024)),
        ];
        let router = Router::new([policy(PrivacyClass::CloudAllowed, 4096)]);
        let chosen = router.route(&request(&models, false, 100)).unwrap();
        assert_eq!(chosen.id, "platform");

        let without_platform = &models[..2];
        let chosen = router
            .route(&request(without_platform, false, 100))
            .unwrap();
        assert_eq!(chosen.id, "local");

        let cloud_only = &models[..1];
        let chosen = router.route(&request(cloud_only, false, 100)).unwrap();
        assert_eq!(chosen.kind, ModelKind::Cloud);
    }

    #[test]
    fn a_small_window_falls_through_and_a_budget_stops_the_call() {
        let models = vec![
            model("platform", ModelKind::Platform, Some(50)),
            model("local", ModelKind::Local, Some(500)),
        ];
        let router = Router::new([policy(PrivacyClass::LocalPreferred, 200)]);
        let chosen = router.route(&request(&models, false, 100)).unwrap();
        assert_eq!(chosen.id, "local");
        assert!(!probe(&models[0], Feature::Summarize, 100));
        assert!(matches!(
            router.route(&request(&models, false, 201)),
            Err(Error::OverBudget)
        ));
    }

    #[test]
    fn local_preferred_does_not_fall_through_to_cloud() {
        let models = vec![model("cloud", ModelKind::Cloud, None)];
        let router = Router::new([policy(PrivacyClass::LocalPreferred, 100)]);
        assert!(matches!(
            router.route(&request(&models, false, 10)),
            Err(Error::NoProvider)
        ));
        assert!(matches!(
            Router::new([]).route(&request(&models, false, 10)),
            Err(Error::NoPolicy)
        ));
    }

    #[test]
    fn a_model_that_lacks_the_feature_is_not_probed_in() {
        let mut draft = model("local", ModelKind::Local, Some(1000));
        draft.features = vec![Feature::DraftReply];
        assert!(!probe(&draft, Feature::Summarize, 10));
        let router = Router::new([policy(PrivacyClass::LocalOnly, 100)]);
        let models = vec![draft];
        assert!(matches!(
            router.route(&request(&models, false, 10)),
            Err(Error::NoProvider)
        ));
    }
}
