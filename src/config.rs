use wheel::Directory;
use serde_json;
use futures::{Future, future};
use wheel::prelude::*;

#[derive(Deserialize)]
struct RawConfig {
    style_dir:  String,
    font_dir:   String,
    data_dir:   String,
    yarn_dir:   String,
}

#[derive(Clone)]
pub struct Config {
    pub style_dir:  Directory,
    pub data_dir:   Directory,
    pub font_dir:   Directory,
    pub yarn_dir:   Directory
}

#[derive(Debug)]
pub enum ParseError {
    IoRead(<File as AsyncRead>::Error),
    Json(serde_json::Error)
}
impl Config {
    pub fn parse(file: File) -> Box<Future<Item=Config, Error=ParseError>> {
        box file.read()
        .map_err(|e| ParseError::IoRead(e))
        .and_then(|data| {
            future::result(serde_json::from_slice(&data))
            .map_err(|e| ParseError::Json(e))
            .and_then(|raw: RawConfig| {
                let styles = Directory::open(&raw.style_dir);
                let fonts = Directory::open(&raw.font_dir);
                let data = Directory::open(&raw.data_dir);
                let yarn = Directory::open(&raw.yarn_dir);
            
                styles.join4(fonts, data, yarn)
                .map_err(|e| ParseError::IoRead(e))
            })
            .map(|(styles, fonts, data, yarn)| Config {
                style_dir:  styles,
                font_dir:   fonts,
                data_dir:   data,
                yarn_dir:   yarn
            })
        })
    }
}
