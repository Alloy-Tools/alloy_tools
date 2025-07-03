use serde::de::{DeserializeSeed, Visitor};
use crate::EventRegistry;

pub struct EventWrapper<'a> {
    pub(crate) registry: &'a EventRegistry,
    pub(crate) format: &'static str,
}

impl<'de, 'a> Visitor<'de> for EventWrapper<'a> {
    type Value = Box<dyn crate::Event>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "an event tagged with 'type' followed by 'data'")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        if let Some("Event") = map.next_key::<&str>()? {
            map.next_value_seed(WrapperSeed {
                registry: self.registry,
                format: self.format,
            })
        } else {
            Err(serde::de::Error::custom(
                "Expected 'Event' key at the start of the map",
            ))
        }
    }
}

struct WrapperSeed<'a> {
    registry: &'a EventRegistry,
    format: &'static str,
}

impl<'a, 'de> DeserializeSeed<'de> for WrapperSeed<'a> {
    type Value = Box<dyn crate::Event>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(EventVisitor {
            registry: self.registry,
            format: self.format,
        })
    }
}

struct EventVisitor<'a> {
    registry: &'a EventRegistry,
    format: &'static str,
}

impl<'de, 'a> Visitor<'de> for EventVisitor<'a> {
    type Value = Box<dyn crate::Event>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "an event tagged with 'type' followed by 'data'")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let mut type_name: Option<String> = None;

        while let Some(key) = map.next_key::<&str>()? {
            match key {
                "type" => type_name = Some(map.next_value()?),
                "data" => {
                    let type_name =
                        type_name.ok_or_else(|| serde::de::Error::missing_field("type"))?;
                    return map.next_value_seed(EventSeed {
                        registry: self.registry,
                        format: self.format,
                        type_name: &type_name,
                    });
                }
                _ => {
                    let _: serde::de::IgnoredAny = map.next_value()?;
                }
            }
        }

        Err(serde::de::Error::missing_field("data"))
    }
}

struct EventSeed<'a> {
    registry: &'a EventRegistry,
    format: &'a str,
    type_name: &'a str,
}

impl<'de, 'a> DeserializeSeed<'de> for EventSeed<'a> {
    type Value = Box<dyn crate::Event>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let deser = self
            .registry
            .get_deserializer(self.format, self.type_name).map_err(|e| serde::de::Error::custom(format!("Registry error: {}", e)))?
            .ok_or_else(|| {
                serde::de::Error::custom(format!("Error getting Event type: {}", self.type_name))
            })?;
        let mut erased = <dyn erased_serde::Deserializer>::erase(deserializer);
        deser(&mut erased).map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}
