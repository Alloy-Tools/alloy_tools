use std::{future::Future, pin::Pin, sync::Arc};

use al_structures::{
    collections::storage::utils::{
        indexed::{IndexedHandle, IndexedStorage},
        keyed::{KeyedHandle, KeyedStorage, KeyedStorageRead},
        HandleBulkRead, HandleError,
    },
    traits::CloneEqError,
};

pub type BoxFut<R> = Pin<Box<dyn Future<Output = R> + Send + 'static>>;

/// A type-erased handler.
pub type Handler<T, Meta, State> =
    Arc<dyn Fn(Meta, State, T) -> BoxFut<()> + Send + Sync + 'static>;

pub type KeyedHandlerKey<O, I> = (O, I);

/// Maps a message to the set of routing keys it should fire.
pub type Router<T, Keys> = Arc<dyn Fn(&T) -> Result<Keys, DispatcherError> + Send + Sync + 'static>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DispatcherError {
    HandleError(HandleError),
    RoutingError(Box<dyn CloneEqError>),
    Custom(Box<dyn CloneEqError>),
}

impl std::fmt::Display for DispatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HandleError(err) => err.fmt(f),
            Self::RoutingError(err) => err.fmt(f),
            Self::Custom(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for DispatcherError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Custom(err) => err.source(),
            Self::RoutingError(err) => err.source(),
            Self::HandleError(err) => err.source(),
        }
    }
}

impl From<HandleError> for DispatcherError {
    fn from(value: HandleError) -> Self {
        DispatcherError::HandleError(value)
    }
}

impl From<al_structures::collections::storage::utils::StorageError> for DispatcherError {
    fn from(value: al_structures::collections::storage::utils::StorageError) -> Self {
        DispatcherError::HandleError(HandleError::Storage(value))
    }
}

impl From<Box<dyn CloneEqError>> for DispatcherError {
    fn from(value: Box<dyn CloneEqError>) -> Self {
        DispatcherError::Custom(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub enum RouteKeys<K> {
    #[default]
    None,
    One(K),
    Two([K; 2]),
    Many(Vec<K>),
}

impl<K> IntoIterator for RouteKeys<K> {
    type Item = K;
    type IntoIter = RouteKeysIter<K>;
    fn into_iter(self) -> Self::IntoIter {
        match self {
            RouteKeys::None => RouteKeysIter::None(std::iter::empty()),
            RouteKeys::One(k) => RouteKeysIter::One(std::iter::once(k)),
            RouteKeys::Two(keys) => RouteKeysIter::Two(keys.into_iter()),
            RouteKeys::Many(keys) => RouteKeysIter::Many(keys.into_iter()),
        }
    }
}

pub enum RouteKeysIter<K> {
    None(std::iter::Empty<K>),
    One(std::iter::Once<K>),
    Two(std::array::IntoIter<K, 2>),
    Many(std::vec::IntoIter<K>),
}

impl<K> Iterator for RouteKeysIter<K> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            RouteKeysIter::None(iter) => iter.next(),
            RouteKeysIter::One(iter) => iter.next(),
            RouteKeysIter::Two(iter) => iter.next(),
            RouteKeysIter::Many(iter) => iter.next(),
        }
    }
}

// A type-agnostic dispatcher.
///
/// - `T`     - message type
/// - `K`     - routing key (eg. `T::id`, `"Command"`)
/// - `Meta`  - per-call context eg. `(conn id, sender, …)`
/// - `State` - shared state
/// - `A`     - Handler registry for any message
/// - `R`     - Handler registry for messages routed by key
/// - `S`     - Inner registry type for `R`
pub struct Dispatcher<
    T,
    K,
    Keys,
    Meta,
    State,
    A: IndexedHandle<Handler<T, Meta, State>>,
    R: KeyedHandle<K, S>,
    S: IndexedStorage<Handler<T, Meta, State>>,
> {
    routing: Router<T, Keys>,
    /// Handlers that fire on every message.
    any: A,
    /// Handlers keyed by routing key.
    keyed: R,
    state: State,
    _phantom: std::marker::PhantomData<fn() -> (K, Meta, S)>,
}

