use std::marker::PhantomData;

use crate::{Command, Event, Query};

pub type DynMessage = Message<Box<dyn Command>, Box<dyn Event>, Box<dyn Query>>;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(
        bound = "Cmd: serde::Serialize + serde::de::DeserializeOwned, Evt: serde::Serialize + serde::de::DeserializeOwned, Qry: serde::Serialize + serde::de::DeserializeOwned"
    )
)]
#[derive(Debug)]
pub enum Message<Cmd, Evt, Qry> {
    Command(Cmd),
    Event(Evt),
    Query(Qry),
}

impl<Cmd, Evt, Qry> Message<Cmd, Evt, Qry> {
    pub fn command(cmd: Cmd) -> Self {
        Self::Command(cmd)
    }
    pub fn event(evt: Evt) -> Self {
        Self::Event(evt)
    }
    pub fn query(qry: Qry) -> Self {
        Self::Query(qry)
    }

    pub fn into_command(self) -> Option<Cmd> {
        if let Self::Command(c) = self {
            Some(c)
        } else {
            None
        }
    }
    pub fn into_event(self) -> Option<Evt> {
        if let Self::Event(e) = self {
            Some(e)
        } else {
            None
        }
    }
    pub fn into_query(self) -> Option<Qry> {
        if let Self::Query(q) = self {
            Some(q)
        } else {
            None
        }
    }

    pub fn as_command(&self) -> Option<&Cmd> {
        if let Self::Command(c) = self {
            Some(c)
        } else {
            None
        }
    }
    pub fn as_event(&self) -> Option<&Evt> {
        if let Self::Event(e) = self {
            Some(e)
        } else {
            None
        }
    }
    pub fn as_query(&self) -> Option<&Qry> {
        if let Self::Query(q) = self {
            Some(q)
        } else {
            None
        }
    }

    pub fn as_mut_command(&mut self) -> Option<&mut Cmd> {
        if let Self::Command(c) = self {
            Some(c)
        } else {
            None
        }
    }
    pub fn as_mut_event(&mut self) -> Option<&mut Evt> {
        if let Self::Event(e) = self {
            Some(e)
        } else {
            None
        }
    }
    pub fn as_mut_query(&mut self) -> Option<&mut Qry> {
        if let Self::Query(q) = self {
            Some(q)
        } else {
            None
        }
    }

    pub fn map_command<U, F>(self, f: F) -> Message<U, Evt, Qry>
    where
        F: FnOnce(Cmd) -> U,
    {
        match self {
            Message::Command(c) => Message::Command(f(c)),
            Message::Event(e) => Message::Event(e),
            Message::Query(q) => Message::Query(q),
        }
    }
    pub fn map_event<U, F>(self, f: F) -> Message<Cmd, U, Qry>
    where
        F: FnOnce(Evt) -> U,
    {
        match self {
            Message::Command(c) => Message::Command(c),
            Message::Event(e) => Message::Event(f(e)),
            Message::Query(q) => Message::Query(q),
        }
    }
    pub fn map_query<U, F>(self, f: F) -> Message<Cmd, Evt, U>
    where
        F: FnOnce(Qry) -> U,
    {
        match self {
            Message::Command(c) => Message::Command(c),
            Message::Event(e) => Message::Event(e),
            Message::Query(q) => Message::Query(f(q)),
        }
    }
}

impl<Cmd: Clone, Evt: Clone, Qry: Clone> Clone for Message<Cmd, Evt, Qry> {
    fn clone(&self) -> Self {
        match self {
            Message::Command(c) => Message::Command(c.clone()),
            Message::Event(e) => Message::Event(e.clone()),
            Message::Query(q) => Message::Query(q.clone()),
        }
    }
}

impl<Cmd: PartialEq, Evt: PartialEq, Qry: PartialEq> PartialEq for Message<Cmd, Evt, Qry> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Message::Command(a), Message::Command(b)) => a == b,
            (Message::Event(a), Message::Event(b)) => a == b,
            (Message::Query(a), Message::Query(b)) => a == b,
            _ => false,
        }
    }
}
impl<Cmd: Eq, Evt: Eq, Qry: Eq> Eq for Message<Cmd, Evt, Qry> {}

