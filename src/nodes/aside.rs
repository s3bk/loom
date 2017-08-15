use nodes::prelude::*;

pub struct Aside {
    inner: NodeP
}

impl Aside {
    pub fn new(inner: NodeP) -> Aside {
        Aside {
            inner: inner
        }
    }
}

impl Node for Aside {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.inner.childs(out);
    }
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        // TODO: Add counter
        
        w.anchor(&mut |w| self.inner.layout(env.clone(), w));
    }
    fn fields(&self) -> Option<&Fields> {
        None
    }
}
