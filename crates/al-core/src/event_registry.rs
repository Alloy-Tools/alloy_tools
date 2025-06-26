use serde::de::{DeserializeSeed, MapAccess, Visitor};
use std::{collections::HashMap, marker::PhantomData};

use crate::Event;

type DeserializeFn = Box<
    dyn for<'de> Fn(
            &'de mut dyn erased_serde::Deserializer<'static>,
        ) -> Result<Box<dyn Event>, Box<dyn std::error::Error>>
        + Send
        + Sync,
>;

#[derive(Default)]
pub struct EventRegistry {
    deserializers: HashMap<&'static str, DeserializeFn>,
}

pub trait EventRegistryTrait: Default {
    fn register_event<E: Event + Default + serde::Deserialize<'static> + 'static>(&mut self);
    fn get_deserializer(&self, key: &str) -> Option<&DeserializeFn>;
    /*fn deserialize(
        &self,
        data: &[u8],
        deserializer_factory: &dyn Fn(
            &[u8],
        ) -> Result<
            Box<dyn erased_serde::Deserializer<'static>>,
            Box<dyn std::error::Error>,
        >,
    ) -> Result<Box<dyn Event>, Box<dyn std::error::Error>>;*/
    fn deserialize<'de, D: serde::Deserializer<'de>>(
        &self,
        deserializer: D,
    ) -> Result<Box<dyn Event>, D::Error>;
}

#[derive(serde::Deserialize)]
struct TaggedEvent {
    #[serde(rename = "type")]
    type_name: String,
}

struct EventVisitor<'a, 'de: 'a>(&'a EventRegistry, PhantomData<&'de ()>);

impl<'a, 'de> Visitor<'de> for EventVisitor<'a, 'de> {
    type Value = Box<dyn Event>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a type tagged event")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut type_name = None;
        let mut event = None;
        while let Some(key) = map.next_key::<&str>()? {
            println!("Key: {key}");
            match key {
                "Event" => event = Some(map.next_value_seed(EventSeed(self.0, self.1))),
                "type" => type_name = Some(map.next_value::<String>()?),
                _ => {
                    println!("Key: {key}");
                    map.next_value::<serde::de::IgnoredAny>();
                }
            }
        }

        todo!()
    }
}

struct EventSeed<'a, 'de: 'a>(&'a EventRegistry, PhantomData<&'de ()>);

impl<'a, 'de> DeserializeSeed<'de> for EventSeed<'a, 'de> {
    type Value = Box<dyn Event>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(EventVisitor(self.0, self.1))
    }
}

impl EventRegistryTrait for EventRegistry {
    fn register_event<E: Event + Default + serde::Deserialize<'static> + 'static>(&mut self) {
        let type_name = E::type_name(&E::default());
        let de_fn: DeserializeFn = Box::new(|de: &mut dyn erased_serde::Deserializer| {
            //let event = E::deserialize(<dyn erased_serde::Deserializer>::erase(de))?;
            let event: E = erased_serde::deserialize(de)?;
            Ok(Box::new(event) as Box<dyn Event>)
        });
        self.deserializers.insert(type_name, de_fn);
    }

    fn get_deserializer(&self, key: &str) -> Option<&DeserializeFn> {
        self.deserializers.get(key)
    }

    fn deserialize<'de, D: serde::Deserializer<'de>>(
        &self,
        deserializer: D,
    ) -> Result<Box<dyn Event>, D::Error> {
        deserializer.deserialize_map(EventVisitor(self, PhantomData))
    }

    /*fn deserialize(
        &self,
        data: &[u8],
        deserializer_factory: &dyn Fn(
            &[u8],
        ) -> Result<
            Box<dyn erased_serde::Deserializer<'static>>,
            Box<dyn std::error::Error>,
        >,
    ) -> Result<Box<dyn Event>, Box<dyn std::error::Error>> {
        let mut deserializer = deserializer_factory(data)?;
        let tag = TaggedEvent::deserialize(&mut *deserializer)?;

        let deserialize_fn = self
            .deserializers
            .get(&tag.type_name.as_str())
            .ok_or_else(|| format!("Unknown event type: {}", tag.type_name))?;

        let mut deserializer = deserializer_factory(data)?;
        deserialize_fn(&mut *deserializer)
    }*/
}
