use crate::{TaskStateRequirements, TaskTypes};
use std::sync::Arc;
use std::time::Duration;
use std::{future::Future, marker::PhantomData};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::Instant;

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
    pub mode: TaskMode,
    pub interval: Duration,
    pub stop_on_error: bool,
}

impl TaskConfig {
    /// Create a `TaskConfig` with default values and a certain `TaskMode`
    pub fn new(mode: TaskMode) -> Self {
        Self {
            mode,
            interval: Duration::from_millis(100),
            stop_on_error: false,
        }
    }
}

impl Default for TaskConfig {
    /// Creates an `TaskMode::Infinite` `TaskConfig` with an interval of 100 miliseconds
    fn default() -> Self {
        Self {
            mode: Default::default(),
            interval: Duration::from_millis(100),
            stop_on_error: false,
        }
    }
}

/// `TaskState` contains all required functions hooks and should hold all values a `Task` tracks between cycles
pub trait TaskState<T: TaskTypes = (), E: TaskTypes = ()>: TaskStateRequirements {
    fn get_mode(&self) -> &TaskMode;
    fn get_iterations(&self) -> usize;
    fn set_iteration(&mut self, iterations: usize);

    fn get_last_result(&self) -> Option<Result<T, E>>;
    fn set_last_result(&mut self, result: Result<T, E>);

    fn get_is_running(&self) -> bool;
    fn set_is_running(&mut self, is_running: bool);

    fn on_task_start(&mut self) {}
    fn on_task_complete(&mut self) {}
}

/// `BaseTaskState` contains all values a `Task` tracks between cycles
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct BaseTaskState<T: TaskTypes = (), E: TaskTypes = ()> {
    mode: TaskMode,
    iterations: usize,
    last_result: Option<Result<T, E>>,
    is_running: bool,
}

impl<T: TaskTypes, E: TaskTypes> Default for BaseTaskState<T, E> {
    fn default() -> Self {
        Self::new(TaskMode::default())
    }
}

impl<T: TaskTypes, E: TaskTypes> BaseTaskState<T, E> {
    pub fn new(mode: TaskMode) -> Self {
        Self {
            mode,
            iterations: 0,
            last_result: None,
            is_running: false,
        }
    }
}

impl<T: TaskTypes, E: TaskTypes> TaskState<T, E> for BaseTaskState<T, E> {
    fn get_mode(&self) -> &TaskMode {
        &self.mode
    }

    fn get_iterations(&self) -> usize {
        self.iterations
    }

    fn set_iteration(&mut self, iterations: usize) {
        self.iterations = iterations;
    }

    fn get_last_result(&self) -> Option<Result<T, E>> {
        self.last_result.clone()
    }

    fn set_last_result(&mut self, result: Result<T, E>) {
        self.last_result = Some(result);
    }

    fn get_is_running(&self) -> bool {
        self.is_running
    }

    fn set_is_running(&mut self, is_running: bool) {
        self.is_running = is_running;
    }
}

pub trait WithTaskState<T: TaskTypes, E: TaskTypes>: TaskStateRequirements {
    fn as_task_state(self) -> ExtendedTaskState<T, E, Self>;
    fn with_task_state(self, mode: TaskMode) -> ExtendedTaskState<T, E, Self>;
}

impl<T: TaskTypes, E: TaskTypes, S: TaskStateRequirements> WithTaskState<T, E> for S {
    fn as_task_state(self) -> ExtendedTaskState<T, E, Self> {
        Self::with_task_state(self, TaskMode::default())
    }

    fn with_task_state(self, mode: TaskMode) -> ExtendedTaskState<T, E, Self> {
        ExtendedTaskState::new(mode, self)
    }
}

/// `ExtendedTaskState` extra state not strictly required by `Task`, passed by the user.
/// `ExtendedTaskState` is auto implemented for all types `'static + Send + Sync + Default + Clone`
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct ExtendedTaskState<T: TaskTypes, E: TaskTypes, S: TaskStateRequirements> {
    base: BaseTaskState<T, E>,
    extended: S,
}