impl<Cmd: std::hash::Hash, Evt: std::hash::Hash, Qry: std::hash::Hash> std::hash::Hash
    for Message<Cmd, Evt, Qry>
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Message::Command(c) => c.hash(state),
            Message::Event(e) => e.hash(state),
            Message::Query(q) => q.hash(state),
        }
    }
}

impl<Cmd: Default, Evt, Qry> Default for Message<Cmd, Evt, Qry> {
    fn default() -> Self {
        Message::Command(Cmd::default())
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub enum BorrowedMessage<'a, Cmd, Evt, Qry> {
    Command(Cmd, #[serde(skip)] PhantomData<&'a ()>),
    Event(Evt, #[serde(skip)] PhantomData<&'a ()>),
    Query(Qry, #[serde(skip)] PhantomData<&'a ()>),
}

impl<'de, Cmd, Evt, Qry> BorrowedMessage<'de, Cmd, Evt, Qry> {
    pub fn command(cmd: Cmd) -> Self {
        BorrowedMessage::Command(cmd, PhantomData)
    }
    pub fn event(evt: Evt) -> Self {
        BorrowedMessage::Event(evt, PhantomData)
    }
    pub fn query(qry: Qry) -> Self {
        BorrowedMessage::Query(qry, PhantomData)
    }

    pub fn into_command(self) -> Option<Cmd> {
        if let Self::Command(c, ..) = self {
            Some(c)
        } else {
            None
        }
    }
    pub fn into_event(self) -> Option<Evt> {
        if let Self::Event(e, ..) = self {
            Some(e)
        } else {
            None
        }
    }
    pub fn into_query(self) -> Option<Qry> {
        if let Self::Query(q, ..) = self {
            Some(q)
        } else {
            None
        }
    }

    pub fn as_command(&self) -> Option<&Cmd> {
        if let Self::Command(c, ..) = self {
            Some(c)
        } else {
            None
        }
    }
    pub fn as_event(&self) -> Option<&Evt> {
        if let Self::Event(e, ..) = self {
            Some(e)
        } else {
            None
        }
    }
    pub fn as_query(&self) -> Option<&Qry> {
        if let Self::Query(q, ..) = self {
            Some(q)
        } else {
            None
        }
    }

    pub fn as_mut_command(&mut self) -> Option<&mut Cmd> {
        if let Self::Command(c, ..) = self {
            Some(c)
        } else {
            None
        }
    }
    pub fn as_mut_event(&mut self) -> Option<&mut Evt> {
        if let Self::Event(e, ..) = self {
            Some(e)
        } else {
            None
        }
    }
    pub fn as_mut_query(&mut self) -> Option<&mut Qry> {
        if let Self::Query(q, ..) = self {
            Some(q)
        } else {
            None
        }
    }

    pub fn map_command<U, F>(self, f: F) -> BorrowedMessage<'de, U, Evt, Qry>
    where
        F: FnOnce(Cmd) -> U,
    {
        match self {
            BorrowedMessage::Command(c, p) => BorrowedMessage::Command(f(c), p),
            BorrowedMessage::Event(e, p) => BorrowedMessage::Event(e, p),
            BorrowedMessage::Query(q, p) => BorrowedMessage::Query(q, p),
        }
    }
    pub fn map_event<U, F>(self, f: F) -> BorrowedMessage<'de, Cmd, U, Qry>
    where
        F: FnOnce(Evt) -> U,
    {
        match self {
            BorrowedMessage::Command(c, p) => BorrowedMessage::Command(c, p),
            BorrowedMessage::Event(e, p) => BorrowedMessage::Event(f(e), p),
            BorrowedMessage::Query(q, p) => BorrowedMessage::Query(q, p),
        }
    }
    pub fn map_query<U, F>(self, f: F) -> BorrowedMessage<'de, Cmd, Evt, U>
    where
        F: FnOnce(Qry) -> U,
    {
        match self {
            BorrowedMessage::Command(c, p) => BorrowedMessage::Command(c, p),
            BorrowedMessage::Event(e, p) => BorrowedMessage::Event(e, p),
            BorrowedMessage::Query(q, p) => BorrowedMessage::Query(f(q), p),
        }
    }
}

impl<'de, Cmd: Clone, Evt: Clone, Qry: Clone> Clone for BorrowedMessage<'de, Cmd, Evt, Qry> {
    fn clone(&self) -> Self {
        match self {
            BorrowedMessage::Command(c, p) => BorrowedMessage::Command(c.clone(), p.clone()),
            BorrowedMessage::Event(e, p) => BorrowedMessage::Event(e.clone(), p.clone()),
            BorrowedMessage::Query(q, p) => BorrowedMessage::Query(q.clone(), p.clone()),
        }
    }
}

impl<'de, Cmd: PartialEq, Evt: PartialEq, Qry: PartialEq> PartialEq
    for BorrowedMessage<'de, Cmd, Evt, Qry>
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (BorrowedMessage::Command(a, ..), BorrowedMessage::Command(b, ..)) => a == b,
            (BorrowedMessage::Event(a, ..), BorrowedMessage::Event(b, ..)) => a == b,
            (BorrowedMessage::Query(a, ..), BorrowedMessage::Query(b, ..)) => a == b,
            _ => false,
        }
    }
}

impl<'de, Cmd: Eq, Evt: Eq, Qry: Eq> Eq for BorrowedMessage<'de, Cmd, Evt, Qry> {}

impl<'de, Cmd: std::hash::Hash, Evt: std::hash::Hash, Qry: std::hash::Hash> std::hash::Hash
    for BorrowedMessage<'de, Cmd, Evt, Qry>
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            BorrowedMessage::Command(c, ..) => c.hash(state),
            BorrowedMessage::Event(e, ..) => e.hash(state),
            BorrowedMessage::Query(q, ..) => q.hash(state),
        }
    }
}

