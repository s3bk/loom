use nodes::prelude::*;

pub struct Block {
    // the macro itself
    target: Ref,
    
    env: LocalEnv,
    
    fields: Fields
}

impl Block {
    pub fn from_block(io: &Io, env: &GraphChain, block: parser::Block)
     -> Box<Future<Item=NodeP, Error=LoomError>>
    {
        let io2 = io.clone();
        
        let argument = block.argument;
        let body = block.body;
        let name = block.name.to_string();
        let childs = body.childs;
        
        box init_env(io.clone(), env.clone(), body.commands, body.parameters)
        .and_then(move |env| {
            let args = Ptr::new(
                NodeList::from(&io2,
                    argument.into_iter().map(|n| item_node(&io2, &env, n))
                )
            );
            
            process_body(io2, env, childs)
            .map(|(env, body)| -> NodeP {
                let p = Ptr::new(Block {
                    target:     Ref::new(name).resolve(&env),
                    env:        env.take(),
                    fields:     Fields {
                        args:   Some(args),
                        body:   Some(body)
                    }
                });
                p.into()
            })
        })
    }
}
impl Node for Block {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.env.childs(out);
        self.fields.childs(out);
    }
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        if let Some(ref target) = self.target.get() {
            target.layout(env.link(self), w);
        } else {
            warn!(Log::root(), "unresolved name: {}", self.target.name());
            for s in &["unresolved" as &str, "macro" as &str, self.target.name()] {
                w.word(Atom {
                    left:   Glue::space(),
                    right:  Glue::space(),
                    text:   s
                });
            }
        }
    }
    fn env(&self) -> Option<&LocalEnv> {
        Some(&self.env)
    }
    fn fields(&self) -> Option<&Fields> {
        Some(&self.fields)
    }
}
