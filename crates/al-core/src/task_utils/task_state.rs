use crate::{TaskMode, TaskStateRequirements, TaskTypes};

mod sealed {
    use crate::{BaseTaskState, ExtendedTaskState, TaskStateRequirements, TaskTypes};

    pub trait Sealed {}
    impl<T: TaskTypes, E: TaskTypes> Sealed for BaseTaskState<T, E> {}
    impl<T: TaskTypes, E: TaskTypes, S: TaskStateRequirements> Sealed for ExtendedTaskState<T, E, S> {}
}

/// `TaskState` contains all required functions hooks and should hold all values a `Task` tracks between iterations
pub trait TaskState<T: TaskTypes = (), E: TaskTypes = ()>: TaskModeSetter + TaskStateRequirements {
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

pub trait TaskModeSetter: sealed::Sealed {
    fn set_mode(&mut self, mode: TaskMode);
}

/// `BaseTaskState` contains all values a `Task` tracks between iterations
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

impl<T: TaskTypes, E: TaskTypes> TaskModeSetter for BaseTaskState<T, E> {
    fn set_mode(&mut self, mode: TaskMode) {
        self.mode = mode
    }
}

/// Impl `TaskState` for `BaseTaskState`, allowing `Tasks` access to the interal data
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

/// `WithTaskState` allows any type with `'static + Send + Sync + Clone` to use `as_task_state()` and `with_task_state(mode)`
pub trait WithTaskState<T: TaskTypes, E: TaskTypes>: TaskStateRequirements {
    fn as_task_state(self) -> ExtendedTaskState<T, E, Self>;
    fn with_task_state(self, mode: TaskMode) -> ExtendedTaskState<T, E, Self>;
}

/// Blanket impl for all types `'static + Send + Sync + Clone`
impl<T: TaskTypes, E: TaskTypes, S: TaskStateRequirements> WithTaskState<T, E> for S {
    fn as_task_state(self) -> ExtendedTaskState<T, E, Self> {
        Self::with_task_state(self, TaskMode::default())
    }

    fn with_task_state(self, mode: TaskMode) -> ExtendedTaskState<T, E, Self> {
        ExtendedTaskState::new(mode, self)
    }
}

/// `ExtendedTaskState` holds extra state not strictly required by `Task`, passed by the user.
/// `ExtendedTaskState` is blanket implemented for all types `'static + Send + Sync + Clone`
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

/// Impl `ExtendedTaskState` exposing extended to be used in `Task` iterations
impl<T: TaskTypes, E: TaskTypes, S: TaskStateRequirements> ExtendedTaskState<T, E, S> {
    pub fn new(mode: TaskMode, extended: S) -> Self {
        Self {
            base: BaseTaskState::new(mode),
            extended,
        }
    }
    
    /// Sets `extended` to the passed `S`
    pub fn set_inner(&mut self, inner: S) {
        self.extended = inner
    }

    /// Returns the extended state
    pub fn into_inner(&self) -> &S {
        &self.extended
    }

    /// Returns a clone of the extended state
    pub fn inner_clone(&self) -> S {
        self.extended.clone()
    }
}

impl<T: TaskTypes, E: TaskTypes, S: TaskStateRequirements> TaskModeSetter for ExtendedTaskState<T, E, S> {
    fn set_mode(&mut self, mode: TaskMode) {
        self.base.set_mode(mode)
    }
}

/// Blanket impl `TaskState` for every `ExtendedTaskState` by delegating to `BaseTaskState`
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