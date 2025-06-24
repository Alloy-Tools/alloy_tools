use crate::{command::Command, markers::EventMarker};
use std::{
    any::Any,
    fmt::Debug,
    hash::{Hash, Hasher},
};

/// Used to wrap any passed hashers to support `Event::_hash_event()`
// ----------------------------------------------------------
struct EventHasher<'a>(&'a mut dyn Hasher);

impl<'a> Hasher for EventHasher<'a> {
    fn finish(&self) -> u64 {
        self.0.finish()
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.write(bytes)
    }
}
// ----------------------------------------------------------

/// Used to mark other code with required traits as valid for `Event` support
// ----------------------------------------------------------
mod sealed {
    use crate::EventMarker;

    #[cfg(not(feature = "json"))]
    pub trait JsonFeature {}

    #[cfg(feature = "json")]
    pub trait JsonFeature: erased_serde::Serialize {
        fn _to_sealed_json(&self) -> Result<String, serde_json::Error>;
        fn _from_sealed_json(json: &'static str) -> Result<Self, erased_serde::Error>
        where
            Self: Sized;
    }

    #[cfg(feature = "json")]
    impl<T: EventMarker + serde::Serialize /*+ serde::Deserialize<'static>*/> JsonFeature for T {
        fn _to_sealed_json(&self) -> Result<String, serde_json::Error> {
            serde_json::to_string::<T>(self)
        }

        fn _from_sealed_json(_json: &'static str) -> Result<T, erased_serde::Error> {
            todo!()
            /*let deserializer = &mut serde_json::Deserializer::from_str(json);
            let mut erased_deserializer = <dyn erased_serde::Deserializer>::erase(deserializer);
            erased_serde::deserialize(&mut erased_deserializer)*/
        }
    }
    #[cfg(not(feature = "json"))]
    impl<T: EventMarker> JsonFeature for T {}

    //#[cfg(not(feature = "binary"))]
    pub trait BinaryFeature {}
    //#[cfg(not(feature = "binary"))]
    impl<T: EventMarker> BinaryFeature for T {}
    /*#[cfg(feature = "binary")]
    pub trait BinaryFeature: erased_serde::Serialize {}
    #[cfg(feature = "binary")]
    impl<T: EventMarker + erased_serde::Serialize + bincode::Encode + bincode::Decode<T>>
        BinaryFeature for T
    {
    }*/
}
// ----------------------------------------------------------

pub trait Event:
    Send + Sync + Debug + Any + sealed::JsonFeature + sealed::BinaryFeature + 'static
{
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn type_name(&self) -> &'static str;
    fn _clone_event(&self) -> Box<dyn Event>;
    fn _partial_equals_event(&self, other: &dyn Any) -> bool;
    fn _hash_event(&self, state: &mut dyn Hasher);
    #[cfg(feature = "json")]
    fn _to_json(&self) -> Result<String, serde_json::Error>;
    #[cfg(feature = "json")]
    fn _from_json(json: &'static str) -> Result<Self, erased_serde::Error>
    where
        Self: Sized;

    fn to_cmd(self) -> Command
    where
        Self: Sized,
    {
        Command::Event(Box::new(self))
    }
}

/*#[cfg(any(feature = "json", feature = "binary"))]
pub fn register_event<T: Event + sealed::JsonFeature + sealed::BinaryFeature + 'static>() {
    let key = std::any::type_name::<T>();
    let deserializer = |de: &mut dyn erased_serde::Deserializer| {
        let value: T = erased_serde::deserialize(de);

    }
}

#[cfg(any(feature = "json", feature = "binary"))]
pub fn deserialize_event(type_name: &str, deserializer: &mut dyn erased_serde::Deserializer) -> Result<Box<dyn Event>, erased_serde::Error> {

}*/

/*#[cfg(any(feature = "json", feature = "binary"))]
impl erased_serde::Serialize for dyn Event {
    fn erased_serialize(&self, serializer: &mut dyn erased_serde::Serializer) -> Result<(), erased_serde::Error> {
        todo!()
    }

    fn do_erased_serialize(&self, serializer: &mut dyn erased_serde::Serializer) -> Result<(), ErrorImpl> {
        todo!()
    }
}*/

// Blanket implementation for all compatible types
impl<
        T: EventMarker
            + 'static
            + Send
            + Sync
            + Debug
            + Any
            + Hash
            + Clone
            + Default
            + PartialEq
            + sealed::JsonFeature
            + sealed::BinaryFeature,
    > Event for T
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_name(&self) -> &'static str {
        self._type_name()
    }

    fn _clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }

    fn _partial_equals_event(&self, other: &dyn Any) -> bool {
        if let Some(other) = other.downcast_ref::<T>() {
            self == other
        } else {
            false
        }
    }

    fn _hash_event(&self, state: &mut dyn Hasher) {
        let mut wrapper = EventHasher(state);
        self.type_name().hash(&mut wrapper);
        self.hash(&mut wrapper);
    }

    #[cfg(feature = "json")]
    fn _to_json(&self) -> Result<String, serde_json::Error> {
        self._to_sealed_json()
    }

    #[cfg(feature = "json")]
    fn _from_json(json: &'static str) -> Result<T, erased_serde::Error> {
        T::_from_sealed_json(json)
    }
}
