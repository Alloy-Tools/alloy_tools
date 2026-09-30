#[macro_export]
macro_rules! impl_erased_deserializer {
    ($wrapper_name:ident, $deserializer_type:ty) => {
        pub struct $wrapper_name<'de> {
            inner: $deserializer_type,
            human_readable: bool,
        }

        impl<'de> $wrapper_name<'de> {
            pub fn new(mut inner: $deserializer_type) -> Self {
                use serde::Deserializer;
                let human_readable = (&mut inner).is_human_readable();
                Self {
                    inner,
                    human_readable,
                }
            }

            pub fn with_human_readable(inner: $deserializer_type, human_readable: bool) -> Self {
                Self {
                    inner,
                    human_readable,
                }
            }
        }

        impl<'de> serde::Deserializer<'de> for $wrapper_name<'de> {
            type Error = <&'de mut $deserializer_type as serde::Deserializer<'de>>::Error;

            fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_any(visitor)
            }
            fn deserialize_bool<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_bool(visitor)
            }
            fn deserialize_i8<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_i8(visitor)
            }
            fn deserialize_i16<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_i16(visitor)
            }
            fn deserialize_i32<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_i32(visitor)
            }
            fn deserialize_i64<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_i64(visitor)
            }
            fn deserialize_i128<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_i128(visitor)
            }
            fn deserialize_u8<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_u8(visitor)
            }
            fn deserialize_u16<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_u16(visitor)
            }
            fn deserialize_u32<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_u32(visitor)
            }
            fn deserialize_u64<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_u64(visitor)
            }
            fn deserialize_u128<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_u128(visitor)
            }
            fn deserialize_f32<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_f32(visitor)
            }
            fn deserialize_f64<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_f64(visitor)
            }
            fn deserialize_char<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_char(visitor)
            }
            fn deserialize_str<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_str(visitor)
            }
            fn deserialize_string<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_string(visitor)
            }
            fn deserialize_bytes<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_bytes(visitor)
            }
            fn deserialize_byte_buf<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_byte_buf(visitor)
            }
            fn deserialize_option<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_option(visitor)
            }
            fn deserialize_unit<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_unit(visitor)
            }
            fn deserialize_unit_struct<V>(
                mut self,
                name: &'static str,
                visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_unit_struct(name, visitor)
            }
            fn deserialize_newtype_struct<V>(
                mut self,
                name: &'static str,
                visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_newtype_struct(name, visitor)
            }
            fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_seq(visitor)
            }
            fn deserialize_tuple<V>(
                mut self,
                len: usize,
                visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_tuple(len, visitor)
            }
            fn deserialize_tuple_struct<V>(
                mut self,
                name: &'static str,
                len: usize,
                visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_tuple_struct(name, len, visitor)
            }
            fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_map(visitor)
            }
            fn deserialize_struct<V>(
                mut self,
                name: &'static str,
                fields: &'static [&'static str],
                visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_struct(name, fields, visitor)
            }
            fn deserialize_enum<V>(
                mut self,
                name: &'static str,
                variants: &'static [&'static str],
                visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_enum(name, variants, visitor)
            }
            fn deserialize_identifier<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_identifier(visitor)
            }
            fn deserialize_ignored_any<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                self.inner.deserialize_ignored_any(visitor)
            }

            fn is_human_readable(&self) -> bool {
                self.human_readable
            }
        }
    };
}
