use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display, Formatter},
    io::{self, Cursor, Read, Seek},
    path::{Path, PathBuf},
};

use super::{Archive, Entry};

#[derive(Debug)]
pub enum Xp3Error {
    Xp3(xp3::XP3Error),
    BadFileName(PathBuf),
    FileTooLarge(u64),
}

impl Display for Xp3Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Xp3Error::Xp3(error) => match error.kind() {
                xp3::XP3ErrorKind::Io(error) => Display::fmt(error, f),
                xp3::XP3ErrorKind::InvalidFile => write!(f, "File invalid"),
                xp3::XP3ErrorKind::InvalidHeader => write!(f, "File header invalid"),
                xp3::XP3ErrorKind::InvalidFileIndexHeader => write!(f, "File index header invalid"),
                xp3::XP3ErrorKind::InvalidFileIndex => write!(f, "File index invalid"),
                xp3::XP3ErrorKind::InvalidFileIndexFlag => write!(f, "File index flag invalid"),
                xp3::XP3ErrorKind::FileNotFound => write!(f, "File not found"),
            },
            Xp3Error::BadFileName(file_name) => {
                write!(f, "File name {} is not valid Unicode", file_name.display())
            }
            Xp3Error::FileTooLarge(size) => write!(f, "File size {} is too large", size),
        }
    }
}

impl Error for Xp3Error {}

impl From<xp3::XP3Error> for Xp3Error {
    fn from(value: xp3::XP3Error) -> Self {
        Self::Xp3(value)
    }
}

impl From<io::Error> for Xp3Error {
    fn from(value: io::Error) -> Self {
        Self::Xp3(value.into())
    }
}

pub struct Xp3Entry<'a, T: Read + Seek> {
    reader: &'a Xp3Archive<T>,
    index: &'a xp3::index::file::XP3FileIndex,
}

impl<'a, T: Read + Seek> Entry<'a> for Xp3Entry<'a, T> {
    type Error = Xp3Error;

    fn name(&self) -> Result<Cow<'_, str>, Self::Error> {
        Ok(self.index.info().name().as_str().into())
    }

    fn unpack(&mut self) -> Result<Vec<u8>, Self::Error> {
        let file_size = self.index.info().file_size();
        let file_size: usize = file_size
            .try_into()
            .map_err(|_| Xp3Error::FileTooLarge(file_size))?;
        let buffer = Vec::with_capacity(file_size);
        let mut cursor = Cursor::new(buffer);
        self.reader
            .0
            .unpack(self.index.info().name(), &mut cursor)?;
        Ok(cursor.into_inner())
    }
}

pub struct Xp3Archive<T: Read + Seek>(xp3::archive::XP3Archive<T>);

impl<T: Read + Seek> Archive<T> for Xp3Archive<T> {
    type Error = Xp3Error;

    type Entry<'a> = Xp3Entry<'a, T>
    where
        Self: 'a;

    fn new(stream: T) -> Result<Self, Self::Error> {
        xp3::XP3Reader::open_archive(stream)
            .map(|archive| Self(archive))
            .map_err(|err| err.into())
    }

    fn by_path<P: AsRef<Path>>(&mut self, name: P) -> Result<Option<Self::Entry<'_>>, Self::Error> {
        let name = name
            .as_ref()
            .as_os_str()
            .to_str()
            .ok_or_else(|| Xp3Error::BadFileName(name.as_ref().to_path_buf()))?;
        // XP3 library requires &String
        let name = String::from(name);
        let index = self.0.container().index_set().get(&name);
        Ok(index.map(|index| Xp3Entry {
            reader: self,
            index,
        }))
    }
}
