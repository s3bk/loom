use environment::{LocalEnv, GraphChain};
use io::{Io, AioError};
use document::NodeP;
use std::error::Error;
use std::fmt::{self, Display};
use std::boxed::FnBox;
use futures::{Future, BoxFuture};
use futures::future::{ok, err, join_all};
use inlinable_string::InlinableString;
use super::LoomError;
use std;

pub fn register(env: &mut LocalEnv) {
 // env.add_command("fontsize",     cmd_fontsize);
    env.add_command("group",        cmd_group);
    env.add_command("hyphens",      cmd_hyphens);
    env.add_command("load",         cmd_load);
 // env.add_command("use",          cmd_use);
    env.add_command("symbol",       cmd_symbol);
}

macro_rules! try_msg {
    ($msg:expr ; $arg:$expr) => {
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
            None => return box err(LoomError::MissingArg(stringify!(out)))
        };
        )+
    };
}

fn complete<F: FnOnce(&mut LocalEnv) + 'static>(f: F) -> CommandComplete {
    (box f) as CommandComplete
}

pub type CommandComplete = Box<FnBox(&mut LocalEnv)>;
pub type CommandResult = Box<Future<Item=CommandComplete, Error=LoomError>>;
pub type Command = fn(&Io, &GraphChain, Vec<InlinableString>) -> CommandResult;

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
fn cmd_group(_io: &Io, env: &GraphChain, mut args: Vec<InlinableString>)
 -> CommandResult
{
    cmd_args!{args;
        opening,
        name,
        closing,
    };
    
    let n = env.get_target(&name).expect("name not found").clone();
    
    box ok(complete(move |mut local| local.add_group(&opening, &closing, n)))
}

fn cmd_hyphens(_io: &Io, env: &GraphChain, args: Vec<InlinableString>)
 -> CommandResult
{
    use hyphenation::Hyphenator;

    if args.len() != 1 {
        return box err(LoomError::MissingArg("filename"))
    }
    let ref filename = args[0];
    box env.search_file(&filename)
    .map_err(|e| e.into())
    .and_then(|data| {
        Hyphenator::load(data)
        .map_err(|e| e.into())
        .and_then(|h| 
            Ok(complete(|local: &mut LocalEnv| local.set_hyphenator(h)))
        )
    })
}

fn cmd_load(io: &Io, env: &GraphChain, mut args: Vec<InlinableString>)
 -> CommandResult
{
    use blocks::Module;
    use std::str;
    
    let modules = args.drain(..)
    .map(move |arg| {
        // FIXME
        let io = io.clone();
        let env = env.clone();
        let name = arg.rsplitn(2, '/').next().unwrap().to_owned();
        io.fetch(&arg)
        .and_then(move |data| {
            let string = String::from_utf8(data).unwrap();
            Ok(Module::parse(&io, env, string))
        })
        .flatten()
        .map(|module| (module, name))
    })
    .collect::<Vec<_>>();
    
    box join_all(modules)
    .and_then(|mut modules: Vec<(NodeP, String)>| {
        Ok(complete(move |local: &mut LocalEnv| {
            for (module, name) in modules.drain(..) {
                local.add_target(name, module);
            }
        }))
    })
}

/// for each argument:
///  1. first looks whether the name is in the envonmentment
///  2. if not presend, checks the presens of the name.yarn in CWD
///  3. if not present, check presence of file in $LOOM_DATA
///  4. otherwise gives an error
/*
fn cmd_use(_: &Io, env: GraphChain, local: &mut LocalEnv, args: &[String])
 -> CommandResult
{
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
    ok(()).boxed()
}*/

fn cmd_symbol(_io: &Io, _env: &GraphChain, mut args: Vec<InlinableString>)
 -> CommandResult
{

    cmd_args!{args;
        src,
        dst,
    };
    
    box ok(complete(move |local: &mut LocalEnv| local.add_symbol(&src, &dst)))
}
