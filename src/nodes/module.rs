use prelude::*;
use nodes::prelude::*;

pub struct Module {
    env:        LocalEnv,
    body:       NodeListP
}
impl Module {
    #[async]
    pub fn parse(io: Io, env: GraphChain, input: String) -> Result<NodeP, LoomError>
    {
        let body = source::parse(&io, &input)?;
        let env = await!(init_env(io.clone(), env, body.commands, body.parameters))?;
        let (env, childs) = await!(process_body(io, env, body.childs))?;
        Ok(Ptr::new(Module {
            env:    env.take(),
            body:   childs
        }).into())
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
