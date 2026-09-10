use crate::{
    metadata::{CommandMeta, EventMeta, QueryMeta},
    Command, Event, Query,
};
use al_structures::traits::DynTypeName;
#[cfg(feature = "serde")]
use al_structures::{
    collections::storage::CowStorage,
    serde_utils::serde_registries::{FormatTypeRegistry, SerdeFactory, TypeId, TypeIdRegistry},
};
use std::time::Duration;
#[cfg(feature = "serde")]
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

#[cfg(feature = "serde")]
type CowVec<V> = CowStorage<Vec<V>>;
#[cfg(feature = "serde")]
type CowHashMap<K, V> = CowStorage<HashMap<K, V>>;
#[cfg(feature = "serde")]
pub(crate) type MessageRegistryType<'a> = FormatTypeRegistry<
    DynMessage,
    &'a TypeIdRegistry<CowHashMap<Arc<str>, TypeId>, CowVec<Arc<str>>>,
    CowHashMap<Arc<str>, TypeId>,
    CowVec<Arc<str>>,
    CowHashMap<TypeId, usize>,
    CowVec<SerdeFactory<DynMessage>>,
>;
#[cfg(feature = "serde")]
al_structures::init_registries!(MESSAGE: DynMessage, CowHashMap, CowVec, CowHashMap, CowVec);

#[cfg(feature = "serde")]
impl<Cmd: serde::Serialize, Evt: serde::Serialize, Qry: serde::Serialize> serde::Serialize
    for Message<Cmd, Evt, Qry>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Message::Command(m, _) => m.serialize(serializer),
            Message::Event(m, _) => m.serialize(serializer),
            Message::Query(m, _) => m.serialize(serializer),
        }
    }
}

impl<Cmd: DynTypeName, Evt: DynTypeName, Qry: DynTypeName> DynTypeName for Message<Cmd, Evt, Qry> {
    fn module_path(&self) -> &'static str {
        match self {
            Message::Command(c, _) => c.module_path(),
            Message::Event(e, _) => e.module_path(),
            Message::Query(q, _) => q.module_path(),
        }
    }

    fn type_with_generics(&self) -> String {
        match self {
            Message::Command(c, _) => c.type_with_generics(),
            Message::Event(e, _) => e.type_with_generics(),
            Message::Query(q, _) => q.type_with_generics(),
        }
    }
}

pub type DynMessage = Message<Box<dyn Command>, Box<dyn Event>, Box<dyn Query>>;

#[derive(Debug)]
pub enum Message<Cmd, Evt, Qry> {
    Command(Cmd, CommandMeta),
    Event(Evt, EventMeta),
    Query(Qry, QueryMeta),
}

impl<Cmd, Evt, Qry> Message<Cmd, Evt, Qry> {
    pub fn as_string(&self) -> &str {
        match self {
            Message::Command(..) => "Command",
            Message::Event(..) => "Event",
            Message::Query(..) => "Query",
        }
    }

    pub fn command(cmd: Cmd, correlation_id: Uuid, expected_version: Option<u64>) -> Self {
        Self::Command(cmd, CommandMeta::new(correlation_id, expected_version))
    }
    pub fn event(evt: Evt, correlation_id: Uuid) -> Self {
        Self::Event(evt, EventMeta::new(correlation_id))
    }
    pub fn query(qry: Qry, correlation_id: Uuid, timeout: Option<Duration>) -> Self {
        Self::Query(qry, QueryMeta::new(correlation_id, timeout))
    }

    pub fn into_command(self) -> Option<(Cmd, CommandMeta)> {
        if let Self::Command(c, m) = self {
            Some((c, m))
        } else {
            None
        }
    }
    pub fn into_event(self) -> Option<(Evt, EventMeta)> {
        if let Self::Event(e, m) = self {
            Some((e, m))
        } else {
            None
        }
    }
    pub fn into_query(self) -> Option<(Qry, QueryMeta)> {
        if let Self::Query(q, m) = self {
            Some((q, m))
        } else {
            None
        }
    }

