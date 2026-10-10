//! The device side of the relay. Each account gets a channel drawn from
//! host randomness; the device registers that channel with the relay and
//! hands the webhook URL to the provider. When the empty wake arrives, the
//! accounts registered here become due for sync. The account behind a
//! channel stays on the device: the relay learns a channel and a device
//! handle, nothing else.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use mailune_protocol::{AccountId, Http, HttpRequest, Method};
use serde_json::json;

use crate::{DeviceHandle, Provider};

/// Bytes of host randomness behind one channel id.
pub const CHANNEL_ENTROPY: usize = 16;

/// Failure returned by the relay client. No variant carries the channel.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The transport did not return a response.
    #[error(transparent)]
    Transport(#[from] mailune_protocol::Error),
    /// The relay already routes this channel. Draw new randomness and retry.
    #[error("relay channel is taken")]
    ChannelTaken,
    /// The relay refused the registration.
    #[error("relay answered {status}")]
    Status {
        /// HTTP status code.
        status: u16,
    },
}

/// One account the relay wakes this device for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    /// Account to sync on a wake.
    pub account: AccountId,
    /// Provider that will call the webhook.
    pub provider: Provider,
    /// URL for the provider's subscription: the Gmail Pub/Sub push endpoint
    /// or the Graph `notificationUrl`. The channel in it is its only secret.
    pub webhook_url: String,
}

/// Registers accounts with one relay and tracks which are due for sync.
#[derive(Debug)]
pub struct RelayClient {
    relay_url: String,
    device: DeviceHandle,
    registrations: Vec<Registration>,
    due: Vec<AccountId>,
}

impl RelayClient {
    /// A client for the relay at `relay_url` that registers `device`, the
    /// handle the platform push service gave this device.
    pub fn new(relay_url: &str, device: DeviceHandle) -> Self {
        Self {
            relay_url: relay_url.trim_end_matches('/').to_string(),
            device,
            registrations: Vec::new(),
            due: Vec::new(),
        }
    }

    /// Registers `account` under a channel made from `entropy`, which must
    /// come from the host's secure random source.
    ///
    /// # Errors
    ///
    /// [`Error::ChannelTaken`] when the relay already routes that channel,
    /// [`Error::Status`] when it refuses, and transport errors.
    pub async fn register<H: Http>(
        &mut self,
        http: &H,
        account: AccountId,
        provider: Provider,
        entropy: [u8; CHANNEL_ENTROPY],
    ) -> Result<Registration, Error> {
        let channel = URL_SAFE_NO_PAD.encode(entropy);
        let body =
            json!({ "provider": provider.path(), "channel": channel, "device": self.device.0 });
        let request = HttpRequest::new(Method::Post, format!("{}/register", self.relay_url))
            .json(body.to_string());
        match http.send(request).await?.status {
            201 => {}
            409 => return Err(Error::ChannelTaken),
            status => return Err(Error::Status { status }),
        }
        let registration = Registration {
            account,
            provider,
            webhook_url: format!("{}/hook/{}/{channel}", self.relay_url, provider.path()),
        };
        self.registrations.push(registration.clone());
        Ok(registration)
    }

    /// The platform delivered an empty wake from the relay. The wake does
    /// not say which account changed, because the push service would see
    /// it too, so every account registered here becomes due.
    pub fn wake(&mut self) {
        for registration in &self.registrations {
            if !self.due.contains(&registration.account) {
                self.due.push(registration.account.clone());
            }
        }
    }

    /// Accounts due since the last call, in registration order, for the
    /// sync scheduler. The list is empty afterwards.
    pub fn take_due(&mut self) -> Vec<AccountId> {
        std::mem::take(&mut self.due)
    }

    /// Accounts registered through this client, oldest first.
    pub fn registrations(&self) -> &[Registration] {
        &self.registrations
    }

    /// The handle this client registers.
    pub fn device(&self) -> &DeviceHandle {
        &self.device
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use mailune_protocol::{AccountId, Http, HttpRequest, HttpResponse};
    use serde_json::json;
    use tower::ServiceExt;

    use super::{Error, RelayClient};
    use crate::{DeviceHandle, MemoryRegistry, Notifier, Provider, router};

    const RELAY: &str = "https://relay.example";

    #[derive(Clone, Default)]
    struct Recorder(Arc<Mutex<Vec<DeviceHandle>>>);

    impl Notifier for Recorder {
        fn wake(&self, device: &DeviceHandle) {
            self.0.lock().unwrap().push(device.clone());
        }
    }

    /// The relay's router behind the `Http` trait, called in process.
    struct InProcess(Router);

    impl Http for InProcess {
        async fn send(
            &self,
            request: HttpRequest,
        ) -> Result<HttpResponse, mailune_protocol::Error> {
            let path = request.url.strip_prefix(RELAY).unwrap().to_string();
            let request = Request::post(path).body(Body::from(request.body)).unwrap();
            let response = self.0.clone().oneshot(request).await.unwrap();
            let status = response.status().as_u16();
            let body = to_bytes(response.into_body(), 4096).await.unwrap();
            Ok(HttpResponse::new(status, body.to_vec()))
        }
    }

    #[tokio::test]
    async fn a_registered_account_is_due_after_a_webhook_wakes_the_device() {
        let recorder = Recorder::default();
        let relay = InProcess(router(MemoryRegistry::default(), recorder.clone()));
        let mut client = RelayClient::new("https://relay.example/", DeviceHandle("apns-1".into()));
        let work = AccountId::new("work");
        let registration = client
            .register(&relay, work.clone(), Provider::Graph, [7; 16])
            .await
            .unwrap();
        assert_eq!(
            registration.webhook_url,
            "https://relay.example/hook/graph/BwcHBwcHBwcHBwcHBwcHBw"
        );
        assert!(matches!(
            client
                .register(&relay, AccountId::new("home"), Provider::Graph, [7; 16])
                .await,
            Err(Error::ChannelTaken)
        ));
        assert_eq!(client.registrations().len(), 1);
        assert!(client.take_due().is_empty());

        let notice = json!({ "value": [{ "subscriptionId": "s", "changeType": "created" }] });
        let webhook = HttpRequest::new(mailune_protocol::Method::Post, &registration.webhook_url)
            .json(notice.to_string());
        assert_eq!(relay.send(webhook).await.unwrap().status, 202);

        // The platform push service hands the empty wake to the device.
        let woken = recorder.0.lock().unwrap().clone();
        assert_eq!(woken, [client.device().clone()]);
        client.wake();
        client.wake();
        assert_eq!(client.take_due(), [work]);
        assert!(client.take_due().is_empty());
    }

    #[tokio::test]
    async fn a_refused_registration_is_not_kept() {
        let relay = InProcess(router(MemoryRegistry::default(), Recorder::default()));
        let mut client = RelayClient::new(RELAY, DeviceHandle("Bearer abc".into()));
        assert!(matches!(
            client
                .register(&relay, AccountId::new("a"), Provider::Gmail, [1; 16])
                .await,
            Err(Error::Status { status: 422 })
        ));
        client.wake();
        assert!(client.registrations().is_empty());
        assert!(client.take_due().is_empty());
    }
}
