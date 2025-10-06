use crate::{TaskConfig, TaskError, TaskMode, TaskState, TaskTypes};
use al_derive::with_bounds;
use std::sync::Arc;
use std::{future::Future, marker::PhantomData};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::Instant;

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

/// Implement `Task`, generating the actual functions with the `with_common_bounds` macro
impl<T: TaskTypes, E: TaskTypes, S: TaskState<T, E>> Task<T, E, S> {
    /// Constant to allow `Task::NO_CONDITION` rather than specifying `None::<...>`
    pub const NO_CONDITION: Option<
        fn(&Arc<RwLock<S>>) -> std::pin::Pin<Box<dyn Future<Output = bool> + Send + Sync>>,
    > = None;

    #[with_bounds(C)]
    pub fn some_condition(_: &S, condition: C) -> Option<C> {
        Some(condition)
    }

    /// Creates a `Task` with the default `TaskConfig`
    #[with_bounds(F)]
    pub fn infinite(f: F, state: S) -> Self {
        let mut state = state;
        state.set_mode(TaskMode::Infinite);
        Self::_infinite(f, TaskConfig::default(), state)
    }

    /// Creates a `Task` that runs a fixed number of times, with the default `TaskConfig`
    #[with_bounds(F)]
    pub fn fixed(iterations: usize, f: F, state: S) -> Self {
        let mut state = state;
        state.set_mode(TaskMode::Fixed(iterations));
        Self::_fixed(f, TaskConfig::default(), state)
    }

    /// Creates a `Task` that runs for a specific duration, with the default `TaskConfig`
    #[with_bounds(F)]
    pub fn for_duration(duration: std::time::Duration, f: F, state: S) -> Self {
        let mut state = state;
        state.set_mode(TaskMode::Duration(duration));
        Self::_duration(f, TaskConfig::default(), state, Instant::now())
    }

    /// Creates a `Task` that runs until a condition is met, with the default `TaskConfig`
    #[with_bounds(F, C)]
    pub fn until_condition(f: F, state: S, condition: C) -> Self {
        let mut state = state;
        state.set_mode(TaskMode::Conditional);
        Self::_conditional(f, TaskConfig::default(), state, condition)
    }

