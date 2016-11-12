use environment::{Environment, LocalEnv};
use io::IoRef;

pub fn register(env: &mut LocalEnv) {
    env.add_command("fontsize",     cmd_fontsize);
    env.add_command("hyphens",      cmd_hyphens);
    env.add_command("load",         cmd_load);
    env.add_command("use",          cmd_use);
}


fn cmd_fontsize(io: IoRef, env: Environment, local: &mut LocalEnv, args: &[String]) -> bool {
    use typeset::RustTypeEngine;
    
    let size = args.get(0).expect("no size given").parse().expect("not a number");
    local.set_default_font(RustTypeEngine::default().scale(size));
    println!("fontsize set to {}", size);
    true
}

fn cmd_hyphens(io: IoRef, env: Environment, local: &mut LocalEnv, args: &[String]) -> bool {
    use hyphenation::Hyphenator;

    if args.len() != 1 {
        println!("expected one argument");
        return false;
    }
    let ref filename = args[0];
    match env.search_file(&filename) {
        None => {
            println!("hyphens file not found: {}", &filename as &str);
            false
        },
        Some(ref path) => {
            let h = Hyphenator::load(&path);
            local.set_hyphenator(h);
            true
        }
    }
}

fn cmd_load(io: IoRef, env: Environment, local: &mut LocalEnv, args: &[String]) -> bool {
    use blocks::Module;
    use std::str;
    
    for arg in args.iter() {
        let data = if arg.contains("://") {
            io.get(arg)
        } else {
            let filename = &format!("{}.yarn", arg);
            match env.search_file(&filename) {
                Some(ref path) => io.get_path(path),
                None => {
                    println!("{} not found", filename);
                    continue
                }
            }
        };
        
        let s = str::from_utf8(&data).expect("invalid file");
        let m = Module::parse(io.clone(), env, s);
        local.add_target(arg, m);
    }
    true
}

/// for each argument:
///  1. first looks whether the name is in the envonmentment
///  2. if not presend, checks the presens of the name.yarn in CWD
///  3. if not present, check presence of file in $LOOM_DATA
///  4. otherwise gives an error
fn cmd_use(_: IoRef, env: Environment, local: &mut LocalEnv, args: &[String]) -> bool {
    for arg in args.iter() {
        if let Some(idx) = arg.rfind('/') {
            let parent = &arg[.. idx];
            let child = &arg[idx+1 ..];
            
            let source = match env.link(local).get_target(parent) {
                Some(s) => s.clone(),
                None => { 
                    println!("use: source not found");
                    continue;
                }
            };
            
            let source_env = match source.env() {
                Some(e) => e,
                None => {
                    println!("use: source has no namespace");
                    continue;
                }
            };
            
            if child == "*" {
                for (name, node) in source_env.targets() {
                    local.add_target(name, node.clone());
                }
            } else {
                local.add_target(
                    child,
                    source_env.get_target(child).expect("not found").clone()
                );
            }
        } else {
            println!("use: no name given");
        }
    }
    println!("use {:?}", args);
    true
}
