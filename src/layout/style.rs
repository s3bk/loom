use output::Output;
use std::collections::HashMap;
use std::rc::Rc;
use futures::{Future, future};
use serde_json;
use LoomError;
use io::*;

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
    map:        HashMap<String, Style<O>>
}
impl<O: Output + 'static> Stylist<O> {
    pub fn load(config: Config, output: Rc<O>)
     -> Box<Future<Item=Stylist<O>, Error=LoomError>>
    {
        box open_read(&config.style_dir, "style")
        .and_then(move |data| {
            let mut font_cache: HashMap<String, _> = HashMap::new();
            
            let raw_map: HashMap<String, RawStyle> = serde_json::from_slice(&data).unwrap();
            for value in raw_map.values() {
                font_cache.entry(value.font_name.clone()).or_insert_with(|| {
                    open_read(&config.font_dir, &value.font_name)
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
            
            let style_map: HashMap<String, Style<O>> = raw_map.into_iter()
            .map(|(name, raw)| {
                let style = Style {
                    font_size:  raw.font_size,
                    leading:    raw.leading,
                    font:       output.scale(&fonts[&raw.font_name], raw.font_size)
                };
                (name, style)
            })
            .collect();
            
            Stylist {
                map: style_map
            }
        })
    }
    pub fn default(&self) -> &Style<O> {
        &self.map["*"]
    }
    pub fn get(&self, n: &str) -> &Style<O> {
        &self.map.get(n).unwrap_or_else(||self.default())
    }
}
