use environment::{LocalEnv, GraphChain};
use io::{Io, open_read};
use document::NodeP;
use std::boxed::FnBox;
use futures::Future;
use futures::future::{ok, join_all};
use super::{IString, LoomError};

pub fn register(env: &mut LocalEnv) {
 // env.add_command("fontsize",     cmd_fontsize);
    env.add_command("group",        cmd_group);
    env.add_command("hyphens",      cmd_hyphens);
    env.add_command("load",         cmd_load);
    env.add_command("use",          cmd_use);
    env.add_command("symbol",       cmd_symbol);
}

macro_rules! try_msg {
    ($msg:expr ; $arg:expr) => {
        match $expr {
            Ok(r) => r,
            Err(e) => CommandError {
                msg:    $msg,
                cause:  e.into()
            }
        }
    };
}

macro_rules! cmd_args {
    ($args:expr; $($out:ident ,)+) => {
        let mut iter = $args.drain(..);
        $(
        let $out = match iter.next() {
            Some(v) => v,
            None => return Err(LoomError::MissingArg(stringify!(out)))
        };
        )+
    };
}

fn complete<F: FnOnce(&GraphChain, &mut LocalEnv) + 'static>(f: F) -> CommandComplete {
    (box f) as CommandComplete
}

pub type CommandComplete = Box<FnBox(&GraphChain, &mut LocalEnv)>;
pub type CommandResult = Result<Box<Future<Item=CommandComplete, Error=LoomError>>, LoomError>;
pub type Command = fn(&Io, &GraphChain, Vec<IString>) -> CommandResult;

/*
fn cmd_fontsize(_io: &Io, _env: GraphChain, args: Vec<String>)
 -> CommandResult
{
    cmd_args!{args;
        size,
    };
    //let scale: f32 = try!(size.parse().into());
    
    //local.set_default_font(RustTypeEngine::default().scale(scale));
    println!("fontsize set to {}", size);
    box ok(complete(|_| ()))
}
*/
fn cmd_group(io: &Io, _env: &GraphChain, mut args: Vec<IString>)
 -> CommandResult
{
    cmd_args!{args;
        opening,
        name,
        closing,
    };
    let io = io.clone();
    
    Ok(box ok(complete(move |env: &GraphChain, local| {
        let t = local.get_target(&name).or_else(|| env.get_target(&name)).cloned();
        if let Some(n) = t {
            local.add_group(opening, closing, n);
        } else {
            error!(io.log, "group '{}' not found", name);
            //Err(LoomError::MissingItem(name.to_string()))
        }
    })))
}

fn cmd_hyphens(io: &Io, _env: &GraphChain, args: Vec<IString>)
 -> CommandResult
{
    use hyphenation::Hyphenator;

    if args.len() != 1 {
        return Err(LoomError::MissingArg("filename"))
    }
    let f = io.config(|conf| open_read(&conf.data_dir, &args[0]))
    .and_then(|data| {
        Hyphenator::load(data.to_vec())
        .map_err(|e| LoomError::Hyphenator(e))
        .and_then(|h| 
            Ok(complete(|_env: &GraphChain, local: &mut LocalEnv| local.set_hyphenator(h)))
        )
    });
    Ok(box f)
}

fn cmd_load(io: &Io, env: &GraphChain, mut args: Vec<IString>)
 -> CommandResult
{
    use blocks::Module;
    use std::str;
    
    let modules = args.drain(..)
    .map(move |arg| {
        // FIXME
        let io = io.clone();
        let env = env.clone();
        let name = arg.to_string();
        debug!(io.log, "load '{}'", name);
        
        let filename = format!("{}.yarn", name);
        io.config(|conf| open_read(&conf.yarn_dir, &filename))
        .and_then(move |data| {
            let string = String::from_utf8(data.to_vec()).unwrap();
            Module::parse(io, env, string)
        })
        .map(|module| (module, name))
    })
    .collect::<Vec<_>>();
    
    let f = join_all(modules)
    .and_then(|mut modules: Vec<(NodeP, String)>| {
        Ok(complete(move |_env: &GraphChain, local: &mut LocalEnv| {
            for (module, name) in modules.drain(..) {
                local.add_target(name, module);
            }
        }))
    });
    Ok(box f)
}

/// for each argument:
///  1. first looks whether the name is in the envonmentment
///  2. if not presend, checks the presens of the name.yarn in CWD
///  3. if not present, check presence of file in $LOOM_DATA
///  4. otherwise gives an error

fn cmd_use(io: &Io, _env: &GraphChain, args: Vec<IString>)
 -> CommandResult
{
    let io = io.clone();
    let f = move |env: &GraphChain, local: &mut LocalEnv| {
        for arg in args.iter() {
            let mut parts = arg.split('/').peekable();
            let name = parts.next().unwrap();
            let mut current = match local.get_target(name).or_else(|| env.get_target(name)) {
                Some(n) => n.clone(),
                None => {
                    warn!(io.log, "use: name '{}' not found", name);
                    continue;
                }
            };
            
            while let Some(name) = parts.next() {
                if parts.peek().is_some() {
                    // not the end yet
                    let next = {
                        let env = match current.env() {
                            Some(env) => env,
                            None => {
                                warn!(io.log, "use: '{}' has no environment", name);
                                break;
                            }
                        };
                        
                        if let Some(c) = env.get_target(name) {
                            c.clone()
                        } else {
                            warn!(io.log, "use: name '{}' not found", name);
                            break;
                        }
                    };
                    current = next;
                    continue;
                }
                // end
                if name == "*" {
                    // import all
                    if let Some(env) = current.env() {
                        for (t, node) in env.targets() {
                            local.add_target(t.clone(), node.clone());
                        }
                    } else {
                        warn!(io.log, "use: '{}' has no environment", arg);
                    }
                } else {
                    local.add_target(name.to_string(), current.clone());
                }
            }
        }
    };
    Ok(box ok(complete(f)))
}


fn cmd_symbol(_io: &Io, _env: &GraphChain, mut args: Vec<IString>)
 -> CommandResult
{

    cmd_args!{args;
        src,
        dst,
    };
    
    Ok(box ok(complete(move |_env: &GraphChain, local: &mut LocalEnv|
        local.add_symbol(&src, &dst)
    )))
}