impl<'de, Cmd: Default, Evt, Qry> Default for BorrowedMessage<'de, Cmd, Evt, Qry> {
    fn default() -> Self {
        BorrowedMessage::command(Cmd::default())
    }
}

impl<'de, Cmd, Evt, Qry> From<BorrowedMessage<'de, Cmd, Evt, Qry>> for Message<Cmd, Evt, Qry>
where
    Cmd: ToOwned<Owned = Cmd>,
    Evt: ToOwned<Owned = Evt>,
    Qry: ToOwned<Owned = Qry>,
{
    fn from(val: BorrowedMessage<'de, Cmd, Evt, Qry>) -> Self {
        match val {
            BorrowedMessage::Command(cmd, ..) => Message::Command(cmd.to_owned()),
            BorrowedMessage::Event(evt, ..) => Message::Event(evt.to_owned()),
            BorrowedMessage::Query(qry, ..) => Message::Query(qry.to_owned()),
        }
    }
}

#[cfg(feature = "serde")]
pub(crate) mod message_serde {
    use std::sync::Arc;

    #[derive(Clone)]
    pub struct DeserializerFn<T>(
        pub(crate)  Arc<
            dyn for<'de> Fn(
                    &mut dyn erased_serde::Deserializer<'de>,
                ) -> Result<T, erased_serde::Error>
                + Send
                + Sync,
        >,
    );

    impl<T> DeserializerFn<T> {
        pub fn new(
            f: impl for<'de> Fn(
                    &mut dyn erased_serde::Deserializer<'de>,
                ) -> Result<T, erased_serde::Error>
                + Send
                + Sync
                + 'static,
        ) -> Self {
            Self(Arc::new(f))
        }

        pub fn call<'de>(
            &self,
            de: &mut dyn erased_serde::Deserializer<'de>,
        ) -> Result<T, erased_serde::Error> {
            self.0(de)
        }
    }

    impl<T> std::ops::Deref for DeserializerFn<T> {
        type Target = dyn for<'de> Fn(&mut dyn erased_serde::Deserializer<'de>) -> Result<T, erased_serde::Error>
            + Send
            + Sync;
        fn deref(&self) -> &Self::Target {
            &*self.0
        }
    }

    pub type MessageDeserializer<T> = DeserializerFn<Box<T>>;

    pub type MessageRegistry<T> = al_structures::collections::registries::CowRegistry<String, T>;
}

