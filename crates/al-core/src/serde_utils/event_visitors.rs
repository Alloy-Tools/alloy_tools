use serde::de::{DeserializeSeed, Visitor};

/// Visitor for deserializing `Box<dyn Event>` from a tuple sequence where the first element is the event type name and the second element is the event data.
pub(crate) struct EventVisitor<'a> {
    pub(crate) registry: &'a crate::serde_utils::event_registry::EventRegistry,
}

impl<'de, 'a> Visitor<'de> for EventVisitor<'a> {
    type Value = Box<dyn crate::Event>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "an event with a sequence containing the event type followed by the event data"
        )
    }

    /// Visit a sequence and deserialize it into a `Box<dyn Event>` using the event type name and the event registry
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        // Get the type name from the first element of the sequence
        let type_name = seq
            .next_element::<String>()?
            .ok_or_else(|| serde::de::Error::custom("Expected event type name as first element"))?;
        // Pass the type name and the registry to the EventSeed to deserialize the event data
        Ok(seq
            .next_element_seed(EventSeed {
                type_name: &type_name,
                registry: self.registry,
            })?
            .ok_or_else(|| serde::de::Error::custom("Expected event data as second element"))?)
    }
}

/// Seed for deserializing a specific event type using its type name and the event registry
struct EventSeed<'a> {
    type_name: &'a str,
    registry: &'a crate::serde_utils::event_registry::EventRegistry,
}

impl<'de, 'a> DeserializeSeed<'de> for EventSeed<'a> {
    type Value = Box<dyn crate::Event>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        // Get the deserializer for the given type name from the registry
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
        // Erase the deserializer and pass it to the registry deserializser function
        deser(&mut <dyn erased_serde::Deserializer>::erase(deserializer))
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}
