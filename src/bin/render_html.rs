
extern crate loom;
extern crate futures;
#[macro_use] extern crate wheel;
use std::fs::OpenOptions;
use std::env;
use std::str;
use std::path::PathBuf;
use loom::output::*;
use loom::io::*;
use loom::config::*;
use loom::LoomError;
use wheel::prelude::*;
use futures::Future;

fn main() {
    let f = open_dir(".")
    .and_then(|d|
        open(&d, ".config")
        .join(open_read(&d, "html.style"))
        .map(|(config, style)| (d, config, style))
    )
    .and_then(|(dir, config, style)| {
        let name: PathBuf = env::args().nth(1).expect("no file specified").into();
        let config = Config::parse(config).map_err(|e| LoomError::ConfigError(e));
        let yarn = open(&dir, name.with_extension("yarn").to_str().unwrap());
        config.join(yarn).map(|(config, yarn)| (config, yarn, name, style))
    })
    .and_then(|(config, yarn, name, style)| {
        info!(Log::root(), "got the config");
        let io = IoMachine::new(config).to_ref();
        io.load_yarn(yarn).map(|yarn| (yarn, name, style))
    })
    .map(|(yarn, name, style)| {
        let mut output = HtmlOutput::new(str::from_utf8(&style).unwrap());
        use html::*;
        
        let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(name.with_extension("html").to_str().unwrap())
        .expect("could not open/create HTML file for writing");
        
        let mut w = HtmlWriter::new(&mut output, &mut file);
        yarn.layout(&mut w);
        
        w.finish();
    });
    
    
    wheel::run(f);
}
