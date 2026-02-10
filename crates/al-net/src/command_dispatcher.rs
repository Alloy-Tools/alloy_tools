use al_core::{Command, DowncastEvent, Event, EventMarker};
use std::{collections::HashMap, future::Future, marker::PhantomData, pin::Pin, sync::Arc};
use tokio::sync::RwLock;

type FutureOutput = ();

/// Type for event handlers
type EventHandler<MetaData, State> = Arc<
    dyn Fn(MetaData, State, Box<dyn Event>) -> Pin<Box<dyn Future<Output = FutureOutput> + Send>>
        + Send
        + Sync,
>;

/// Type for command handlers
type CommandHandler<MetaData, State> = Arc<
    dyn Fn(MetaData, State, Command) -> Pin<Box<dyn Future<Output = FutureOutput> + Send>>
        + Send
        + Sync,
>;

#[derive(Clone, Default)]
pub struct CommandDispatcher<
    MetaData: Send + Sync + Clone + 'static,
    State: Send + Sync + Clone + 'static,
> {
    event_handlers: Arc<RwLock<HashMap<String, Vec<EventHandler<MetaData, State>>>>>,
    command_handlers: Arc<RwLock<Vec<CommandHandler<MetaData, State>>>>,
    state: State,
    _phantom: PhantomData<MetaData>,
}

impl<MetaData: Send + Sync + Clone + 'static, State: Send + Sync + Clone + 'static>
    CommandDispatcher<MetaData, State>
{
    pub fn new(state: State) -> Self {
        Self {
            event_handlers: Arc::new(RwLock::new(HashMap::new())),
            command_handlers: Arc::new(RwLock::new(Vec::new())),
            state,
            _phantom: PhantomData,
        }
    }

    pub async fn deep_clone(&self) -> Self {
        Self {
            event_handlers: Arc::new(RwLock::new(self.event_handlers.read().await.clone())),
            command_handlers: Arc::new(RwLock::new(self.command_handlers.read().await.clone())),
            state: self.state.clone(),
            _phantom: self._phantom.clone(),
        }
    }

    /// Register a handler for a specific event type.
    /// The handler receives: (metadata, state, event).
    pub async fn register_event<
        E: Event + EventMarker,
        F: Future<Output = FutureOutput> + Send + Sync + 'static,
    >(
        &mut self,
        handler: impl Fn(MetaData, State, E) -> F + Send + Sync + 'static,
    ) {
        let handler = Arc::new(handler);
        let wrapped_handler: EventHandler<MetaData, State> =
            Arc::new(move |metadata, state, event| {
                let handler = handler.clone();
                Box::pin(async move {
                    if let Ok(typed_event) = event.downcast::<E>() {
                        handler(metadata, state, typed_event).await;
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

    /// Register a handler for all commands.
    /// The handler receives: (metadata, state, command).
    pub async fn register_command<F: Future<Output = FutureOutput> + Send + Sync + 'static>(
        &mut self,
        handler: impl Fn(MetaData, State, Command) -> F + Send + Sync + 'static,
    ) {
        let handler = Arc::new(handler);
        let wrapped_handler: CommandHandler<MetaData, State> =
            Arc::new(move |metadata, state, command| {
                let handler = handler.clone();
                Box::pin(async move {
                    handler(metadata, state, command).await;
                })
            });

        self.command_handlers.write().await.push(wrapped_handler);
    }

    /// Dispatch a command to all command handlers, also routes event vairants to event handlers
    pub async fn dispatch(&self, metadata: MetaData, command: Command) {
        // Call all command handlers first
        for handler in &*self.command_handlers.read().await {
            handler(metadata.clone(), self.state.clone(), command.clone()).await;
        }

        // If it's an event, also route to event-specific handlers
        if let Command::Event(event) = command {
            self.dispatch_event(metadata, event).await;
        }
    }

    /// Dispatch an event to all registered handlers
    pub async fn dispatch_event(&self, metadata: MetaData, event: Box<dyn Event>) {
        if let Some(handlers) = self
            .event_handlers
            .read()
            .await
            .get(&event.type_with_generics())
        {
            for handler in handlers {
                handler(metadata.clone(), self.state.clone(), event.clone()).await;
            }
        }
    }
}
