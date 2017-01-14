use std::error::Error as StdErr;
use std::fmt;
use std::path::Path;
use std::io;
use std;
use fst;

#[derive(Debug)]
pub enum Error {
    NotFound,
    Denied,
    Io(io::Error),
    Fst(fst::Error),
    Other(Box<StdErr>),
    String(String)
}
impl StdErr for Error {
    fn description(&self) -> &str {
        match self {
            &Error::NotFound => "not found",
            &Error::Denied => "access denied",
            &Error::Io(ref e) => e.description(),
            &Error::Fst(ref e) => e.description(),
            &Error::Other(ref e) => e.description(),
            &Error::String(ref s) => s
        }
    }

    fn cause(&self) -> Option<&StdErr> {
        match self {
            &Error::Io(ref e) => Some(e),
            &Error::Fst(ref e) => Some(e),
            &Error::Other(ref e) => Some(&**e),
            _ => None
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}
impl From<fst::Error> for Error {
    fn from(e: fst::Error) -> Error {
        Error::Fst(e)
    }
}

pub trait Platform {
    fn data_dir(&self) -> Directory;
    fn fetch(&self, url: &str) -> Result<Vec<u8>, Error>;
}

pub trait FileEntry {
    fn read_into(&self, buf: &mut Vec<u8>) -> Result<(), Error>;
    fn read(&self) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::new();
        self.read_into(&mut buf).map(|_| buf)
    }
    fn write(&mut self, buf: &[u8]) -> Result<(), Error>;
    fn path(&self) -> Option<&Path>;
}

pub trait DirectoryEntry {
    fn get(&self, name: &str) -> Result<Entry, Error>;
}

pub type Directory = Box<DirectoryEntry>;
pub type File = Box<FileEntry>;


#[cfg(feature = "platform_default")]
pub mod default;
#[cfg(feature = "platform_default")]
pub use self::default::*;

pub enum Entry {
    File(File),
    Directory(Directory)
}
