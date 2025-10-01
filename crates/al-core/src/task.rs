use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

/// `TaskMode` defines parameters for if a task should be stopped automatically
#[derive(Clone, Default, PartialEq, Debug, Hash)]
pub enum TaskMode {
    /// Run until canceled
    #[default]
    Infinite,
    /// Run for a fixed number of iterations
    For(usize),
    /// Run until a condition is met
    Conditional,
    /// Run for a specific duration
    Duration(Duration),
}

#[derive(Clone, PartialEq, Debug, Hash)]
pub struct TaskConfig {
    pub mode: TaskMode,
    pub interval: Duration,
    pub immediate_run: bool,
    pub stop_on_error: bool,
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            mode: Default::default(),
            interval: Duration::from_millis(100),
            immediate_run: Default::default(),
            stop_on_error: Default::default(),
        }
    }
}

#[derive(Clone, PartialEq, Debug, Hash, Default)]
pub struct TaskState<T = (), E = ()> {
    pub iterations: usize,
    pub last_result: Option<Result<T, E>>,
    pub is_running: bool,
    pub mode: TaskMode,
}

pub struct Task<T = (), E = ()> {
    handle: Option<JoinHandle<()>>,
    cancelled: Arc<RwLock<bool>>,
    state: Arc<RwLock<TaskState<T, E>>>,
}
