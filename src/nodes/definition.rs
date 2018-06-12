use nodes::prelude::*;

pub struct Definition {
    name:       String,

    args:       NodeListP,
    
    // body of the macro declaration
    body:       Ptr<NodeList<NodeP>>,
    
    // referencing macro invocations
    references: RefCell<Vec<Weak<Node>>>,
    
    env:        LocalEnv
}
impl Node for Definition {
    fn childs(&self, out: &mut Vec<NodeP>) {
        out.push(self.args.clone().into());
        out.push(self.body.clone().into());
    }
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        w.with(&self.name,
            &mut |w| self.args.layout(env.clone() /* .link(self) */, w),
            &mut |w| self.body.layout(env.clone() /* .link(self) */, w)
        )
    }
    fn add_ref(&self, source: &Rc<Node>) {
        self.references.borrow_mut().push(Rc::downgrade(source));
    }
    fn env(&self) -> Option<&LocalEnv> {
        Some(&self.env)
    }
}

impl Definition {
    pub fn from_param(io: Io, env: GraphChain, p: source::Parameter)
     -> Box<Future<Item=Definition, Error=LoomError>>
    {
        let args = p.args;
        let name = p.name.to_string();
        let body = p.value;
        let childs = body.childs;
        
        box init_env(io.clone(), env, body.commands, body.parameters)
        .and_then(move |env| {
            let arglist = Ptr::new(
                NodeList::from(&io,
                    args.into_iter()
                    .map(|n| item_node(&io, &env, n))
                )
            );
            process_body(io, env, childs)
            .and_then(move |(env, childs)| {
                Ok(Definition {
                    name:       name,
                    args:       arglist,
                    body:       childs,
                    references: RefCell::new(vec![]),
                    env:        env.take()
                })
            })
        })
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}