impl<T: TaskTypes, E: TaskTypes, S: TaskStateRequirements + Default> Default
    for ExtendedTaskState<T, E, S>
{
    fn default() -> Self {
        Self {
            base: BaseTaskState::default(),
            extended: S::default(),
        }
    }
}

impl<T: TaskTypes, E: TaskTypes, S: TaskStateRequirements> ExtendedTaskState<T, E, S> {
    pub fn new(mode: TaskMode, extended: S) -> Self {
        Self {
            base: BaseTaskState::new(mode),
            extended,
        }
    }

    pub fn into_inner(self) -> S {
        self.extended
    }

    pub fn inner_clone(&self) -> S {
        self.extended.clone()
    }
}

/// Auto impl `TaskState`
impl<T: TaskTypes, E: TaskTypes, S: TaskStateRequirements> TaskState<T, E>
    for ExtendedTaskState<T, E, S>
{
    fn get_mode(&self) -> &TaskMode {
        self.base.get_mode()
    }

    fn get_iterations(&self) -> usize {
        self.base.get_iterations()
    }

    fn set_iteration(&mut self, iterations: usize) {
        self.base.set_iteration(iterations)
    }

    fn get_last_result(&self) -> Option<Result<T, E>> {
        self.base.get_last_result()
    }

    fn set_last_result(&mut self, result: Result<T, E>) {
        self.base.set_last_result(result)
    }

    fn get_is_running(&self) -> bool {
        self.base.get_is_running()
    }

    fn set_is_running(&mut self, is_running: bool) {
        self.base.set_is_running(is_running)
    }
}

/// `Task` handles the interactions and state of the background thread it spawned
#[derive(Debug)]
pub struct Task<T: TaskTypes, E: TaskTypes, S: TaskState<T, E>> {
    handle: Option<JoinHandle<()>>,
    panicked: Arc<RwLock<bool>>,
    cancelled: Arc<RwLock<bool>>,
    state: Arc<RwLock<S>>,
    _phantom: std::marker::PhantomData<(T, E)>,
}

impl<T: TaskTypes, E: TaskTypes, S: TaskState<T, E>> Drop for Task<T, E, S> {
    /// `Task` aborts its spawned thread on drop
    fn drop(&mut self) {
        self.abort();
    }
}

