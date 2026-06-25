//! Tracks the subscriptions the consumer wants active, for replay on reconnect.

use std::collections::BTreeMap;

use super::protocol::Subscription;

/// Records subscription intent keyed by id. Transport-agnostic: it stores what
/// should be active so the client can replay it after a reconnect.
#[derive(Debug, Default)]
pub(crate) struct SubscriptionManager {
    subscriptions: BTreeMap<String, Subscription>,
}

impl SubscriptionManager {
    /// Record intent to be subscribed. Returns `true` if the id is new.
    pub(crate) fn add(&mut self, subscription: Subscription) -> bool {
        self.subscriptions
            .insert(subscription.id.clone(), subscription)
            .is_none()
    }

    /// Drop intent for `id`. Returns `true` if it was present.
    pub(crate) fn remove(&mut self, id: &str) -> bool {
        self.subscriptions.remove(id).is_some()
    }

    /// Every subscription that should currently be active.
    pub(crate) fn desired(&self) -> impl Iterator<Item = &Subscription> {
        self.subscriptions.values()
    }
}
