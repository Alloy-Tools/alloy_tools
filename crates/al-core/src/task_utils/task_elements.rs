use std::{sync::Arc, time::Duration};

/// Error type for `Task` types
#[derive(Debug, Clone)]
pub enum TaskError {
    Custom(String),
    NoCondition(String),
}

/// `TaskMode` defines parameters for if a `Task` should be stopped automatically
#[derive(Clone, Default, PartialEq, Debug, Hash)]
pub enum TaskMode {
    /// Run until canceled
    #[default]
    Infinite,
    /// Run for a fixed number of iterations
    Fixed(usize),
    /// Run until a condition is met
    Conditional,
    /// Run for a specific duration
    Duration(Duration),
}

/// `TaskConfig` contains the required config to initialize a `Task`
#[derive(Clone)]
pub struct TaskConfig {
    interval: Duration,
    stop_on_error: bool,
    mode: TaskMode,
    on_task_start: Option<Arc<dyn Fn() + Send + Sync>>,
    on_task_complete: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl TaskConfig {
    /// Create a `TaskConfig` with default values and a certain `TaskMode`
    pub fn new(
        interval: Duration,
        stop_on_error: bool,
        mode: TaskMode,
        on_task_start: Option<Arc<dyn Fn() + Send + Sync>>,
        on_task_complete: Option<Arc<dyn Fn() + Send + Sync>>,
    ) -> Self {
        Self {
            interval,
            stop_on_error,
            mode,
            on_task_start,
            on_task_complete,
        }
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn stop_on_error(&self) -> bool {
        self.stop_on_error
    }

    pub fn mode(&self) -> &TaskMode {
        &self.mode
    }

    pub fn on_task_start(&self) {
        if let Some(f) = &self.on_task_start {
            f();
        }
    }

    pub fn on_task_complete(&self) {
        if let Some(f) = &self.on_task_complete {
            f();
        }
    }

    /// Checks if the `Task` should stop based on the `TaskMode` and number of iterations
    pub async fn check_iterations(&self, iteration: usize) -> bool {
        match &self.mode {
            TaskMode::Duration(_) | TaskMode::Infinite | TaskMode::Conditional => false,
            TaskMode::Fixed(max_iters) => iteration >= *max_iters,
        }
    }

    /// Check if the `Task` should stop based on the `TaskMode` and the start_time
    pub async fn check_duration(&self, start_time: tokio::time::Instant) -> bool {
        match &self.mode {
            TaskMode::Fixed(_) | TaskMode::Infinite | TaskMode::Conditional => false,
            TaskMode::Duration(duration) => start_time.elapsed() >= *duration,
        }
    }

    fn default_interval() -> Duration {
        Duration::from_millis(100)
    }

    fn default_stop_on_error() -> bool {
        false
    }
}

impl Default for TaskConfig {
    /// Creates an `TaskMode::Infinite` `TaskConfig` with an interval of 100 miliseconds
    fn default() -> Self {
        Self {
            interval: TaskConfig::default_interval(),
            stop_on_error: TaskConfig::default_stop_on_error(),
            mode: TaskMode::default(),
            on_task_start: None,
            on_task_complete: None,
        }
    }
}

impl PartialEq for TaskConfig {
    fn eq(&self, other: &Self) -> bool {
        self.interval == other.interval
            && self.stop_on_error == other.stop_on_error
            && self.mode == other.mode
            && (self.on_task_start.is_some() == other.on_task_start.is_some())
            && (self.on_task_complete.is_some() == other.on_task_complete.is_some())
    }
}

impl std::fmt::Debug for TaskConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskConfig")
            .field("interval", &self.interval)
            .field("stop_on_error", &self.stop_on_error)
            .field("mode", &self.mode)
            .field(
                "on_task_start",
                if self.on_task_start.is_some() {
                    &"<Fn>"
                } else {
                    &"None"
                },
            )
            .field(
                "on_task_stop",
                if self.on_task_complete.is_some() {
                    &"<Fn>"
                } else {
                    &"None"
                },
            )
            .finish()
    }
}

impl std::hash::Hash for TaskConfig {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.interval.hash(state);
        self.stop_on_error.hash(state);
        self.mode.hash(state);
        self.on_task_start.is_some().hash(state);
        self.on_task_complete.is_some().hash(state);
    }
}

impl From<Duration> for TaskConfig {
    fn from(interval: Duration) -> Self {
        Self::new(
            interval,
            TaskConfig::default_stop_on_error(),
            TaskMode::default(),
            None,
            None,
        )
    }
}

impl From<bool> for TaskConfig {
    fn from(stop_on_error: bool) -> Self {
        Self::new(
            TaskConfig::default_interval(),
            stop_on_error,
            TaskMode::default(),
            None,
            None,
        )
    }
}

impl From<TaskMode> for TaskConfig {
    fn from(mode: TaskMode) -> Self {
        Self::new(
            TaskConfig::default_interval(),
            TaskConfig::default_stop_on_error(),
            mode,
            None,
            None,
        )
    }
}