impl<
        T,
        K,
        Keys,
        Meta,
        State,
        A: IndexedHandle<Handler<T, Meta, State>>,
        R: KeyedHandle<K, S>,
        S: IndexedStorage<Handler<T, Meta, State>>,
    > Clone for Dispatcher<T, K, Keys, Meta, State, A, R, S>
where
    State: Clone,
    A: Clone,
    R: Clone,
{
    fn clone(&self) -> Self {
        Self {
            routing: self.routing.clone(),
            any: self.any.clone(),
            keyed: self.keyed.clone(),
            state: self.state.clone(),
            _phantom: self._phantom.clone(),
        }
    }
}

impl<
        T,
        K,
        Keys,
        Meta,
        State,
        A: IndexedHandle<Handler<T, Meta, State>>,
        R: KeyedHandle<K, S>,
        S: IndexedStorage<Handler<T, Meta, State>>,
    > Dispatcher<T, K, Keys, Meta, State, A, R, S>
{
    pub fn new(
        state: State,
        routing: impl Fn(&T) -> Result<Keys, DispatcherError> + Send + Sync + 'static,
        any: A,
        keyed: R,
    ) -> Self {
        Self {
            routing: Arc::new(routing),
            any,
            keyed,
            state,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Isolated copy: registrations on one don't affect the other.
    pub async fn deep_clone(&self) -> Result<Self, DispatcherError>
    where
        K: Clone,
        S: Clone,
        State: Clone,
    {
        Ok(Self {
            routing: self.routing.clone(),
            any: A::from_iter(self.any.values()?),
            keyed: R::from_iter(self.keyed.entries()?),
            state: self.state.clone(),
            _phantom: self._phantom.clone(),
        })
    }

    /// Fires on every message.
    pub async fn register_any<F>(&self, handler: F) -> Result<A::Key, DispatcherError>
    where
        F: Fn(Meta, State, T) -> BoxFut<()> + Send + Sync + 'static,
    {
        Ok(self.any.push(Arc::new(handler))?)
    }

    /// Fires when the message's routing keys include `key`.
    pub async fn register_keyed<F>(
        &self,
        key: K,
        handler: F,
    ) -> Result<KeyedHandlerKey<S::Key, K>, DispatcherError>
    where
        F: Fn(Meta, State, T) -> BoxFut<()> + Send + Sync + 'static,
        K: std::fmt::Display + std::hash::Hash + Eq + Clone,
        S: Clone,
    {
        if !self.keyed.contains_key(&key)? {
            self.keyed.try_insert(key.clone(), S::new())?;
        }
        Ok((
            self.keyed
                .with_mut(&key, |stg| stg.push(Arc::new(handler)))?
                .ok_or_else(|| {
                    DispatcherError::HandleError(HandleError::Storage(
                        al_structures::collections::storage::utils::StorageError::MissingValue(
                            format!("Missing storage under key '{key}' which was just inserted"),
                        ),
                    ))
                })??,
            key,
        ))
    }

    /// Fire `any` handlers, then all handlers keyed by any of the
    /// message's routing keys.
    pub async fn dispatch(&self, meta: Meta, message: T) -> Result<(), DispatcherError>
    where
        Keys: IntoIterator<Item = K>,
        Meta: Clone,
        State: Clone,
        T: Clone,
        R: HandleBulkRead<R::Storage>,
        R::Storage: KeyedStorage<K, S>,
        K: std::hash::Hash + Eq,
    {
        let keys = (self.routing)(&message)?;

        let (any, keyed) = {
            let any = self.any.values()?;
            let keyed = self.keyed.with_read(|map| {
                let mut keyed = Vec::new();
                for k in keys {
                    if let Some(hs) = map.get(&k)? {
                        keyed.extend(hs.iter().map(|(_, v)| v.clone()));
                    }
                }
                Ok::<_, DispatcherError>(keyed)
            })??;
            (any, keyed)
        };

        for h in any {
            h(meta.clone(), self.state.clone(), message.clone()).await;
        }
        for h in keyed {
            h(meta.clone(), self.state.clone(), message.clone()).await;
        }
        Ok(())
    }
}
