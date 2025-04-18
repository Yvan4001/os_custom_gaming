use core::fmt;

pub trait Serializer: Sized {
    type Ok;
    type Error: fmt::Display + fmt::Debug;
    type SerializeSeq: SerializeSeq;
    type SerializeTuple: SerializeTuple;
    type SerializeTupleStruct: SerializeTupleStruct;
    type SerializeTupleVariant: SerializeTupleVariant;
    type SerializeMap: SerializeMap;
    type SerializeStruct: SerializeStruct;
    type SerializeStructVariant: SerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error>;
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error>;
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error>;
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error>;
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error>;
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error>;
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error>;
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error>;
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error>;
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error>;
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error>;
}

pub trait SerializeSeq: Sized {
    type Ok;
    type Error: fmt::Display + fmt::Debug;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: super::Serialize;

    fn end(self) -> Result<Self::Ok, Self::Error>;
}

pub trait SerializeTuple: Sized {
    type Ok;
    type Error: fmt::Display + fmt::Debug;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: super::Serialize;

    fn end(self) -> Result<Self::Ok, Self::Error>;
}

pub trait SerializeTupleStruct: Sized {
    type Ok;
    type Error: fmt::Display + fmt::Debug;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: super::Serialize;

    fn end(self) -> Result<Self::Ok, Self::Error>;
}

pub trait SerializeTupleVariant: Sized {
    type Ok;
    type Error: fmt::Display + fmt::Debug;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: super::Serialize;

    fn end(self) -> Result<Self::Ok, Self::Error>;
}

pub trait SerializeMap: Sized {
    type Ok;
    type Error: fmt::Display + fmt::Debug;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: super::Serialize;

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: super::Serialize;

    fn end(self) -> Result<Self::Ok, Self::Error>;
}

pub trait SerializeStruct: Sized {
    type Ok;
    type Error: fmt::Display + fmt::Debug;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: super::Serialize;

    fn end(self) -> Result<Self::Ok, Self::Error>;
}

pub trait SerializeStructVariant: Sized {
    type Ok;
    type Error: fmt::Display + fmt::Debug;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: super::Serialize;

    fn end(self) -> Result<Self::Ok, Self::Error>;
} 