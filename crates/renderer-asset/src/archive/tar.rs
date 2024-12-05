use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display, Formatter},
    io::{self, Cursor, Read, Seek},
    path::Path,
};

use tar::{Archive as TarArchive, Entry as TarEntry};

use super::{Archive, Entry};

#[derive(Debug)]
pub enum TarError {
    Tar(io::Error),
    FileTooLarge(u64),
    BadFileName,
}

impl Display for TarError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TarError::Tar(error) => Display::fmt(error, f),
            TarError::FileTooLarge(size) => write!(f, "File size {} is too large", size),
            TarError::BadFileName => write!(f, "Bad file name (not unicode)"),
        }
    }
}

impl Error for TarError {}

impl From<io::Error> for TarError {
    fn from(value: io::Error) -> Self {
        Self::Tar(value)
    }
}

impl<'a, R: Read> Entry<'a> for TarEntry<'a, R> {
    type Error = TarError;

    fn name(&self) -> Result<Cow<'_, str>, Self::Error> {
        let Ok(path) = self.path() else {
            return Err(TarError::BadFileName);
        };
        let Some(path) = path.as_os_str().to_str() else {
            return Err(TarError::BadFileName);
        };
        Ok(Cow::Owned(String::from(path)))
    }

    fn unpack(&mut self) -> Result<Vec<u8>, Self::Error> {
        let file_size = self.header().size()?;
        let file_size: usize = file_size
            .try_into()
            .map_err(|_| TarError::FileTooLarge(file_size))?;
        let buffer = Vec::with_capacity(file_size);
        let mut cursor = Cursor::new(buffer);
        io::copy(self, &mut cursor)?;
        Ok(cursor.into_inner())
    }
}

impl<R: Read + Seek> Archive<R> for TarArchive<R> {
    type Error = TarError;

    type Entry<'a> = TarEntry<'a, R>
    where
        Self: 'a;

    fn new(stream: R) -> Result<Self, Self::Error> {
        Ok(TarArchive::new(stream))
    }

    fn by_path<P: AsRef<Path>>(&mut self, name: P) -> Result<Option<Self::Entry<'_>>, Self::Error> {
        for entry in self.entries_with_seek()? {
            let entry = entry?;
            if entry.path()? == name.as_ref() {
                return Ok(Some(entry));
            }
        }
        Ok(None)
    }
}
