//! Push relay. Gmail (Cloud Pub/Sub push) and Microsoft Graph change
//! notifications come in on per-registration webhook URLs; an empty wake
//! goes out to the device, which then syncs with its own credentials.
//!
//! The relay holds only channel-to-device routes. It never holds an OAuth
//! token or any mail, and refuses a notice that carries either. The device
//! side, [`RelayClient`], registers those routes and keeps which account
//! each channel belongs to.

mod client;
mod relay;
mod screen;

pub use client::{CHANNEL_ENTROPY, Error, Registration, RelayClient};
pub use relay::{DeviceHandle, MemoryRegistry, Notifier, Registry, router};
pub use screen::{MAX_BODY, Provider, Refusal, screen};
