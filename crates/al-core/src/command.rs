use crate::event::Event;
use std::hash::Hash;

#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "binary", derive(bincode::Encode, bincode::Decode))]
#[derive(Clone, Default, PartialEq, Hash, std::fmt::Debug)]
pub enum Command {
    Event(Box<dyn Event>),
    Restart,
    Stop,
    #[default]
    Pulse,
}

impl Clone for Box<dyn Event> {
    fn clone(&self) -> Self {
        self._clone_event()
    }
}

impl PartialEq for Box<dyn Event> {
    fn eq(&self, other: &Self) -> bool {
        self._partial_equals_event(other.as_any())
    }
}

impl Hash for Box<dyn Event> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self._hash_event(state);
    }
}

#[cfg(feature = "json")]
impl serde::Serialize for Box<dyn Event> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!()
    }
}

#[cfg(feature = "json")]
impl<'a> serde::Deserialize<'a> for Box<dyn Event> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        todo!()
    }
}

#[cfg(feature = "binary")]
impl bincode::Encode for Box<dyn Event> {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        todo!()
    }
}

#[cfg(feature = "binary")]
impl<Context> bincode::Decode<Context> for Box<dyn Event> {
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        todo!()
    }
}

#[cfg(feature = "binary")]
impl<'de, Context> bincode::BorrowDecode<'de, Context> for Box<dyn Event> {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        todo!()
    }
}

impl Command {
    pub fn is_event(&self) -> bool {
        matches!(self, Command::Event(_))
    }

    pub fn downcast_event<T: Event + 'static>(&self) -> Option<&T> {
        match self {
            Command::Event(event) => event.as_ref().as_any().downcast_ref::<T>(),
            _ => None,
        }
    }

    pub fn event_type_name(&self) -> Option<&'static str> {
        match self {
            Command::Event(event) => Some(event.as_ref().type_name()),
            _ => None,
        }
    }
}
