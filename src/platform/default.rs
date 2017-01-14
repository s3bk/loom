use std::path::{Path, PathBuf};
use std::fs;
use std::env;
use std::io::Write;
use platform::{Platform, Error, FileEntry, DirectoryEntry, Entry};

pub struct DefaultPlatform {}

impl DefaultPlatform {
    pub fn init() -> DefaultPlatform {
        DefaultPlatform {}
    }
}

impl Platform for DefaultPlatform {

    fn data_dir(&self) -> Box<DirectoryEntry> {
        let p: PathBuf = match env::var_os("LOOM_DATA") {
            Some(v) => v.into(),
            None => {
                let p = Path::new("data").into();
                println!("LOOM_DATA not set. Using '{:?}' instead.", p);
                p
            }
        };
        box p as Box<DirectoryEntry>
    }
    fn fetch(&self, url: &str) -> Result<Vec<u8>, Error> {
        let mut data = Vec::new();
        use curl::easy::Easy;
        
        let mut easy = Easy::new();
        easy.url(url).unwrap();
        
        {
            let mut transfer = easy.transfer();
            transfer.write_function(|part| {
                data.extend_from_slice(part);
                Ok(part.len())
            }).unwrap();
            transfer.perform().map_err(|e| Error::Other(box e))?;
        }
        Ok(data)
    }
}

pub struct File {
    path: PathBuf,
}

impl FileEntry for File {
    fn path(&self) -> Option<&Path> {
        Some(&*self.path)
    }

    fn read_into(&self, buf: &mut Vec<u8>) -> Result<(), Error> {
        use std::io::Read;
        let mut f = fs::File::open(&self.path)?;
        f.read_to_end(buf)?;
        Ok(())
    }
    fn read(&self) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::new();
        self.read_into(&mut buf).map(|_| buf)
    }
    fn write(&mut self, buf: &[u8]) -> Result<(), Error> {
        unimplemented!();
    }
}

impl DirectoryEntry for PathBuf {
    fn get(&self, name: &str) -> Result<Entry, Error> {
        let e = self.join(name);
        let t = e.metadata()?.file_type();
        if t.is_file() {
            Ok(Entry::File(box File {
                path: e
            }))
        } else if t.is_dir() {
            Ok(Entry::Directory(box e))
        } else {
            Err(Error::NotFound)
        }
    }
}
