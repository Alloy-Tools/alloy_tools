use std::sync::Arc;

use al_events::{Command, DynIdCache, DynMessage, Event, Query, TypeId};
use al_structures::{
    collections::storage::utils::{
        indexed::{IndexedHandle, IndexedStorage},
        keyed::{KeyedHandle, KeyedStorage},
        HandleBulkRead,
    },
    traits::Downcast,
};
use al_transport::dispatcher::{BoxFut, Dispatcher, DispatcherError, Handler, KeyedHandlerKey};

/// Routing keys used by `DynMessage`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DynMessageKey {
    /// Any `Command`.
    Command,
    /// Any `Event`.
    Event,
    /// Any `Query`.
    Query,
    /// One specific type.
    Typed(TypeId),
}

impl std::fmt::Display for DynMessageKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DynMessageKey::Command => write!(f, "Command"),
            DynMessageKey::Event => write!(f, "Event"),
            DynMessageKey::Query => write!(f, "Query"),
            DynMessageKey::Typed(id) => write!(f, "Typed({id})"),
        }
    }
}

/// `DynMessage`-aware dispatcher.
///
/// Registers handlers at five levels of specificity:
/// - `register_any`     - every message
/// - `register_command` - every command
/// - `register_event`   - every event
/// - `register_query`   - every query
/// - `register::<T>`    - messages of concrete type `T`
///
/// A message fires handlers from every layer it matches. A `Msg` event hits `any` + `event` +
/// `Typed(Msg::message_id())`. While a command `Cmd` hits `any` + `command` + `Typed(Cmd::message_id())`.
pub struct DynMessageDispatcher<
    Meta,
    State,
    A: IndexedHandle<Handler<DynMessage, Meta, State>>,
    R: KeyedHandle<DynMessageKey, S>,
    S: IndexedStorage<Handler<DynMessage, Meta, State>>,
> {
    inner: Dispatcher<DynMessage, DynMessageKey, [DynMessageKey; 2], Meta, State, A, R, S>,
}

impl<
        Meta,
        State,
        A: IndexedHandle<Handler<DynMessage, Meta, State>>,
        R: KeyedHandle<DynMessageKey, S>,
        S: IndexedStorage<Handler<DynMessage, Meta, State>>,
    > Clone for DynMessageDispatcher<Meta, State, A, R, S>
