#[macro_use]
extern crate clap;
extern crate loom;

use loom::hyphenation::Hyphenator;
use std::path::Path;

fn main() {
    use clap::App;
    let yaml = load_yaml!("hyphen_import.yaml");
    let matches = App::from_yaml(yaml).get_matches();
    
    let create = matches.is_present("create");
    let map = Path::new(matches.value_of("map").unwrap());
    
    let mut h = if create {
        assert!(map.exists() == false);
        Hyphenator::empty()
    } else {
        Hyphenator::load(map)
    };
    
    if let Some(lists) = matches.values_of("list") {
        for list in lists {
            h.add_hyphenlist(Path::new(list));
        }
    }
    
    if let Some(queries) = matches.values_of("query") {
        for q in queries {
            match h.get(q) {
                Some(hn) => {
                    println!("found {:?}", hn);
                    for h in hn.iter() {
                        let (pre, post) = h.apply(q);
                        println!("\t{}-{}", pre, post);
                    }
                },
                None => {
                    println!("{} not found", q);
                }
            }
        }
    }
    h.save(map);
}
