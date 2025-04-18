use core::fmt;
use alloc::vec::Vec;
use alloc::string::String;

pub trait Deserializer<'de>: Sized {
    type Error: fmt::Display + fmt::Debug;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;
}

pub trait Visitor<'de>: Sized {
    type Value;

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: fmt::Display + fmt::Debug;

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: fmt::Display + fmt::Debug;

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: fmt::Display + fmt::Debug;

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: fmt::Display + fmt::Debug;

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: fmt::Display + fmt::Debug;

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: fmt::Display + fmt::Debug;

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: fmt::Display + fmt::Debug;

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: fmt::Display + fmt::Debug;

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: fmt::Display + fmt::Debug;

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: fmt::Display + fmt::Debug;

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: fmt::Display + fmt::Debug;

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: fmt::Display + fmt::Debug;
} 