    /// Creates a `Task` with a specific `TaskConfig`
    #[with_bounds(F, C)]
    pub fn with_config(
        f: F,
        config: TaskConfig,
        state: S,
        condition: Option<C>,
    ) -> Result<Self, TaskError> {
        match state.get_mode() {
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

    /// Starts a `Task` with a infinite structure, only checking cancelation
    #[with_bounds(F)]
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
    #[with_bounds(F)]
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

    /// Starts a `Task` with a conditional structure, checking cancelation along with conditions
    #[with_bounds(F, C)]
    fn _conditional(mut f: F, config: TaskConfig, state: S, mut condition: C) -> Self {
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
                if condition(&state_clone).await {
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

    /// Starts a `Task` with a duration structure, checking cancelation along with elapsed time
    #[with_bounds(F)]
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

    ///Wait for the result of the `Task` finishing naturally
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

    /// Cancel the `Task` and wait for the result
    pub async fn stop_and_wait(&mut self) -> Option<Result<T, E>> {
        self.cancel().await;
        self.wait_for_complete().await
    }

    /// Cancel the `Task` and let it finish gracefully
    pub async fn cancel(&mut self) {
        // Signal cancelled
        {
            let mut cancelled = self.cancelled.write().await;
            *cancelled = true;
        }
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
    use crate::{BaseTaskState, Task, TaskConfig, TaskMode, WithTaskState};
    use std::time::Duration;
    use tokio::time::{sleep, Instant};

    #[tokio::test]
    async fn infinite_task() {
        let duration = 5;
        let mut task = Task::infinite(
            |i, _| async move { Ok::<_, ()>(i) },
            BaseTaskState::default(),
        );
        sleep(Duration::from_secs(duration)).await;
        assert!(task
            .stop_and_wait()
            .await
            .is_some_and(|res| res.is_ok_and(|i| {
                println!("Iterations: {}", i);
                i > duration as usize
            })))
    }

    #[tokio::test]
    async fn fixed_task() {
        assert!(Task::fixed(
            5,
            |i, state| {
                let state = state.clone();
                async move {
                    let x = state.read().await.into_inner()[i];
                    assert!((i + 1) == x);
                    Ok::<_, ()>(x)
                }
            },
            vec![1usize, 2, 3, 4, 5].as_task_state()
        )
        .wait_for_complete()
        .await
        .is_some_and(|res| res.is_ok_and(|i| i == 5)));
    }

    #[tokio::test]
    async fn conditional_task() {
        let target_iteration = 20;
        assert!(Task::until_condition(
            move |i, state| {
                let state = state.clone();
                async move {
                    let mut state = state.write().await;
                    assert!(*state.into_inner() == false);
                    if i >= target_iteration {
                        state.set_inner(true);
                    }
                    Ok::<_, ()>(i)
                }
            },
            false.as_task_state(),
            |state| {
                let state = state.clone();
                async move { state.read().await.inner_clone() }
            }
        )
        .wait_for_complete()
        .await
        .is_some_and(|res| res.is_ok_and(|i| i == target_iteration)));
    }

    #[tokio::test]
    async fn duration_task() {
        // Spawn a series of tasks with increasing durations
        let mut tasks = Vec::new();
        for x in 1..31 {
            let duration = Duration::from_secs(x);
            let expected_end = Instant::now() + duration;
            tasks.push((
                x,
                expected_end,
                Task::for_duration(
                    duration,
                    |_, _| async move { Ok::<_, ()>(Instant::now()) },
                    BaseTaskState::default(),
                ),
            ));
        }

        // Wait for each task to end, checking the difference between the expected end `Instant` and the last result `Instant` is <= .1 seconds
        let mut avg = 0usize;
        let len = tasks.len();
        for (x, expected_end, mut task) in tasks {
            assert!(task
                .wait_for_complete()
                .await
                .is_some_and(|res| res.is_ok_and(|i| {
                    let diff = expected_end - i;
                    println!(
                        "Difference {:?}: {:?} ({} millis) {:?} {:?}",
                        x,
                        diff,
                        diff.as_millis(),
                        expected_end,
                        i
                    );
                    avg += diff.as_millis() as usize;
                    diff.as_millis() <= 100
                })));
        }
        println!("Average /{}: {} millis", len, avg / len)
    }

    #[tokio::test]
    async fn tasks_with_config() {
        // Infinite
        let duration = 5;
        let mut task = Task::with_config(
            |i, _| async move { Ok::<_, ()>(i) },
            TaskConfig::default(),
            BaseTaskState::new(TaskMode::Infinite),
            Task::NO_CONDITION,
        )
        .unwrap();
        sleep(Duration::from_secs(duration)).await;
        assert!(task
            .stop_and_wait()
            .await
            .is_some_and(|res| res.is_ok_and(|i| i > duration as usize)));

        // Fixed
        let list = vec![1usize, 2, 3, 4, 5];
        assert!(Task::with_config(
            |i, state| {
                let state = state.clone();
                async move {
                    let x = state.read().await.into_inner()[i];
                    assert!((i + 1) == x);
                    Ok::<_, ()>(x)
                }
            },
            TaskConfig::default(),
            list.clone().with_task_state(TaskMode::Fixed(list.len())),
            Task::NO_CONDITION
        )
        .unwrap()
        .wait_for_complete()
        .await
        .is_some_and(|res| res.is_ok_and(|i| i == *list.last().unwrap())));

        // Conditional
        let target_iteration = 20;
        let cond_state = false.with_task_state(TaskMode::Conditional);
        assert!(Task::with_config(
            move |i, state| {
                let state = state.clone();
                async move {
                    let mut state = state.write().await;
                    assert!(*state.into_inner() == false);
                    if i >= target_iteration {
                        state.set_inner(true);
                    }
                    Ok::<_, ()>(i)
                }
            },
            TaskConfig::default(),
            cond_state.clone(),
            Task::some_condition(&cond_state, |state| {
                let state = state.clone();
                async move { state.read().await.inner_clone() }
            })
        )
        .unwrap()
        .wait_for_complete()
        .await
        .is_some_and(|res| res.is_ok_and(|i| i == target_iteration)));

        // Duration
        // Spawn a series of tasks with increasing durations
        let mut tasks = Vec::new();
        for x in 1..31 {
            let duration = Duration::from_secs(x);
            let expected_end = Instant::now() + duration;
            tasks.push((
                expected_end,
                Task::with_config(
                    |_, _| async move { Ok::<_, ()>(Instant::now()) },
                    TaskConfig::default(),
                    BaseTaskState::new(TaskMode::Duration(duration)),
                    Task::NO_CONDITION,
                )
                .unwrap(),
            ));
        }

        // Wait for each task to end, checking the difference between the expected end `Instant` and the last result `Instant` is <= .1 seconds
        for (expected_end, mut task) in tasks {
            assert!(task
                .wait_for_complete()
                .await
                .is_some_and(|res| res.is_ok_and(|i| { (expected_end - i).as_millis() <= 100 })));
        }
    }
}