/// Helper macro to edit the common where clause in one place for `Task` functions
macro_rules! with_common_bounds {
    (
        $(
            $(#[$meta:meta])*
            $vis:vis fn $name:ident$(<$($generics:ident),*>)?($($params:tt)*) $(-> $ret:ty)?
            $(where $($rest:tt)*)?
            $body:block
        )*
    ) => {
        $(
            $(#[$meta])*
            $vis fn $name<$($($generics),*)? F, Fut>($($params)*) $(-> $ret)?
            where
                F: FnMut(usize, &Arc<RwLock<S>>) -> Fut + Send + Sync + 'static,
                Fut: Future<Output = Result<T, E>> + Send + Sync + 'static,
                $($($rest)*)?
            $body
        )*
    };
}

/// Implement `Task`, generating the actual functions with the `with_common_bounds` macro
impl<T: TaskTypes, E: TaskTypes, S: TaskState<T, E>> Task<T, E, S> {
    with_common_bounds! {
        /// Creates a `Task` with the default `TaskConfig`
        pub fn new(f: F, state: S) -> Self {
            Self::_infinite(f, TaskConfig::default(), state)
        }

        /// Creates a `Task` that runs a fixed number of times, with the default `TaskConfig`
        pub fn fixed(iterations: usize, f: F, state: S) -> Self {
            Self::_fixed(f, TaskConfig::new(TaskMode::Fixed(iterations)), state)
        }

        /// Creates a `Task` that runs for a specific duration, with the default `TaskConfig`
        pub fn for_duration(duration: Duration, f: F, state: S) -> Self {
            Self::_duration(
                f,
                TaskConfig::new(TaskMode::Duration(duration)),
                state,
                Instant::now(),
            )
        }

        /*/// Creates a `Task` that runs until a condition is met, with the default `TaskConfig`
        pub fn until_condition<C, FutC>(f: F, state: S, condition: C) -> Result<Self, TaskError>
        where
            C: FnMut(&S) -> FutC + Send + Sync + 'static,
            FutC: Future<Output = bool> + Send + Sync + 'static,
        {
            let config = TaskConfig::new(TaskMode::Conditional);
            Self::with_config(f, config, state, Some(condition))
        }*/

        /*/// Creates a `Task` with a specific `TaskConfig`
        pub fn with_config<C, FutC>(
            f: F,
            config: TaskConfig,
            state: S,
            condition: Option<C>,
        ) -> Result<Self, TaskError>
        where
            C: FnMut(&S) -> FutC + Send + Sync + 'static,
            FutC: Future<Output = bool> + Send + Sync + 'static,
        {
            match config.mode {
                TaskMode::Infinite => Ok(Task::_infinite(f, config, state)),
                TaskMode::Fixed(_) => Ok(Task::_fixed(f, config, state)),
                TaskMode::Conditional => Ok(Task::_conditional(
                    f,
                    config,
                    state,
                    condition.ok_or_else(|| {
                        TaskError::NoCondition(
                            "Missing condition function for `TaskMode::Conditional`".to_string(),
                        )
                    })?,
                )),
                TaskMode::Duration(_) => Ok(Task::_duration(f, config, state, Instant::now())),
            }
        }*/

        /// Starts a `Task` with a infinite structure, only checking cancelation
        fn _infinite(mut f: F, config: TaskConfig, state: S) -> Self {
            let cancelled = Arc::new(RwLock::new(false));
            let cancelled_clone = cancelled.clone();

            let state = Arc::new(RwLock::new(state));
            let state_clone = state.clone();

            let handle = tokio::spawn(async move {
                state_clone.write().await.on_task_start();
                let mut iteration = 0usize;
                let mut interval = tokio::time::interval(config.interval);

                loop {
                    // Check if cancelled
                    if let true = *cancelled_clone.read().await {
                        break;
                    }

                    // Execute the closure
                    let result = f(iteration, &state_clone).await;

                    // Update the state
                    {
                        // Check if last result causes a stop
                        if config.stop_on_error {
                            if let Err(_) = result {
                                Task::set_state(&mut *state_clone.write().await, iteration, result)
                                    .await;
                                break;
                            }
                        }

                        Task::set_state(&mut *state_clone.write().await, iteration, result).await;
                    }

                    // Check interval bounds
                    if iteration >= usize::max_value() {
                        iteration = 0;
                    }

                    iteration += 1;
                    interval.tick().await;
                }

                // Call `Task` complete function and mark state as not running
                let mut state = state_clone.write().await;
                state.on_task_complete();
                state.set_is_running(false);
            });

            Self {
                handle: Some(handle),
                panicked: Arc::new(RwLock::new(false)),
                cancelled,
                state,
                _phantom: PhantomData::<(T, E)>,
            }
        }

        /// Starts a `Task` with a fixed structure, checking cancelation along with iterations
        fn _fixed(mut f: F, config: TaskConfig, state: S) -> Self {
            let cancelled = Arc::new(RwLock::new(false));
            let cancelled_clone = cancelled.clone();

            let state = Arc::new(RwLock::new(state));
            let state_clone = state.clone();

            let handle = tokio::spawn(async move {
                state_clone.write().await.on_task_start();
                let mut iteration = 0usize;
                let mut interval = tokio::time::interval(config.interval);

                loop {
                    // Check if cancelled
                    if let true = *cancelled_clone.read().await {
                        break;
                    }

                    // Check iterations completion
                    if Task::check_iterations(iteration, &*state_clone.read().await).await {
                        break;
                    }

                    // Execute the closure
                    let result = f(iteration, &state_clone).await;

                    // Update the state
                    {
                        // Check if last result causes a stop
                        if config.stop_on_error {
                            if let Err(_) = result {
                                Task::set_state(&mut *state_clone.write().await, iteration, result)
                                    .await;
                                break;
                            }
                        }
                        Task::set_state(&mut *state_clone.write().await, iteration, result).await;
                    }

                    // Check interval bounds
                    if iteration >= usize::max_value() {
                        iteration = 0;
                    }

                    iteration += 1;
                    interval.tick().await;
                }

                // Call `Task` complete function and mark state as not running
                let mut state = state_clone.write().await;
                state.on_task_complete();
                state.set_is_running(false);
            });

            Self {
                handle: Some(handle),
                panicked: Arc::new(RwLock::new(false)),
                cancelled,
                state,
                _phantom: PhantomData::<(T, E)>,
            }
        }

        /*/// Starts a `Task` with a conditional structure, checking cancelation along with conditions
        fn _conditional<C, FutC>(
            mut f: F,
            config: TaskConfig,
            state: S,
            mut condition: C,
        ) -> Self
        where
            C: FnMut(&S) -> FutC + Send + Sync + 'static,
            FutC: Future<Output = bool> + Send + Sync + 'static,
        {
            let cancelled = Arc::new(RwLock::new(false));
            let cancelled_clone = cancelled.clone();

            let state = Arc::new(RwLock::new(state));
            let state_clone = state.clone();

            let handle = tokio::spawn(async move {
                state_clone.write().await.on_task_start();
                let mut iteration = 0usize;
                let mut interval = tokio::time::interval(config.interval);

                loop {
                    // Check if cancelled
                    if let true = *cancelled_clone.read().await {
                        break;
                    }

                    // Check completion conditions
                    if condition(&*state_clone.read().await).await {
                        break;
                    }

                    // Execute the closure
                    let result = f(iteration, &state_clone).await;

                    // Update the state
                    {
                        // Check if last result causes a stop
                        if config.stop_on_error {
                            if let Err(_) = result {
                                Task::set_state(&mut *state_clone.write().await, iteration, result)
                                    .await;
                                break;
                            }
                        }
                        Task::set_state(&mut *state_clone.write().await, iteration, result).await;
                    }

                    // Check interval bounds
                    if iteration >= usize::max_value() {
                        iteration = 0;
                    }

                    iteration += 1;
                    interval.tick().await;
                }

                // Call `Task` complete function and mark state as not running
                let mut state = state_clone.write().await;
                state.on_task_complete();
                state.set_is_running(false);
            });

            Self {
                handle: Some(handle),
                panicked: Arc::new(RwLock::new(false)),
                cancelled,
                state,
                _phantom: PhantomData::<(T, E)>,
            }
        }*/

        /// Starts a `Task` with a duration structure, checking cancelation along with elapsed time
        fn _duration(mut f: F, config: TaskConfig, state: S, start_time: Instant) -> Self {
            let cancelled = Arc::new(RwLock::new(false));
            let cancelled_clone = cancelled.clone();

            let state = Arc::new(RwLock::new(state));
            let state_clone = state.clone();

            let handle = tokio::spawn(async move {
                state_clone.write().await.on_task_start();
                let mut iteration = 0usize;
                let mut interval = tokio::time::interval(config.interval);

                loop {
                    // Check if cancelled
                    if let true = *cancelled_clone.read().await {
                        break;
                    }

                    // Check duration completion
                    if Task::check_duration(start_time, &*state_clone.read().await).await {
                        break;
                    }

                    // Execute the closure
                    let result = f(iteration, &state_clone).await;

                    // Update the state
                    {
                        // Check if last result causes a stop
                        if config.stop_on_error {
                            if let Err(_) = result {
                                Task::set_state(&mut *state_clone.write().await, iteration, result)
                                    .await;
                                break;
                            }
                        }
                        Task::set_state(&mut *state_clone.write().await, iteration, result).await;
                    }

                    // Check interval bounds
                    if iteration >= usize::max_value() {
                        iteration = 0;
                    }

                    iteration += 1;
                    interval.tick().await;
                }

                // Call `Task` complete function and mark state as not running
                let mut state = state_clone.write().await;
                state.on_task_complete();
                state.set_is_running(false);
            });

            Self {
                handle: Some(handle),
                panicked: Arc::new(RwLock::new(false)),
                cancelled,
                state,
                _phantom: PhantomData::<(T, E)>,
            }
        }
    }

    /// Creates a `Task` that runs until a condition is met, with the default `TaskConfig`
    pub fn until_condition<F, Fut, C, FutC>(f: F, state: S, condition: C) -> Result<Self, TaskError>
    where
        F: FnMut(usize, &Arc<RwLock<S>>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send + Sync + 'static,
        C: FnMut(&S) -> FutC + Send + Sync + 'static,
        FutC: Future<Output = bool> + Send + Sync + 'static,
    {
        let config = TaskConfig::new(TaskMode::Conditional);
        Self::with_config(f, config, state, Some(condition))
    }

    /// Creates a `Task` with a specific `TaskConfig`
    pub fn with_config<F, Fut, C, FutC>(
        f: F,
        config: TaskConfig,
        state: S,
        condition: Option<C>,
    ) -> Result<Self, TaskError>
    where
        F: FnMut(usize, &Arc<RwLock<S>>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send + Sync + 'static,
        C: FnMut(&S) -> FutC + Send + Sync + 'static,
        FutC: Future<Output = bool> + Send + Sync + 'static,
    {
        match config.mode {
            TaskMode::Infinite => Ok(Task::_infinite(f, config, state)),
            TaskMode::Fixed(_) => Ok(Task::_fixed(f, config, state)),
            TaskMode::Conditional => Ok(Task::_conditional(
                f,
                config,
                state,
                condition.ok_or_else(|| {
                    TaskError::NoCondition(
                        "Missing condition function for `TaskMode::Conditional`".to_string(),
                    )
                })?,
            )),
            TaskMode::Duration(_) => Ok(Task::_duration(f, config, state, Instant::now())),
        }
    }

    /// Starts a `Task` with a conditional structure, checking cancelation along with conditions
    fn _conditional<F, Fut, C, FutC>(
        mut f: F,
        config: TaskConfig,
        state: S,
        mut condition: C,
    ) -> Self
    where
        F: FnMut(usize, &Arc<RwLock<S>>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send + Sync + 'static,
        C: FnMut(&S) -> FutC + Send + Sync + 'static,
        FutC: Future<Output = bool> + Send + Sync + 'static,
    {
        let cancelled = Arc::new(RwLock::new(false));
        let cancelled_clone = cancelled.clone();

        let state = Arc::new(RwLock::new(state));
        let state_clone = state.clone();

        let handle = tokio::spawn(async move {
            state_clone.write().await.on_task_start();
            let mut iteration = 0usize;
            let mut interval = tokio::time::interval(config.interval);

            loop {
                // Check if cancelled
                if let true = *cancelled_clone.read().await {
                    break;
                }

                // Check completion conditions
                if condition(&*state_clone.read().await).await {
                    break;
                }

                // Execute the closure
                let result = f(iteration, &state_clone).await;

                // Update the state
                {
                    // Check if last result causes a stop
                    if config.stop_on_error {
                        if let Err(_) = result {
                            Task::set_state(&mut *state_clone.write().await, iteration, result)
                                .await;
                            break;
                        }
                    }
                    Task::set_state(&mut *state_clone.write().await, iteration, result).await;
                }

                // Check interval bounds
                if iteration >= usize::max_value() {
                    iteration = 0;
                }

                iteration += 1;
                interval.tick().await;
            }

            // Call `Task` complete function and mark state as not running
            let mut state = state_clone.write().await;
            state.on_task_complete();
            state.set_is_running(false);
        });

        Self {
            handle: Some(handle),
            panicked: Arc::new(RwLock::new(false)),
            cancelled,
            state,
            _phantom: PhantomData::<(T, E)>,
        }
    }

    /// Sets the current iteration and result for the `Task`
    async fn set_state(state: &mut S, iteration: usize, result: Result<T, E>) {
        state.set_iteration(iteration + 1);
        state.set_last_result(result);
    }

    /// Checks if the `Task` should stop based on the `TaskMode` and number of iterations
    async fn check_iterations(iteration: usize, state: &S) -> bool {
        match state.get_mode() {
            TaskMode::Duration(_) | TaskMode::Infinite | TaskMode::Conditional => false,
            TaskMode::Fixed(max_iters) => iteration >= *max_iters,
        }
    }

    /// Check if the `Task` should stop based on the `TaskMode` and the start_time
    async fn check_duration(start_time: Instant, state: &S) -> bool {
        match state.get_mode() {
            TaskMode::Fixed(_) | TaskMode::Infinite | TaskMode::Conditional => false,
            TaskMode::Duration(duration) => start_time.elapsed() >= *duration,
        }
    }

    /// Gets the `Task` current state
    pub async fn state(&self) -> S {
        let state = self.state.read().await;
        state.clone()
    }

    pub async fn last_result(&self) -> Option<Result<T, E>> {
        self.state.read().await.get_last_result().clone()
    }

    pub async fn is_panic(&mut self) -> bool {
        let handle_done = match &self.handle {
            Some(handle) => handle.is_finished(),
            None => true,
        };
        match handle_done {
            false => false, // Has handle that isnt finished
            true => match self.handle.take() {
                None => *self.panicked.read().await, // No handle, check panicked state
                Some(handle) => match handle.await {
                    Ok(_) => false,
                    Err(e) => {
                        let is_panic = e.is_panic();
                        if is_panic {
                            *self.panicked.write().await = true;
                        }
                        is_panic
                    }
                },
            },
        }
    }

    /// Checks if the `Task` is currently running
    pub async fn is_running(&self) -> bool {
        match &self.handle {
            Some(handle) => !handle.is_finished(),
            None => false,
        }
    }

    ///Wait for the `Task` to finish naturally
    pub async fn wait_for_complete(&mut self) -> Option<Result<T, E>> {
        let panicked = match self.handle.take() {
            Some(handle) => match handle.await {
                Err(e) => e.is_panic(),
                Ok(_) => false,
            },
            None => false,
        };
        // Set panicked state and return `None`
        if panicked || *self.panicked.read().await {
            // Only set panic if task finished this call
            if panicked {
                *self.panicked.write().await = true;
            }
            return None;
        }
        self.last_result().await
    }

    /// Cancel the `Task` and let it finish gracefully
    pub async fn stop(&mut self) {
        // Signal cancelled
        {
            let mut cancelled = self.cancelled.write().await;
            *cancelled = true;
        }

        // Wait for the `Task` to finish
        if let Some(handle) = self.handle.take() {
            let _ = handle.await;
        }

        // Ensure state is updated
        self.state.write().await.set_is_running(false);
    }

    /// Stop the `Task` immediately
    pub fn abort(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }

        // Update the state without blocking
        let state = self.state.clone();
        let cancelled = self.cancelled.clone();
        tokio::spawn(async move {
            state.write().await.set_is_running(false);
            *cancelled.write().await = true;
        });
    }

    /// Returns the `TaskMode` by cloning it from the `TaskState`
    pub fn get_mode(&self) -> TaskMode {
        self.state.blocking_read().get_mode().clone()
    }

    /// Returns the number of iteration ran by reading it from the `TaskState``
    pub fn get_iterations(&self) -> usize {
        self.state.blocking_read().get_iterations()
    }

    /// Returns the last result by cloning it from the `TaskState``
    pub fn get_last_result(&self) -> Option<Result<T, E>> {
        self.state.blocking_read().get_last_result()
    }
}

#[cfg(test)]
mod tests {
    use crate::{task::WithTaskState, Task, TaskMode};

    #[tokio::test]
    async fn infinite_task() {
        assert!(Task::new(
            |i, state| {
                let data = state.blocking_read().inner_clone();
                println!("outer {}: {}", i, data[i]);
                async move {
                    println!("inner {}: {}", i, data[i]);
                    Ok::<(), ()>(())
                }
            },
            vec![1, 2, 3, 4, 5].with_task_state(TaskMode::Fixed(5))
        )
        .wait_for_complete()
        .await
        .is_some_and(|res| res.is_ok()));
    }

    #[tokio::test]
    async fn fixed_task() {}

    #[tokio::test]
    async fn conditional_task() {}

    #[tokio::test]
    async fn duration_task() {}
}
