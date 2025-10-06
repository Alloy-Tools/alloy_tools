use std::time::Duration;

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
#[derive(Clone, PartialEq, Debug, Hash)]
pub struct TaskConfig {
    pub interval: Duration,
    pub stop_on_error: bool,
}

impl TaskConfig {
    /// Create a `TaskConfig` with default values and a certain `TaskMode`
    pub fn new(interval: Duration, stop_on_error: bool) -> Self {
        Self {
            interval,
            stop_on_error,
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
        }
    }
}

impl From<Duration> for TaskConfig {
    fn from(interval: Duration) -> Self {
        Self::new(interval, TaskConfig::default_stop_on_error())
    }
}

impl From<bool> for TaskConfig {
    fn from(stop_on_error: bool) -> Self {
        Self::new(TaskConfig::default_interval(), stop_on_error)
    }
}
