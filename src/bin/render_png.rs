extern crate loom;
extern crate futures;
#[macro_use] extern crate wheel;

use std::env;
use std::str;
use std::path::PathBuf;
use std::io::Write;
use loom::io::*;
use loom::config::*;
use loom::layout::*;
use loom::output::Output;
use loom::output::png::*;
use loom::LoomError;
use wheel::prelude::*;
use futures::Future;

fn main() {
    let f = open_dir(".")
    .and_then(|d|
        open(&d, "config.json")
        .map(|config| (d, config))
    )
    .and_then(|(dir, config)| {
        let name: PathBuf = env::args().nth(1).expect("no file specified").into();
        let config = Config::parse(config).map_err(|e| LoomError::ConfigError(e));
        let yarn = open(&dir, name.with_extension("yarn").to_str().unwrap());
        config.join(yarn).map(|(config, yarn)| (config, yarn, name))
    })
    .and_then(|(config, yarn, name)| {
        info!(Log::root(), "got the config");
        let io = IoMachine::new(config.clone()).to_ref();
        io.load_yarn(yarn)
            .join(PngOutput::load(config))
            .map(|(yarn, output)| (yarn, output, name))
    })
    .map(|(yarn, output, name)| {
        let mut w = GenericWriter::new(&output);
        yarn.layout(&mut w);
        
        let layout = ColumnLayout::new(w.finish(), 800., 800.);
        for (i, column) in layout.columns().enumerate() {
            println!("column {}: {:?}", i, column);
            let mut surface = output.surface((900., 900.));
            for (y, line) in column {
                for (x, word) in line {
                    PngOutput::draw_word(&mut surface, (x+50., y+50.), word);
                }
            }
            ::std::fs::File::create(&format!("{}_{:03}.png", name.to_str().unwrap(), i)).unwrap()
            .write_all(&surface.encode()).unwrap();
        }
    });
    
    
    wheel::run(f);
}
