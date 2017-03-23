use output::Output;
use layout::NodeType;
use std::collections::HashMap;
use std::rc::Rc;
use wheel::prelude::*;
use futures::{Future, future};
use serde_json;

type FileReadError = <File as AsyncRead>::Error;

#[derive(Debug)]
pub struct Style<O: Output> {
    font_size:  f32,
    leading:    f32,
    font:       O::Font
}
impl<O: Output> Style<O> {
    pub fn font(&self) -> &O::Font {
        &self.font
    }
}

use config::Config;

#[derive(Deserialize)]
struct RawStyle {
    font_size:  f32,
    leading:    f32,
    font_name:  String
}

pub struct Stylist<O: Output> {
    map:        HashMap<NodeType, Style<O>>
}
impl<O: Output + 'static> Stylist<O> {
    pub fn load(config: Config, output: Rc<O>)
     -> Box<Future<Item=Stylist<O>, Error=FileReadError>>
    {
        box config.style_dir.get_file("style")
        .and_then(|file| file.read())
        .and_then(move |data| {
            let mut font_cache: HashMap<String, _> = HashMap::new();
            
            let raw_map: HashMap<String, RawStyle> = serde_json::from_slice(&data).unwrap();
            for value in raw_map.values() {
                font_cache.entry(value.font_name.clone()).or_insert_with(|| {
                    config.font_dir
                    .get_file(&value.font_name)
                    .and_then(|file| file.read())
                });
            }
        
            future::join_all(
                font_cache.into_iter()
                .map(|(name, future)| future.map(move |r| (name, r)))
            )
            .map(move |items| (raw_map, items))
        })
        .map(move |(raw_map, items)| {
            // translate files into fonts
            let fonts: HashMap<String, Rc<O::UnscaledFont>> = items.into_iter()
            .map(|(font_name, data)|
                (font_name, Rc::new(output.use_font_data(data)))
            ).collect();
            
            let style_map: HashMap<NodeType, Style<O>> = raw_map.into_iter()
            .map(|(name, raw)| {
                let ntype = match &name as &str {
                    "*" => NodeType::Default,
                    _ => NodeType::Named(name)
                };
                let style = Style {
                    font_size:  raw.font_size,
                    leading:    raw.leading,
                    font:       output.scale(&fonts[&raw.font_name], raw.font_size)
                };
                (ntype, style)
            })
            .collect();
            
            Stylist {
                map: style_map
            }
        })
    }
    pub fn default(&self) -> &Style<O> {
        &self.map[&NodeType::Default]
    }
    pub fn get(&self, n: &NodeType) -> &Style<O> {
        &self.map[n]
    }
}
