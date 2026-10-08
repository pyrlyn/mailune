//! Which accounts may sync, and in what order.
//!
//! [`NetworkState`] reports offline and metered paths. It does not report
//! battery, so that flag is a separate input. The plan is a value: nothing
//! here sleeps or starts a thread.

use std::cmp::Reverse;

use mailune_protocol::{AccountId, NetworkPath, NetworkState};

/// Battery, which the network trait does not carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Power {
    /// The host says the device should avoid background work.
    pub low_battery: bool,
}

/// One account and how soon it should sync relative to the others.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncAccount {
    /// Account to refresh.
    pub account: AccountId,
    /// Higher runs first. Equal values keep the order they were given in.
    pub priority: u32,
}

/// Why no account runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hold {
    /// [`NetworkPath::Offline`].
    Offline,
    /// [`NetworkPath::Metered`].
    Metered,
    /// [`Power::low_battery`] on an otherwise usable path.
    LowBattery,
}

/// Accounts that may run now, highest priority first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPlan {
    /// Set when sync must wait. `order` is empty in that case.
    pub hold: Option<Hold>,
    /// Accounts to sync, highest priority first.
    pub order: Vec<AccountId>,
}

/// Plans sync from the host's current path.
pub fn plan(accounts: &[SyncAccount], network: &impl NetworkState, power: Power) -> SyncPlan {
    plan_for(accounts, network.path(), power)
}

/// Plans sync from a path the caller already read.
///
/// Network reasons win over battery when both apply: there is nothing to
/// fetch on an offline or metered path, so the battery flag is not the cause.
pub fn plan_for(accounts: &[SyncAccount], path: NetworkPath, power: Power) -> SyncPlan {
    let hold = match path {
        NetworkPath::Offline => Some(Hold::Offline),
        NetworkPath::Metered => Some(Hold::Metered),
        NetworkPath::Unmetered if power.low_battery => Some(Hold::LowBattery),
        NetworkPath::Unmetered => None,
    };
    if hold.is_some() {
        return SyncPlan {
            hold,
            order: Vec::new(),
        };
    }
    let mut ranked: Vec<(usize, &SyncAccount)> = accounts.iter().enumerate().collect();
    ranked.sort_by_key(|(index, account)| (Reverse(account.priority), *index));
    SyncPlan {
        hold: None,
        order: ranked
            .into_iter()
            .map(|(_, account)| account.account.clone())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{AccountId, NetworkPath, NetworkState};

    use super::{Hold, Power, SyncAccount, plan, plan_for};

    struct Path(NetworkPath);

    impl NetworkState for Path {
        fn path(&self) -> NetworkPath {
            self.0
        }
    }

    fn account(id: &str, priority: u32) -> SyncAccount {
        SyncAccount {
            account: AccountId::new(id),
            priority,
        }
    }

    fn ids(plan: &super::SyncPlan) -> Vec<&str> {
        plan.order.iter().map(|id| id.as_str()).collect()
    }

    #[test]
    fn offline_metered_and_low_battery_pause() {
        let accounts = vec![account("a", 1)];
        let charged = Power { low_battery: false };
        let offline = plan(&accounts, &Path(NetworkPath::Offline), charged);
        assert_eq!(offline.hold, Some(Hold::Offline));
        assert!(offline.order.is_empty());

        let metered = plan_for(&accounts, NetworkPath::Metered, charged);
        assert_eq!(metered.hold, Some(Hold::Metered));
        assert!(metered.order.is_empty());

        let battery = plan(
            &accounts,
            &Path(NetworkPath::Unmetered),
            Power { low_battery: true },
        );
        assert_eq!(battery.hold, Some(Hold::LowBattery));
        assert!(battery.order.is_empty());

        let both = plan_for(&accounts, NetworkPath::Offline, Power { low_battery: true });
        assert_eq!(both.hold, Some(Hold::Offline));
    }

    #[test]
    fn unmetered_orders_accounts_by_priority() {
        let accounts = vec![account("low", 1), account("high", 5), account("tie", 1)];
        let plan = plan_for(
            &accounts,
            NetworkPath::Unmetered,
            Power { low_battery: false },
        );
        assert_eq!(plan.hold, None);
        assert_eq!(ids(&plan), ["high", "low", "tie"]);
    }
}
