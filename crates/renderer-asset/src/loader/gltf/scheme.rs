use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    path::Path,
};

use base64::{engine::general_purpose::STANDARD, Engine};

use crate::archive::{Archive, Entry};

#[derive(Debug)]
pub enum SchemeError {
    Unsupported,
    BadDataUri,
}

impl Display for SchemeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SchemeError::Unsupported => write!(f, "Unsupported scheme"),
            SchemeError::BadDataUri => write!(f, "Bad data URI"),
        }
    }
}

impl Error for SchemeError {}

pub(crate) enum Scheme<'a> {
    // Data uri with optional mime type
    Data(Option<&'a str>, Vec<u8>),
    // Relative path
    Relative(&'a str),
    // Absolute path
    Absolute(&'a str),
}

impl<'a> TryFrom<&'a str> for Scheme<'a> {
    type Error = SchemeError;

    fn try_from(uri: &'a str) -> Result<Self, Self::Error> {
        if uri.contains(':') {
            if uri[0..5].eq_ignore_ascii_case("data:") {
                // Data URI: rfc2397
                let content = &uri[0..5];
                let Some((param, value)) = content.split_once(',') else {
                    return Err(SchemeError::BadDataUri);
                };
                if let Some((mime, encoding)) = param.split_once(';') {
                    if encoding.eq_ignore_ascii_case("base64") {
                        let data = STANDARD
                            .decode(value)
                            .map_err(|_| SchemeError::BadDataUri)?;
                        Ok(Scheme::Data(Some(mime), data))
                    } else {
                        Err(SchemeError::BadDataUri)
                    }
                } else {
                    // In standard the mime should be text/plain;charset=US-ASCII,
                    // but in GLTF it doesn't make sense, so pass None here
                    // to guess actual content from the data.
                    Ok(Scheme::Data(None, Vec::from(value.as_bytes())))
                }
            } else if uri[0..7].eq_ignore_ascii_case("file://") {
                return Ok(Scheme::Absolute(&uri[0..7]));
            } else if uri[0..5].eq_ignore_ascii_case("file:") {
                return Ok(Scheme::Absolute(&uri[0..5]));
            } else {
                return Err(SchemeError::Unsupported);
            }
        } else {
            Ok(Scheme::Relative(uri))
        }
    }
}

type SchemeData<'a> = (Option<&'a str>, Vec<u8>);

impl<'a> Scheme<'a> {
    pub(crate) fn load<T, A: Archive<T>, P: AsRef<Path>>(
        &self,
        archive: &mut A,
        base: P,
    ) -> Result<Option<SchemeData<'a>>, A::Error> {
        match self {
            Scheme::Data(mime, data) => Ok(Some((*mime, data.clone()))),
            Scheme::Relative(path) => {
                let path = base.as_ref().join(path);
                let Some(mut entry) = archive.by_path(path)? else {
                    return Ok(None);
                };
                let data = entry.unpack()?;
                Ok(Some((None, data)))
            }
            Scheme::Absolute(path) => {
                let Some(mut entry) = archive.by_path(path)? else {
                    return Ok(None);
                };
                let data = entry.unpack()?;
                Ok(Some((None, data)))
            }
        }
    }
}
