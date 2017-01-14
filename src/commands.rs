use environment::{LocalEnv, GraphChain};
use io::IoRef;
use std::error::Error;
use std::fmt::{self, Display};
use std;
use platform;

pub fn register(env: &mut LocalEnv) {
    env.add_command("fontsize",     cmd_fontsize);
    env.add_command("group",        cmd_group);
    env.add_command("hyphens",      cmd_hyphens);
    env.add_command("load",         cmd_load);
    env.add_command("use",          cmd_use);
    env.add_command("symbol",       cmd_symbol);
}

#[derive(Debug)]
pub enum CommandError {
    Message(&'static str),
    Missing(&'static str),
    Other(Box<Error>)
}
impl Error for CommandError {
    fn description(&self) -> &str {
        match *self {
            CommandError::Message(msg) => msg,
            CommandError::Missing(msg) => msg,
            CommandError::Other(ref e) => e.description()
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            CommandError::Other(ref e) => e.cause(),
            _ => None
        }
    }
}
impl Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CommandError::Message(msg) => write!(f, "{}", msg),
            CommandError::Missing(msg) => write!(f, "missing {}", msg),
            CommandError::Other(ref e) => e.fmt(f)
        }
    }
}
impl From<std::num::ParseFloatError> for CommandError {
    fn from(e: std::num::ParseFloatError) -> CommandError {
        CommandError::Other(box e)
    }
}
impl From<platform::Error> for CommandError {
    fn from(e: platform::Error) -> CommandError {
        CommandError::Other(box e)
    }
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
        let mut iter = $args.iter();
        $(
        let $out = match iter.next() {
            Some(v) => v,
            None => return Err(CommandError::Missing(stringify!(out)))
        };
        )+
    };
}

pub type CommandResult = Result<(), CommandError>;
pub type Command = fn(IoRef, GraphChain, &mut LocalEnv, &[String]) -> CommandResult;

fn cmd_fontsize(_io: IoRef, _env: GraphChain, _local: &mut LocalEnv, args: &[String])
 -> CommandResult
{
    cmd_args!{args;
        size,
    };
    let scale: f32 = try!(size.parse().into());
    
    //local.set_default_font(RustTypeEngine::default().scale(scale));
    println!("fontsize set to {}", size);
    Ok(())
}

fn cmd_group(_io: IoRef, env: GraphChain, local: &mut LocalEnv, args: &[String])
 -> CommandResult
{
    cmd_args!{args;
        opening,
        name,
        closing,
    };
    
    let n = env.link(local).get_target(&name).expect("name not found").clone();
    local.add_group(opening, closing, n);
    
    Ok(())
}

fn cmd_hyphens(_io: IoRef, env: GraphChain, local: &mut LocalEnv, args: &[String])
 -> CommandResult
{
    use hyphenation::Hyphenator;

    if args.len() != 1 {
        return Err(CommandError::Message("expected one argument"))
    }
    let ref filename = args[0];
    match env.search_file(&filename) {
        None => {
            Err(CommandError::Message("file not found").into())
        },
        Some(ref path) => {
            let h = Hyphenator::load(&path)?;
            local.set_hyphenator(h);
            Ok(())
        }
    }
}

fn cmd_load(io: IoRef, env: GraphChain, local: &mut LocalEnv, args: &[String])
 -> CommandResult
{
    use blocks::Module;
    use std::str;
    
    for arg in args.iter() {
        let data: Vec<u8> = if arg.contains("://") {
            match io.platform().fetch(arg) {
                Ok(data) => data,
                Err(e) => {
                    println!("failed to fetch {}: {:?}", arg, e);
                    continue;
                }
            }
        } else {
            let filename = &format!("{}.yarn", arg);
            match env.search_file(&filename) {
                Some(file) => file.read().unwrap(),
                None => {
                    println!("{} not found", filename);
                    continue
                }
            }
        };
        
        let s = String::from_utf8(data).expect("invalid file");
        let m = Module::parse(io.clone(), env, &s);
        local.add_target(arg, m);
    }
    Ok(())
}

/// for each argument:
///  1. first looks whether the name is in the envonmentment
///  2. if not presend, checks the presens of the name.yarn in CWD
///  3. if not present, check presence of file in $LOOM_DATA
///  4. otherwise gives an error
fn cmd_use(_: IoRef, env: GraphChain, local: &mut LocalEnv, args: &[String])
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
    Ok(())
}

fn cmd_symbol(_io: IoRef, _env: GraphChain, local: &mut LocalEnv, args: &[String])
 -> CommandResult
{

    cmd_args!{args;
        src,
        dst,
    };
    local.add_symbol(src, dst);
    Ok(())
}
