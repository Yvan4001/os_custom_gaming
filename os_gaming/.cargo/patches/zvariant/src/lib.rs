#![no_std]

use core::fmt;
use serde::{de, ser};

#[derive(Debug)]
pub struct Error;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "zvariant error")
    }
}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        Error
    }
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        Error
    }
}

pub type Result<T> = core::result::Result<T, Error>;

pub mod ser {
    use super::*;
    use serde::ser::{self, Serialize};

    pub struct Serializer;

    impl ser::Serializer for Serializer {
        type Ok = ();
        type Error = Error;
        type SerializeSeq = Self;
        type SerializeTuple = Self;
        type SerializeTupleStruct = Self;
        type SerializeTupleVariant = Self;
        type SerializeMap = Self;
        type SerializeStruct = Self;
        type SerializeStructVariant = Self;

        fn serialize_bool(self, _v: bool) -> Result<()> {
            Ok(())
        }

        fn serialize_i8(self, _v: i8) -> Result<()> {
            Ok(())
        }

        fn serialize_i16(self, _v: i16) -> Result<()> {
            Ok(())
        }

        fn serialize_i32(self, _v: i32) -> Result<()> {
            Ok(())
        }

        fn serialize_i64(self, _v: i64) -> Result<()> {
            Ok(())
        }

        fn serialize_u8(self, _v: u8) -> Result<()> {
            Ok(())
        }

        fn serialize_u16(self, _v: u16) -> Result<()> {
            Ok(())
        }

        fn serialize_u32(self, _v: u32) -> Result<()> {
            Ok(())
        }

        fn serialize_u64(self, _v: u64) -> Result<()> {
            Ok(())
        }

        fn serialize_f32(self, _v: f32) -> Result<()> {
            Ok(())
        }

        fn serialize_f64(self, _v: f64) -> Result<()> {
            Ok(())
        }

        fn serialize_char(self, _v: char) -> Result<()> {
            Ok(())
        }

        fn serialize_str(self, _v: &str) -> Result<()> {
            Ok(())
        }

        fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
            Ok(())
        }

        fn serialize_none(self) -> Result<()> {
            Ok(())
        }

        fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<()>
        where
            T: Serialize,
        {
            Ok(())
        }

        fn serialize_unit(self) -> Result<()> {
            Ok(())
        }

        fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
            Ok(())
        }

        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
        ) -> Result<()> {
            Ok(())
        }

        fn serialize_newtype_struct<T: ?Sized>(
            self,
            _name: &'static str,
            _value: &T,
        ) -> Result<()>
        where
            T: Serialize,
        {
            Ok(())
        }

        fn serialize_newtype_variant<T: ?Sized>(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _value: &T,
        ) -> Result<()>
        where
            T: Serialize,
        {
            Ok(())
        }

        fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
            Ok(self)
        }

        fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
            Ok(self)
        }

        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleStruct> {
            Ok(self)
        }

        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleVariant> {
            Ok(self)
        }

        fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
            Ok(self)
        }

        fn serialize_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStruct> {
            Ok(self)
        }

        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant> {
            Ok(self)
        }
    }

    impl ser::SerializeSeq for Serializer {
        type Ok = ();
        type Error = Error;

        fn serialize_element<T: ?Sized>(&mut self, _value: &T) -> Result<()>
        where
            T: Serialize,
        {
            Ok(())
        }

        fn end(self) -> Result<()> {
            Ok(())
        }
    }

    impl ser::SerializeTuple for Serializer {
        type Ok = ();
        type Error = Error;

        fn serialize_element<T: ?Sized>(&mut self, _value: &T) -> Result<()>
        where
            T: Serialize,
        {
            Ok(())
        }

        fn end(self) -> Result<()> {
            Ok(())
        }
    }

    impl ser::SerializeTupleStruct for Serializer {
        type Ok = ();
        type Error = Error;

        fn serialize_field<T: ?Sized>(&mut self, _value: &T) -> Result<()>
        where
            T: Serialize,
        {
            Ok(())
        }

        fn end(self) -> Result<()> {
            Ok(())
        }
    }

    impl ser::SerializeTupleVariant for Serializer {
        type Ok = ();
        type Error = Error;

        fn serialize_field<T: ?Sized>(&mut self, _value: &T) -> Result<()>
        where
            T: Serialize,
        {
            Ok(())
        }

        fn end(self) -> Result<()> {
            Ok(())
        }
    }

    impl ser::SerializeMap for Serializer {
        type Ok = ();
        type Error = Error;

        fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<()>
        where
            T: Serialize,
        {
            Ok(())
        }

        fn serialize_value<T: ?Sized>(&mut self, _value: &T) -> Result<()>
        where
            T: Serialize,
        {
            Ok(())
        }

        fn end(self) -> Result<()> {
            Ok(())
        }
    }

    impl ser::SerializeStruct for Serializer {
        type Ok = ();
        type Error = Error;

        fn serialize_field<T: ?Sized>(
            &mut self,
            _key: &'static str,
            _value: &T,
        ) -> Result<()>
        where
            T: Serialize,
        {
            Ok(())
        }

        fn end(self) -> Result<()> {
            Ok(())
        }
    }

    impl ser::SerializeStructVariant for Serializer {
        type Ok = ();
        type Error = Error;

        fn serialize_field<T: ?Sized>(
            &mut self,
            _key: &'static str,
            _value: &T,
        ) -> Result<()>
        where
            T: Serialize,
        {
            Ok(())
        }

        fn end(self) -> Result<()> {
            Ok(())
        }
    }
}

pub mod de {
    use super::*;
    use serde::de::{self, Deserialize, Deserializer, Visitor};

    pub struct Deserializer;

    impl<'de> Deserializer<'de> for Deserializer {
        type Error = Error;

        fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_string<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_unit_struct<V>(
            self,
            _name: &'static str,
            _visitor: V,
        ) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_newtype_struct<V>(
            self,
            _name: &'static str,
            _visitor: V,
        ) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_tuple_struct<V>(
            self,
            _name: &'static str,
            _len: usize,
            _visitor: V,
        ) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_tuple_variant<V>(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _visitor: V,
        ) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_struct<V>(
            self,
            _name: &'static str,
            _fields: &'static [&'static str],
            _visitor: V,
        ) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_struct_variant<V>(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _fields: &'static [&'static str],
            _visitor: V,
        ) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }

        fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            Err(Error)
        }
    }
} 