where
    State: Clone,
    A: Clone,
    R: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<
        Meta,
        State,
        A: IndexedHandle<Handler<DynMessage, Meta, State>>,
        R: KeyedHandle<DynMessageKey, S>,
        S: IndexedStorage<Handler<DynMessage, Meta, State>>,
    > DynMessageDispatcher<Meta, State, A, R, S>
{
    pub fn new(state: State, any: A, keyed: R) -> Self {
        Self {
            inner: Dispatcher::new(
                state,
                |msg| {
                    let id = DynMessageKey::Typed(
                        msg.message_id()
                            .map_err(|e| DispatcherError::RoutingError(Box::new(e)))?,
                    );
                    let variant = match msg {
                        al_events::Message::Command(_, _) => DynMessageKey::Command,
                        al_events::Message::Event(_, _) => DynMessageKey::Event,
                        al_events::Message::Query(_, _) => DynMessageKey::Query,
                    };
                    Ok([variant, id])
                },
                any,
                keyed,
            ),
        }
    }

    pub async fn deep_clone(&self) -> Result<Self, DispatcherError>
    where
        State: Clone,
        S: Clone,
    {
        Ok(Self {
            inner: self.inner.deep_clone().await?,
        })
    }

    pub async fn dispatch(&self, meta: Meta, message: DynMessage) -> Result<(), DispatcherError>
    where
        Meta: Clone,
        State: Clone,
        R: HandleBulkRead<R::Storage>,
        R::Storage: KeyedStorage<DynMessageKey, S>,
    {
        self.inner.dispatch(meta, message).await
    }

    /// Fires on every message, whatever its kind.
    pub async fn register_any<F>(&self, handler: F) -> Result<A::Key, DispatcherError>
    where
        F: Fn(Meta, State, DynMessage) -> BoxFut<()> + Send + Sync + 'static,
    {
        self.inner.register_any(handler).await
    }

    /// Fires on every command.
    pub async fn register_command_any<F>(
        &self,
        handler: F,
    ) -> Result<KeyedHandlerKey<S::Key, DynMessageKey>, DispatcherError>
    where
        F: Fn(Meta, State, DynMessage) -> BoxFut<()> + Send + Sync + 'static,
        S: Clone,
    {
        self.inner
            .register_keyed(DynMessageKey::Command, handler)
            .await
    }

    /// Fires on every event.
    pub async fn register_event_any<F>(
        &self,
        handler: F,
    ) -> Result<KeyedHandlerKey<S::Key, DynMessageKey>, DispatcherError>
    where
        F: Fn(Meta, State, DynMessage) -> BoxFut<()> + Send + Sync + 'static,
        S: Clone,
    {
        self.inner
            .register_keyed(DynMessageKey::Event, handler)
            .await
    }

    /// Fires on every query.
    pub async fn register_query_any<F>(
        &self,
        handler: F,
    ) -> Result<KeyedHandlerKey<S::Key, DynMessageKey>, DispatcherError>
    where
        F: Fn(Meta, State, DynMessage) -> BoxFut<()> + Send + Sync + 'static,
        S: Clone,
    {
        self.inner
            .register_keyed(DynMessageKey::Query, handler)
            .await
    }

    /// Fires only on commands of concrete type `C`.
    /// The handler receives the already-downcast `C`.
    pub async fn register_command<F, C>(
        &self,
        handler: F,
    ) -> Result<KeyedHandlerKey<S::Key, DynMessageKey>, DispatcherError>
    where
        Meta: Send + 'static,
        State: Send + 'static,
        F: Fn(Meta, State, C) -> BoxFut<()> + Send + Sync + 'static,
        S: Clone,
        C: Command,
    {
        let handler = Arc::new(handler);
        let wrapped = move |meta, state, msg: DynMessage| -> BoxFut<()> {
            let handler = handler.clone();
            Box::pin(async move {
                let cmd = match msg.into_command().map(|(c, _)| c.downcast::<C>()) {
                    Some(Ok(cmd)) => cmd,
                    _ => return,
                };
                handler(meta, state, cmd).await
            })
        };
        self.inner
            .register_keyed(
                DynMessageKey::Typed(
                    C::type_message_id().map_err(|e| DispatcherError::Custom(Box::new(e)))?,
                ),
                wrapped,
            )
            .await
    }

    /// Fires only on events of concrete type `E`.
    /// The handler receives the already-downcast `E`.
    pub async fn register_event<F, E>(
        &self,
        handler: F,
    ) -> Result<KeyedHandlerKey<S::Key, DynMessageKey>, DispatcherError>
    where
        Meta: Send + 'static,
        State: Send + 'static,
        F: Fn(Meta, State, E) -> BoxFut<()> + Send + Sync + 'static,
        S: Clone,
        E: Event,
    {
        let handler = Arc::new(handler);
        let wrapped = move |meta, state, msg: DynMessage| -> BoxFut<()> {
            let handler = handler.clone();
            Box::pin(async move {
                let evt = match msg.into_event().map(|(e, _)| e.downcast::<E>()) {
                    Some(Ok(evt)) => evt,
                    _ => return,
                };
                handler(meta, state, evt).await
            })
        };
        self.inner
            .register_keyed(
                DynMessageKey::Typed(
                    E::type_message_id().map_err(|e| DispatcherError::Custom(Box::new(e)))?,
                ),
                wrapped,
            )
            .await
    }

    /// Fires only on queries of concrete type `Q`.
    /// The handler receives the already-downcast `Q`.
    pub async fn register_query<F, Q>(
        &self,
        handler: F,
    ) -> Result<KeyedHandlerKey<S::Key, DynMessageKey>, DispatcherError>
    where
        Meta: Send + 'static,
        State: Send + 'static,
        F: Fn(Meta, State, Q) -> BoxFut<()> + Send + Sync + 'static,
        S: Clone,
        Q: Query,
    {
        let handler = Arc::new(handler);
        let wrapped = move |meta, state, msg: DynMessage| -> BoxFut<()> {
            let handler = handler.clone();
            Box::pin(async move {
                let qry = match msg.into_query().map(|(q, _)| q.downcast::<Q>()) {
                    Some(Ok(qry)) => qry,
                    _ => return,
                };
                handler(meta, state, qry).await
            })
        };
        self.inner
            .register_keyed(
                DynMessageKey::Typed(
                    Q::type_message_id().map_err(|e| DispatcherError::Custom(Box::new(e)))?,
                ),
                wrapped,
            )
            .await
    }
}