    pub fn as_command(&self) -> Option<(&Cmd, &CommandMeta)> {
        if let Self::Command(c, m) = self {
            Some((c, m))
        } else {
            None
        }
    }
    pub fn as_event(&self) -> Option<(&Evt, &EventMeta)> {
        if let Self::Event(e, m) = self {
            Some((e, m))
        } else {
            None
        }
    }
    pub fn as_query(&self) -> Option<(&Qry, &QueryMeta)> {
        if let Self::Query(q, m) = self {
            Some((q, m))
        } else {
            None
        }
    }

    pub fn as_mut_command(&mut self) -> Option<(&mut Cmd, &mut CommandMeta)> {
        if let Self::Command(c, m) = self {
            Some((c, m))
        } else {
            None
        }
    }
    pub fn as_mut_event(&mut self) -> Option<(&mut Evt, &mut EventMeta)> {
        if let Self::Event(e, m) = self {
            Some((e, m))
        } else {
            None
        }
    }
    pub fn as_mut_query(&mut self) -> Option<(&mut Qry, &mut QueryMeta)> {
        if let Self::Query(q, m) = self {
            Some((q, m))
        } else {
            None
        }
    }

    pub fn map_command<U, F>(self, f: F) -> Message<U, Evt, Qry>
    where
        F: FnOnce(Cmd, &mut CommandMeta) -> U,
    {
        match self {
            Message::Command(c, mut meta) => Message::Command(f(c, &mut meta), meta),
            Message::Event(e, meta) => Message::Event(e, meta),
            Message::Query(q, meta) => Message::Query(q, meta),
        }
    }
    pub fn map_event<U, F>(self, f: F) -> Message<Cmd, U, Qry>
    where
        F: FnOnce(Evt, &mut EventMeta) -> U,
    {
        match self {
            Message::Command(c, meta) => Message::Command(c, meta),
            Message::Event(e, mut meta) => Message::Event(f(e, &mut meta), meta),
            Message::Query(q, meta) => Message::Query(q, meta),
        }
    }
    pub fn map_query<U, F>(self, f: F) -> Message<Cmd, Evt, U>
    where
        F: FnOnce(Qry, &mut QueryMeta) -> U,
    {
        match self {
            Message::Command(c, meta) => Message::Command(c, meta),
            Message::Event(e, meta) => Message::Event(e, meta),
            Message::Query(q, mut meta) => Message::Query(f(q, &mut meta), meta),
        }
    }
}

impl<Cmd: Clone, Evt: Clone, Qry: Clone> Clone for Message<Cmd, Evt, Qry> {
    fn clone(&self) -> Self {
        match self {
            Message::Command(c, meta) => Message::Command(c.clone(), meta.clone()),
            Message::Event(e, meta) => Message::Event(e.clone(), meta.clone()),
            Message::Query(q, meta) => Message::Query(q.clone(), meta.clone()),
        }
    }
}

impl<Cmd: PartialEq, Evt: PartialEq, Qry: PartialEq> PartialEq for Message<Cmd, Evt, Qry> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Message::Command(a, m_a), Message::Command(b, m_b)) => a == b && m_a == m_b,
            (Message::Event(a, m_a), Message::Event(b, m_b)) => a == b && m_a == m_b,
            (Message::Query(a, m_a), Message::Query(b, m_b)) => a == b && m_a == m_b,
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
            Message::Command(c, m) => {
                c.hash(state);
                m.hash(state);
            }
            Message::Event(e, m) => {
                e.hash(state);
                m.hash(state);
            }
            Message::Query(q, m) => {
                q.hash(state);
                m.hash(state);
            }
        }
    }
}

impl<Cmd: Default, Evt, Qry> Default for Message<Cmd, Evt, Qry> {
    fn default() -> Self {
        Message::Command(Cmd::default(), Default::default())
    }
}