macro_rules! define_message_kind {
    ($kind:ident) => {
        ::paste::paste! {
            #[doc = concat!("`", stringify!($kind), "Marker` trait acts as a marker for `", stringify!($kind), "` systems and should be derived for each `", stringify!($kind), "` type")]
            pub trait [<$kind Marker>]: crate::MessageMarker {}

            #[doc = concat!("Object-safe helper methods for `dyn ", stringify!($kind), "`.")]
            pub trait [<$kind Helpers>]: crate::ObjectTraits {
                #[cfg(feature = "serde")]
                fn register(self) -> Result<Option<[<$kind Deserializer>]>, al_structures::collections::registries::RegistryError>
                where
                    Self: for<'de> serde::Deserialize<'de>;

                fn to_msg(self) -> DynMessage
                where
                    Self: Sized;

                fn as_any(&self) -> &dyn std::any::Any;
                fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
                fn type_with_generics(&self) -> String;
                fn [<clone_ $kind:snake>](&self) -> Box<dyn $kind>;
                fn [<partial_eq_ $kind:snake>](&self, other: &dyn $kind) -> bool;
                fn [<hash_ $kind:snake>](&self, state: &mut dyn std::hash::Hasher);
            }

            // ----- Blanket impl -----
            impl<T: [<$kind Marker>] + crate::ObjectTraits> [<$kind Helpers>] for T {
                #[cfg(feature = "serde")]
                fn register(self) -> Result<Option<[<$kind Deserializer>]>, al_structures::collections::registries::RegistryError>
                where
                    Self: for<'de> serde::Deserialize<'de>,
                {
                    [<try_register_ $kind:snake>]::<T>()
                }

                fn to_msg(self) -> DynMessage
                where
                    Self: Sized,
                {
                    DynMessage::$kind(Box::new(self))
                }

                fn as_any(&self) -> &dyn std::any::Any { self }
                fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
                fn type_with_generics(&self) -> String {
                    <T as crate::MessageMarker>::type_with_generics()
                }
                fn [<clone_ $kind:snake>](&self) -> Box<dyn $kind> {
                    Box::new(self.clone())
                }
                fn [<partial_eq_ $kind:snake>](&self, other: &dyn $kind) -> bool {
                    other.as_any().downcast_ref::<T>().map_or(false, |o| self == o)
                }
                fn [<hash_ $kind:snake>](&self, mut state: &mut dyn std::hash::Hasher) {
                    use std::hash::Hash;
                    self.type_with_generics().hash(&mut state);
                    self.hash(&mut state);
                }
            }

            impl<T: [<$kind Marker>] + crate::ObjectTraits> $kind for T {}

            impl dyn $kind {
                #[doc = concat!("Downcast `&dyn ", stringify!($kind), "` to a concrete `&T`.")]
                pub fn downcast_ref<T: $kind>(&self) -> Option<&T> {
                    self.as_any().downcast_ref::<T>()
                }

                #[doc = concat!("Downcast `&mut dyn ", stringify!($kind), "` to a concrete `&mut T`.")]
                pub fn downcast_mut<T: $kind>(&mut self) -> Option<&mut T> {
                    self.as_any_mut().downcast_mut::<T>()
                }

                #[doc = concat!("Downcast `Box<dyn ", stringify!($kind), ">` to a `Box<T>`. Returns `Err(self)` if the type does not match.")]
                pub fn downcast_box<T: $kind>(self: Box<Self>) -> Result<Box<T>, Box<dyn $kind>> {
                    if self.as_any().is::<T>() {
                        let raw = Box::into_raw(self);
                        // SAFETY: type id checked above.
                        Ok(unsafe { Box::from_raw(raw as *mut T) })
                    } else {
                        Err(self)
                    }
                }

                #[doc = concat!("Clone the concrete event out of a `Box<dyn ", stringify!($kind), ">`. Returns `Err(self)` if the type does not match.")]
                pub fn downcast_clone<T: $kind + Clone>(self: Box<Self>) -> Result<T, Box<dyn $kind>> {
                    match self.as_any().downcast_ref::<T>() {
                        Some(t) => Ok(t.clone()),
                        None => Err(self),
                    }
                }
            }

            impl Clone for Box<dyn $kind> {
                fn clone(&self) -> Self {
                    self.[<clone_ $kind:snake>]()
                }
            }
            impl PartialEq for dyn $kind {
                fn eq(&self, other: &Self) -> bool {
                    self.[<partial_eq_ $kind:snake>](other)
                }
            }
            impl std::hash::Hash for dyn $kind {
                fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                    self.[<hash_ $kind:snake>](state)
                }
            }

            #[cfg(feature = "serde")]
            pub use [< $kind:snake _serde >]::*;

            #[cfg(feature = "serde")]
            pub(crate) mod [< $kind:snake _serde >] {
                use super::*;
                use al_structures::collections::registries::{Registry, RegistryWrite};

                pub type [<$kind Deserializer>] = crate::MessageDeserializer<dyn $kind>;

                pub type [<$kind Registry>] =
                    crate::MessageRegistry<[<$kind Deserializer>]>;

                pub static [<$kind:snake:upper _REGISTRY>]: std::sync::LazyLock<[<$kind Registry>]> =
                    std::sync::LazyLock::new([<$kind Registry>]::new);

                pub fn [<try_register_ $kind:snake>]<K: $kind + [<$kind Marker>] + for<'de> serde::Deserialize<'de> + 'static>(
                ) -> Result<Option<[<$kind Deserializer>]>, al_structures::collections::registries::RegistryError> {
                    [<try_register_ $kind:snake _with>]::<K>(&[<$kind:snake:upper _REGISTRY>])
                }

                pub fn [<try_register_ $kind:snake _with>]<K: $kind + [<$kind Marker>] + for<'de> serde::Deserialize<'de> + 'static>(
                    registry: &[<$kind Registry>],
                ) -> Result<Option<[<$kind Deserializer>]>, al_structures::collections::registries::RegistryError> {
                    let deser: [<$kind Deserializer>] = crate::DeserializerFn::new(move |de| {
                        let msg: K = ::erased_serde::deserialize(de)?;
                        Ok(Box::new(msg) as Box<dyn $kind>)
                    });
                    registry.insert(<K as crate::MessageMarker>::type_with_generics(), deser)
                }

                ::paste::paste! {
                    #[macro_export]
                    macro_rules! [<register_ $kind:snake>] {
                        ($msg:ty) => {{
                            if let Err(e) = $crate::[<try_register_ $kind:snake>]::<$msg>() {
                                ::std::panic!(
                                    "Failed to register {} type {}: {}",
                                    stringify!($kind),
                                    stringify!($msg),
                                    e
                                );
                            }
                        }};
                    }

                    #[macro_export]
                    macro_rules! [<register_ $kind:snake _with>] {
                        ($registry:expr, $msg:ty) => {{
                            if let Err(e) = $crate::[<try_register_ $kind:snake _with>]::<$msg>($registry) {
                                ::std::panic!(
                                    "Failed to register {} type {} in registry {}: {}",
                                    stringify!($kind),
                                    stringify!($msg),
                                    stringify!($registry),
                                    e
                                );
                            }
                        }};
                    }
                }

                impl serde::Serialize for dyn $kind {
                    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where
                        S: serde::Serializer,
                    {
                        ::erased_serde::serialize(
                            &(
                                self.type_with_generics(),
                                self as &dyn ::erased_serde::Serialize,
                            ),
                            serializer,
                        )
                    }
                }

                impl<'de> serde::Deserialize<'de> for Box<dyn $kind> {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: serde::Deserializer<'de>,
                    {
                        deserializer.deserialize_tuple(
                            2,
                            $crate::GenericMessageVisitor::<'_, [<$kind Registry>], dyn $kind>::new(
                                &[<$kind:snake:upper _REGISTRY>],
                            ),
                        )
                    }
                }
            }
        }
    };
}

pub(crate) use define_message_kind;
