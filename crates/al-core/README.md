# Overview
The core consists of a command message system that provides commonly used commands along with a dynamic event command.

The `Event` trait allows defining custom types that can be passed through the command system using the event command variant.

---
# Commands
Commands are simply predefined messages that can be passed through the system.

In practice, commands are defined as a variant of the `Command` enum which holds built-in commands such as `Pulse`, `Stop`, and `Restart`. A full list of built-in commands can be found in the section below.

To allow runtime extension within a system built on compile-time enum variants, an additional built-in `Event` command is provided that holds a `Box<dyn Event>`. The variant allows any `dyn Event` generated at runtime to be boxed and passed through the system.

This flexibility allows the system to be used by code dynamically loaded at runtime—for example, plugins with a WASM runtime—or to route `Event` commands through the system without knowing the details of the underlying `Box<dyn Event>`.

Boxing events with `Box<dyn Event>` is used rather than a simple reference, such as `&dyn Event`, due to trait object ownership and lifetime constraints when storing, returning from functions, transferring between threads, or using dynamic dispatch with owned data.
## Built-In Commands
- Event(`Box<dyn Event>`)
- Pulse
- Stop
- Restart

# Events
The `Event` trait can be added to any type with a `'static` lifetime and the traits `Send + Sync + Any` by using the `#[event]` attribute macro.

The `#[event]` attribute macro will add the required traits by generating the derive attribute`#[derive(Clone + Default + PartialEq + Debug + Hash + EventMarker)]`. If the traits need to be implemented manually, the `EventMarker` trait macro can be manually derived alongside them.

The `Event` trait acts as a wrapper to expose the required functionality, such as hashing or cloning, of the implementing type to the system through a common interface while maintaining dyn object compatibility.

## Examples
### Custom Commands
The simplest events with no data—such as the types below showing a standalone struct and enum representing multiple events—are effectively command extensions allowing custom commands to be passed through the system as an event variant.
```Rust
#[event]
struct NewCommand;

#[event]
enum NewCommands {
   CommandOne,
   CommandTwo,
}
```
### With Data
An event that holds data, such as a key press action, could potentially be modeled with a type similar to either of the following structs: Any types used within an `Event` need to implement the same traits the `Event` requires, through deriving or otherwise, as the `ActionType` enum does.
```Rust
#[derive(Clone, Default, PartialEq, Debug, Hash)]
enum ActionType {
   #[default]
   Pressed,
   Held,
   Released,
}

#[event]
struct KeyAction(char, ActionType);

#[event]
struct KeyAction {
   key: char,
   action: ActionType,
}
```

# Serialization
If the `serde` crate feature is enabled, both the `Command` enum and any types implementing `Event` will require the `serde::Serialize` and `serde::Deserialize` traits. The `#[event]` macro will attempt to add them along with the other required traits.

To support concrete deserialization using `dyn Event`, even with non-self-describing formats, an event `Registry` is used. Once registered with either the `register_event!(MyEvent)` macro or calling `my_event.register()`, any `MyEvent` can be deserialized from its `dyn Event` serialization. To facilitate this, every `dyn Event` is serialized in the tuple format `(type_name, type_data)`. Then when deserializing, the `type_name` is extracted to request the corresponding logic from the `Registry`.

If the type has any generics, each generic must be explicitly declared when registering events. For example, `MyEvent<u8>`, `MyEvent<i8>` and `MyEvent<String>` all register as different events as each generic type is deserialized differently.

## Serialization Formats
With the `serde` feature, the `SerdeFormat` trait is also enabled with the intention of abstracting format differences behind a shared interface. `SerdeFormat` holds functions to Serialize and Deserialize both `Command` and `Event` types using `[u8]` byte slices.

While helpful, a `SerdeFormat` implementation isn't strictly required as the `Registry` code is contained inside the logic for `dyn Event` and will be used regardless of the serialization method.

`JsonFormat` and `BinaryFormat` are provided behind the `json` and `binary` features respectively. `JsonFormat` generates UTF-8 strings for human readability and general use cases while `BinaryFormat` can be used for faster, more compact serialization.
### Custom Formats
To add custom formats, implement the `SerdeFormat` trait and add the custom format code inside each respective function.
## Examples
### Commands
```Rust
let cmd_bytes: [u8] = JsonSerde.serialize_commamd(&my_cmd);
let cmd: Command = JsonSerde.deserialize_command(&cmd_bytes);
```
### Events
```Rust
let event_bytes: [u8] = BinarySerde.serialize_event(&my_event);
let event: MyEvent = BinarySerde.deserialize_event(&event_bytes);
```
### Custom Format
```Rust
struct CustomSerde;

impl SerdeFormat for CustomSerde {
   fn ...
}
```

# Transports
The `Transport<T>` trait represents something with the ability to move a type T, as in, calling `transport.send(t)` or `transport.send_blocking(t)` should result in the same `t`—albeit potentially modified with the `Transport<T>` internal logic—being attainable through `transport.recv()` or `transport.recv_blocking()`.
## Queue
The `Queue<T>` struct implements `Transport<T>` with access to its internal `VecDeque<T>` supporting a FIFO order.
## Pipeline
The `Pipeline<T>` enum is designed to recursively allow multiple `Transport<T>` to be combined together into a single `Pipeline<T>` transport through the `Transform` or `Link` variants. This allows all the transports comprising the internal pipeline to be abstracted behind the outer pipelines own `.send(t)`/`.send_blocking(t)` or `.recv()`/`.recv_blocking()`. For references, look to the `Examples` section below.
### Transform
The `Transform(Arc<Transport<T>>, Arc<Fn(T) -> T + Send + Sync>, Arc<Fn(T) -> T + Send + Sync>)` pipeline variant allows any function that takes and returns the same type as the `Transport<T>` to be passed.

The first `Arc<Fn(T) -> T + Send + Sync>` only activates on `transform.send(t)` or `transform.send_blocking(t)` while the second only activates on `transform.recv()` or `trasnform.recv_blocking()`.

The `Pipeline<T>::NoOP(t)` function is provided to directly return the given `t`, allowing `Transform` stages to effectively be skipped.

### Link
The `Link(Arc<Transport<T>>, Arc<Transport<T>>, LinkTask<T>)` variant can be constructed through `Pipeline<T>::link(Arc<Transport<T>>, Arc<Transport<T>>)` which will handle creating an infinite `Task` to be used for the `LinkTask<T>`. Alternatively, an `Arc<dyn Task>` can be passed for flexibility on how the link functions.
## Splice
The `Splice<F,T>` struct is designed to allow two pipelines of differing types to be spliced together from type `F` to type `T`.
## Examples
### Point to Point
### Transform
### Spliced
### Publish Subscribe

# Tasks
