use nodes::prelude::*;

pub struct Module {
    env:        LocalEnv,
    body:       NodeListP
}
impl Module {
    pub fn parse(io: Io, env: GraphChain, input: String)
     -> Box< Future<Item=NodeP, Error=LoomError> >
    {
        use nom::IResult;
        use futures::future::err;
        
        #[cfg(debug_assertions)]
        let input = slug::wrap(&input);
        
        #[cfg(not(debug_assertions))]
        let input = &input;
        
        let body = match parser::block_body(input, 0) {
            IResult::Done(rem, out) => {
                if rem.len() > 0 {
                    let s: &str = rem.into();
                    warn!(io.log, "remaining:\n{}", s);
                }
                debug!(io.log, "{:?}", out);
                out
            },
            _ => {
                return box err(LoomError::Parser);
            }
        };
        
        let childs = body.childs;
        box init_env(io.clone(), env, body.commands, body.parameters)
        .and_then(move |env| {
            process_body(io, env, childs)
            .map(|(env, childs)| -> NodeP {
                Ptr::new(Module {
                    env:    env.take(),
                    body:   childs
                }).into()
            })
        })
    }
}
impl Node for Module {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.env.childs(out);
        self.body.childs(out);
    }
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        self.body.layout(env.link(self), w)
    }
    fn env(&self) -> Option<&LocalEnv> {
        Some(&self.env)
    }
}
