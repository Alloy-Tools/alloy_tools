use al_core::{Command, DowncastEvent, Event, EventMarker};
use al_crypto::NonceTrait;
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};
use tokio::sync::RwLock;

use crate::ConnectionManager;

type FutureOutput = ();

/// Type for event handlers
type EventHandler<N> = Arc<
    dyn Fn(
            u64,
            Arc<ConnectionManager<N>>,
            Box<dyn Event>,
        ) -> Pin<Box<dyn Future<Output = FutureOutput> + Send>>
        + Send
        + Sync,
>;

/// Type for command handlers
type CommandHandler<N> = Arc<
    dyn Fn(
            u64,
            Arc<ConnectionManager<N>>,
            Command,
        ) -> Pin<Box<dyn Future<Output = FutureOutput> + Send>>
        + Send
        + Sync,
>;

#[derive(Clone, Default)]
pub struct CommandDispatcher<N: NonceTrait> {
    event_handlers: Arc<RwLock<HashMap<String, Vec<EventHandler<N>>>>>,
    command_handlers: Arc<RwLock<Vec<CommandHandler<N>>>>,
}

impl<N: NonceTrait> CommandDispatcher<N> {
    pub fn new() -> Self {
        Self {
            event_handlers: Arc::new(RwLock::new(HashMap::new())),
            command_handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn deep_clone(&self) -> Self {
        Self {
            event_handlers: Arc::new(RwLock::new(self.event_handlers.read().await.clone())),
            command_handlers: Arc::new(RwLock::new(self.command_handlers.read().await.clone())),
        }
    }

    /// Register a handler for a specific event type
    /// The handler receives: (connection_id, connection_manager, event)
    pub async fn register_event<
        E: Event + EventMarker,
        F: Future<Output = FutureOutput> + Send + Sync + 'static,
    >(
        &mut self,
        handler: impl Fn(u64, Arc<ConnectionManager<N>>, E) -> F + Send + Sync + 'static,
    ) {
        let handler = Arc::new(handler);
        let wrapped_handler: EventHandler<N> = Arc::new(move |conn_id, connections, event| {
            let handler = handler.clone();
            Box::pin(async move {
                if let Ok(typed_event) = event.downcast::<E>() {
                    handler(conn_id, connections, typed_event).await;
                }
            })
        });

        self.event_handlers
            .write()
            .await
            .entry(<E as EventMarker>::type_with_generics())
            .or_insert_with(Vec::new)
            .push(wrapped_handler);
    }

    /// Register a handler for all commands
    pub async fn register_command<F: Future<Output = FutureOutput> + Send + Sync + 'static>(
        &mut self,
        handler: impl Fn(u64, Arc<ConnectionManager<N>>, Command) -> F + Send + Sync + 'static,
    ) {
        let handler = Arc::new(handler);
        let wrapped_handler: CommandHandler<N> = Arc::new(move |conn_id, connections, command| {
            let handler = handler.clone();
            Box::pin(async move {
                handler(conn_id, connections, command).await;
            })
        });

        self.command_handlers.write().await.push(wrapped_handler);
    }

    /// Dispatch a command to all command handlers, also routes event vairants to event handlers
    pub async fn dispatch(
        &self,
        conn_id: u64,
        connections: Arc<ConnectionManager<N>>,
        command: Command,
    ) {
        // Call all command handlers first
        for handler in &*self.command_handlers.read().await {
            handler(conn_id, connections.clone(), command.clone()).await;
        }

        // If it's an event, also route to event-specific handlers
        if let Command::Event(event) = command {
            self.dispatch_event(conn_id, connections, event).await;
        }
    }

    /// Dispatch an event to all registered handlers
    pub async fn dispatch_event(
        &self,
        conn_id: u64,
        connections: Arc<ConnectionManager<N>>,
        event: Box<dyn Event>,
    ) {
        if let Some(handlers) = self
            .event_handlers
            .read()
            .await
            .get(&event.type_with_generics())
        {
            for handler in handlers {
                handler(conn_id, connections.clone(), event.clone()).await;
            }
        }
    }
}
