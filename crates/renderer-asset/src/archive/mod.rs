use std::{
    borrow::Cow,
    error::Error,
    io::{Read, Seek},
    path::Path,
};

#[cfg(feature = "tar")]
pub mod tar;
#[cfg(feature = "xp3")]
pub mod xp3;
#[cfg(feature = "zip")]
pub mod zip;

pub trait Entry<'a> {
    type Error: Error;

    fn name(&self) -> Result<Cow<'_, str>, Self::Error>;
    fn unpack(&mut self) -> Result<Vec<u8>, Self::Error>;
}

pub trait Archive<T>: Sized {
    type Error: Error;
    type Entry<'a>: Entry<'a, Error = Self::Error>
    where
        Self: 'a;

    fn new(stream: T) -> Result<Self, Self::Error>
    where
        T: Read + Seek;

    fn by_path<P: AsRef<Path>>(&mut self, path: P) -> Result<Option<Self::Entry<'_>>, Self::Error>;
}