#[cfg(feature = "borrow")]
pub use borrow::BorrowedMessage;
#[cfg(feature = "borrow")]
mod borrow {
    use super::{CommandMeta, EventMeta, Message, QueryMeta, Uuid};
    use std::marker::PhantomData;

    #[derive(Debug)]
    pub enum BorrowedMessage<'a, Cmd, Evt, Qry> {
        Command(Cmd, CommandMeta, PhantomData<&'a ()>),
        Event(Evt, EventMeta, PhantomData<&'a ()>),
        Query(Qry, QueryMeta, PhantomData<&'a ()>),
    }

    impl<'de, Cmd, Evt, Qry> BorrowedMessage<'de, Cmd, Evt, Qry> {
        pub fn command(cmd: Cmd, correlation_id: Uuid, expected_version: Option<u64>) -> Self {
            BorrowedMessage::Command(
                cmd,
                CommandMeta::new(correlation_id, expected_version),
                PhantomData,
            )
        }
        pub fn event(evt: Evt, correlation_id: Uuid) -> Self {
            BorrowedMessage::Event(evt, EventMeta::new(correlation_id), PhantomData)
        }
        pub fn query(qry: Qry, correlation_id: Uuid, timeout: Option<std::time::Duration>) -> Self {
            BorrowedMessage::Query(qry, QueryMeta::new(correlation_id, timeout), PhantomData)
        }

        pub fn into_command(self) -> Option<(Cmd, CommandMeta)> {
            if let Self::Command(c, m, _) = self {
                Some((c, m))
            } else {
                None
            }
        }
        pub fn into_event(self) -> Option<(Evt, EventMeta)> {
            if let Self::Event(e, m, _) = self {
                Some((e, m))
            } else {
                None
            }
        }
        pub fn into_query(self) -> Option<(Qry, QueryMeta)> {
            if let Self::Query(q, m, _) = self {
                Some((q, m))
            } else {
                None
            }
        }

        pub fn as_command(&self) -> Option<(&Cmd, &CommandMeta)> {
            if let Self::Command(c, m, _) = self {
                Some((c, m))
            } else {
                None
            }
        }
        pub fn as_event(&self) -> Option<(&Evt, &EventMeta)> {
            if let Self::Event(e, m, _) = self {
                Some((e, m))
            } else {
                None
            }
        }
        pub fn as_query(&self) -> Option<(&Qry, &QueryMeta)> {
            if let Self::Query(q, m, _) = self {
                Some((q, m))
            } else {
                None
            }
        }

        pub fn as_mut_command(&mut self) -> Option<(&mut Cmd, &mut CommandMeta)> {
            if let Self::Command(c, m, _) = self {
                Some((c, m))
            } else {
                None
            }
        }
        pub fn as_mut_event(&mut self) -> Option<(&mut Evt, &mut EventMeta)> {
            if let Self::Event(e, m, _) = self {
                Some((e, m))
            } else {
                None
            }
        }
        pub fn as_mut_query(&mut self) -> Option<(&mut Qry, &mut QueryMeta)> {
            if let Self::Query(q, m, _) = self {
                Some((q, m))
            } else {
                None
            }
        }

        pub fn map_command<U, F>(self, f: F) -> BorrowedMessage<'de, U, Evt, Qry>
        where
            F: FnOnce(Cmd, &mut CommandMeta) -> U,
        {
            match self {
                BorrowedMessage::Command(c, mut m, p) => {
                    BorrowedMessage::Command(f(c, &mut m), m, p)
                }
                BorrowedMessage::Event(e, m, p) => BorrowedMessage::Event(e, m, p),
                BorrowedMessage::Query(q, m, p) => BorrowedMessage::Query(q, m, p),
            }
        }
        pub fn map_event<U, F>(self, f: F) -> BorrowedMessage<'de, Cmd, U, Qry>
        where
            F: FnOnce(Evt, &mut EventMeta) -> U,
        {
            match self {
                BorrowedMessage::Command(c, m, p) => BorrowedMessage::Command(c, m, p),
                BorrowedMessage::Event(e, mut m, p) => BorrowedMessage::Event(f(e, &mut m), m, p),
                BorrowedMessage::Query(q, m, p) => BorrowedMessage::Query(q, m, p),
            }
        }
        pub fn map_query<U, F>(self, f: F) -> BorrowedMessage<'de, Cmd, Evt, U>
        where
            F: FnOnce(Qry, &mut QueryMeta) -> U,
        {
            match self {
                BorrowedMessage::Command(c, m, p) => BorrowedMessage::Command(c, m, p),
                BorrowedMessage::Event(e, m, p) => BorrowedMessage::Event(e, m, p),
                BorrowedMessage::Query(q, mut m, p) => BorrowedMessage::Query(f(q, &mut m), m, p),
            }
        }
    }

    impl<'de, Cmd: Clone, Evt: Clone, Qry: Clone> Clone for BorrowedMessage<'de, Cmd, Evt, Qry> {
        fn clone(&self) -> Self {
            match self {
                BorrowedMessage::Command(c, m, p) => {
                    BorrowedMessage::Command(c.clone(), m.clone(), p.clone())
                }
                BorrowedMessage::Event(e, m, p) => {
                    BorrowedMessage::Event(e.clone(), m.clone(), p.clone())
                }
                BorrowedMessage::Query(q, m, p) => {
                    BorrowedMessage::Query(q.clone(), m.clone(), p.clone())
                }
            }
        }
    }

    impl<'de, Cmd: PartialEq, Evt: PartialEq, Qry: PartialEq> PartialEq
        for BorrowedMessage<'de, Cmd, Evt, Qry>
    {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (BorrowedMessage::Command(a, m_a, _), BorrowedMessage::Command(b, m_b, _)) => {
                    a == b && m_a == m_b
                }
                (BorrowedMessage::Event(a, m_a, _), BorrowedMessage::Event(b, m_b, _)) => {
                    a == b && m_a == m_b
                }
                (BorrowedMessage::Query(a, m_a, _), BorrowedMessage::Query(b, m_b, _)) => {
                    a == b && m_a == m_b
                }
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
                BorrowedMessage::Command(c, m, _) => {
                    c.hash(state);
                    m.hash(state);
                }
                BorrowedMessage::Event(e, m, _) => {
                    e.hash(state);
                    m.hash(state);
                }
                BorrowedMessage::Query(q, m, _) => {
                    q.hash(state);
                    m.hash(state);
                }
            }
        }
    }

