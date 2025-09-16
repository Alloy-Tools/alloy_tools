use serde::de::{DeserializeSeed, Visitor};

pub(crate) struct EventVisitor<'a> {
    pub(crate) registry: &'a crate::registry::EventRegistry,
}

impl<'de, 'a> Visitor<'de> for EventVisitor<'a> {
    type Value = Box<dyn crate::Event>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "an event with a sequence containing the event type followed by the event data"
        )
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let type_name = seq
            .next_element_seed(StringSeed)?
            .ok_or_else(|| serde::de::Error::custom("Expected event type name as first element"))?;
        Ok(seq
            .next_element_seed(EventSeed {
                type_name: &type_name,
                registry: self.registry,
            })?
            .ok_or_else(|| serde::de::Error::custom("Expected event data as second element"))?)
    }
}

struct EventSeed<'a> {
    type_name: &'a str,
    registry: &'a crate::registry::EventRegistry,
}

impl<'de, 'a> DeserializeSeed<'de> for EventSeed<'a> {
    type Value = Box<dyn crate::Event>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let deser = self
            .registry
            .get_deserializer(self.type_name)
            .map_err(|e| serde::de::Error::custom(format!("Registry error: {}", e)))?
            .ok_or_else(|| {
                serde::de::Error::custom(format!(
                    "Error getting deserializer for Event type: {}",
                    self.type_name
                ))
            })?;
        let mut erased = <dyn erased_serde::Deserializer>::erase(deserializer);
        deser(&mut erased).map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

//TODO: remove if bitcode doesn't require it. can use .next_element::<String>() directly
struct StringSeed;

impl<'de> DeserializeSeed<'de> for StringSeed {
    type Value = String;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(self)
    }
}

impl<'de> Visitor<'de> for StringSeed {
    type Value = String;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.to_string())
    }
}