use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display, Formatter},
    io::{self, Cursor, Read, Seek},
    path::{Path, PathBuf},
};

use zip::{read::ZipFile, ZipArchive};

use super::{Archive, Entry};

#[derive(Debug)]
pub enum ZipError {
    Zip(zip::result::ZipError),
    BadFileName(PathBuf),
    FileTooLarge(u64),
}

impl Display for ZipError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ZipError::Zip(error) => Display::fmt(error, f),
            ZipError::BadFileName(file_name) => {
                write!(f, "File name {} is not valid Unicode", file_name.display())
            }
            ZipError::FileTooLarge(size) => write!(f, "File size {} is too large", size),
        }
    }
}

impl Error for ZipError {}

impl From<zip::result::ZipError> for ZipError {
    fn from(value: zip::result::ZipError) -> Self {
        Self::Zip(value)
    }
}

impl From<io::Error> for ZipError {
    fn from(value: io::Error) -> Self {
        Self::Zip(zip::result::ZipError::Io(value))
    }
}

impl<'a> Entry<'a> for ZipFile<'a> {
    type Error = ZipError;

    fn name(&self) -> Result<Cow<'_, str>, Self::Error> {
        Ok(self.name().into())
    }

    fn unpack(&mut self) -> Result<Vec<u8>, Self::Error> {
        let file_size = self.size();
        let file_size: usize = file_size
            .try_into()
            .map_err(|_| ZipError::FileTooLarge(file_size))?;
        let buffer = Vec::with_capacity(file_size);
        let mut cursor = Cursor::new(buffer);
        io::copy(self, &mut cursor)?;
        Ok(cursor.into_inner())
    }
}

impl<T: Read + Seek> Archive<T> for ZipArchive<T> {
    type Error = ZipError;

    type Entry<'a> = ZipFile<'a>
    where
        Self: 'a;

    fn new(stream: T) -> Result<Self, Self::Error> {
        Ok(ZipArchive::new(stream)?)
    }

    fn by_path<P: AsRef<Path>>(&mut self, name: P) -> Result<Option<Self::Entry<'_>>, Self::Error> {
        let name = name
            .as_ref()
            .as_os_str()
            .to_str()
            .ok_or_else(|| ZipError::BadFileName(name.as_ref().to_path_buf()))?;
        match self.by_name(name) {
            Ok(entry) => Ok(Some(entry)),
            Err(zip::result::ZipError::FileNotFound) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}