    impl<'de, Cmd: Default, Evt, Qry> Default for BorrowedMessage<'de, Cmd, Evt, Qry> {
        fn default() -> Self {
            BorrowedMessage::Command(Cmd::default(), Default::default(), PhantomData)
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
                BorrowedMessage::Command(cmd, m, _) => Message::Command(cmd.to_owned(), m),
                BorrowedMessage::Event(evt, m, _) => Message::Event(evt.to_owned(), m),
                BorrowedMessage::Query(qry, m, _) => Message::Query(qry.to_owned(), m),
            }
        }
    }
}

macro_rules! define_message_kind {
    ($kind:ident) => {
        ::paste::paste! {
            mod sealed {
                pub trait [<$kind Marker>]: crate::MessageRequirements {}
                impl<T: super::[<$kind Marker>]> [<$kind Marker>] for T {}
            }
            #[doc = concat!("`", stringify!($kind), "Marker` trait acts as a marker for `", stringify!($kind), "` systems and should be derived for each `", stringify!($kind), "` type")]
            pub trait [<$kind Marker>]: sealed::[<$kind Marker>] {}

            #[doc = concat!("Object-safe helper methods for `dyn ", stringify!($kind), "`.")]
            pub trait [<$kind Helpers>]: crate::ObjectTraits + al_structures::traits::AsAny {
                #[cfg(feature = "serde")]
                fn register(self) -> Result<al_structures::serde_utils::serde_registries::TypeId, al_structures::collections::storage::utils::HandleError>
                where
                    Self: for<'de> serde::Deserialize<'de>;

                fn to_msg(self) -> DynMessage
                where
                    Self: Sized;

                fn [<clone_ $kind:snake>](&self) -> Box<dyn $kind>;
                fn [<partial_eq_ $kind:snake>](&self, other: &dyn $kind) -> bool;
                fn [<hash_ $kind:snake>](&self, state: &mut dyn std::hash::Hasher);
            }

            // ----- Blanket impl -----
            impl<T: [<$kind Marker>] + crate::ObjectTraits> [<$kind Helpers>] for T {
                #[cfg(feature = "serde")]
                fn register(self) -> Result<al_structures::serde_utils::serde_registries::TypeId, al_structures::collections::storage::utils::HandleError>
                where
                    Self: for<'de> serde::Deserialize<'de>,
                {
                    [<try_register_ $kind:snake>]::<T>()
                }
                fn to_msg(self) -> DynMessage
                where
                    Self: Sized,
                {
                    DynMessage::$kind(Box::new(self), Default::default())
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
                //TODO: Remove these and add the `Downcast` trait
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

                type [<$kind Registry>]<'a> = crate::message::MessageRegistryType<'a>;

                pub fn [<try_register_ $kind:snake>]<K: $kind + [<$kind Marker>] + for<'de> serde::Deserialize<'de> + 'static>(
                ) -> Result<al_structures::serde_utils::serde_registries::TypeId, al_structures::collections::storage::utils::HandleError> {
                    [<try_register_ $kind:snake _with>]::<K>(crate::MESSAGE_TYPE_REGISTRY())
                }

                pub fn [<try_register_ $kind:snake _with>]<K: $kind + [<$kind Marker>] + for<'de> serde::Deserialize<'de> + 'static>(
                    registry: &[<$kind Registry>],
                ) -> Result<al_structures::serde_utils::serde_registries::TypeId, al_structures::collections::storage::utils::HandleError> {
                    let slice_factory = std::sync::Arc::new(move |fmt: &dyn $crate::SerdeFormat, data: &[u8]| {
                        let (meta, offset) = crate::metadata::[<$kind Meta>]::decode_slice(data)?;
                        let mut de = fmt.deserialize_slice(&data[offset..])?;
                        Ok(DynMessage::$kind(Box::new(K::deserialize(&mut *de)?), meta))
                    });
                    let reader_factory = std::sync::Arc::new(
                        move |fmt: &dyn $crate::SerdeFormat, reader: &mut dyn std::io::Read| {
                            let meta = crate::metadata::[<$kind Meta>]::decode_reader(reader)?;
                            let mut de = fmt.deserialize_reader(reader)?;
                            Ok(DynMessage::$kind(Box::new(K::deserialize(&mut *de)?), meta))
                        },
                    );
                    let prelude_factory = std::sync::Arc::new(move |value: &DynMessage, writer: &mut dyn std::io::Write| {
                        let meta = value.[<as_ $kind:snake>]().and_then(|(_, m)| Some(m)).ok_or_else(||
                            al_structures::collections::storage::utils::HandleError::Serialization(
                                format!("Expexcted type {} but found type {}", stringify!($kind), value.as_string())
                            )
                        )?;
                        meta.encode(writer).map_err(Into::into)
                    });
                    registry.register_with::<K>($crate::TypeFactory::new(slice_factory, reader_factory, Some(prelude_factory)))
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
                        erased_serde::serialize(
                            self as &dyn erased_serde::Serialize,
                            serializer,
                        )
                    }
                }
            }
        }
    };
}

pub(crate) use define_message_kind;
