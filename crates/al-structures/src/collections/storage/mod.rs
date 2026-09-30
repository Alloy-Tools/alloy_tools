//! Thread‑safe, extensible storages with a shared interface.
//!
//! # Choosing a storage handle
//!
//! The two core implementations are [`RwLockStorage`] and [`CowStorage`].
//! Both implement the same traits (`...HandleRead`, `...HandleWrite`,
//! `...Handle`) and can be used interchangeably through static dispatch
//! (`impl ...Handle`) or dynamic dispatch (`...DynHandle`).
//!
//! | | **RwLockStorage** | **CowStorage** |
//! |---|---|---|
//! | **Underlying lock** | `RwLock<Storage>` | `ArcSwap<Storage>` + a mutex for writes |
//! | **Read performance** | Acquires a read‑lock (no contention is fast; a waiting writer can block readers) | Lock‑free, wait‑free – always instant |
//! | **Write performance** | O(1) in‑place mutation under a write lock | Clones the entire map on every write (O(n)), serialised by a mutex |
//! | **Memory** | Single live copy of the map | Old copies live until the last reader drops them |
//! | **When to use** | Write‑heavy workloads or when you need in‑place mutation | Read‑dominated workloads or when read latency matters most |
//!
//! For bulk operations that need direct access to the underlying storage,
//! both registries implement [`HandleBulkRead`] and [`HandleBulkWrite`].

mod cow_handle;
mod rwlock_handle;
pub mod utils;

pub use cow_handle::CowStorage;
pub use rwlock_handle::RwLockStorage;

/// Defines a globally accessible, lazily initialized singleton
/// with per‑thread caching for unsynchronized read access.
///
/// The macro expands to:
/// - A `LazyLock` that initializes `T` once on first global access,
///   then `Box::leak`s it to obtain a `&'static T`.
/// - A `thread_local!` `LazyCell` that caches this `&'static T` once per thread,
///   avoiding any global synchronization on subsequent accesses.
/// - A function `$NAME()` returning `&'static T` via the thread‑local cache
///   (the fast path, no atomics after the first call per thread).
/// - A function `RAW_$NAME()` returning `&'static T` directly from the global
///   `LazyLock` (bypasses thread‑local caching, but pays an atomic load each time).
///
/// # Syntax
/// Invoke the macro with the following pattern:
///
/// ```ignore
/// global_thread_local! {
///     [$(#[$meta])*]
///     [pub] static $NAME: $type = $init;
/// }
/// ```
///
/// - **`$meta`** – optional attributes (eg `#[doc(hidden)]`) applied to the generated items.
/// - **`pub`** – optional visibility; the generated functions will inherit this.
/// - **`$NAME`** – the identifier for the singleton (used as the main accessor function).
/// - **`$type`** – the type of the singleton; must be `Send + Sync`.
/// - **`$init`** – an expression that produces a value of `$type`, executed exactly once.
///
/// # Performance Characteristics
/// - **First call to `$NAME()` in a thread**: initializes the `LazyCell`
///   (does a single atomic load from `LazyLock` to retrieve the `&'static T`).
/// - **All subsequent calls to `$NAME()` from the same thread**: a straight
///   pointer dereference — no synchronization, no atomic operations,
///   no locks.
/// - **`RAW_$NAME()`**: always performs the `LazyLock`'s internal atomic check
///   (lightweight, but not zero‑cost).
///
/// # Pros
/// - **Zero‑boilerplate** – just call the generated function.
/// - **One‑time global initialization** – expensive setup runs only once.
/// - **Thread‑local caching** gives near‑zero overhead on hot paths.
///
/// # Cons
/// - The global is intentionally leaked (`Box::leak`) and **will never be dropped**.
///   This is generally desirable for singletons that live for the entire program.
/// - Requires `T: Send + Sync` because the same `&'static T` is shared across threads.
/// - The `RAW_$NAME()` escape hatch incurs the `LazyLock` atomic check,
///   so prefer `$NAME()` for repeated access inside a thread.
///
/// # Example
/// ```ignore
/// al_structures::global_thread_local! {
///     pub static MY_TYPE: MyType = MyType::new();
/// }
///
/// // Use it anywhere, from any thread:
/// let value = MY_TYPE().some_method();
/// ```
///
/// # Generated Items
/// - `$NAME()` – the main accessor (cached per thread).
/// - `RAW_$NAME()` – the uncached escape hatch.
/// - `GLOBAL_$NAME` and `LOCAL_$NAME` are also defined, but are internal
///   implementation details and should not be used directly.
#[macro_export]
macro_rules! global_thread_local {
    (
        $(#[$meta:meta])*
        $vis:vis static $NAME:ident: $T:ty = $init:expr;
    ) => {
        $crate::paste! {
            // Global singleton leaked to get a 'static lifetime.
            $(#[$meta])*
            static [< GLOBAL_ $NAME >]: ::std::sync::LazyLock<&'static $T> = ::std::sync::LazyLock::new(|| {
                ::std::boxed::Box::leak(::std::boxed::Box::new($init))
            });

            // Per-thread cache initialized once per thread to avoid repeated atomic checks.
            ::std::thread_local! {
                static [< LOCAL_ $NAME >]: ::std::cell::LazyCell<&'static $T> = ::std::cell::LazyCell::new(|| {
                    *[< GLOBAL_ $NAME >]
                });
            }

            // Public accessor returns a raw pointer reference avoiding the global synchronization.
            $(#[$meta])*
            #[allow(non_snake_case)]
            $vis fn $NAME() -> &'static $T {
                [< LOCAL_ $NAME >].with(|cache| **cache)
            }

            // Escape hatch for raw access
            $(#[$meta])*
            #[allow(non_snake_case)]
            #[allow(dead_code)]
            $vis fn [< RAW_ $NAME >]() -> &'static $T {
                *[< GLOBAL_ $NAME >]
            }
        }
    };
}
