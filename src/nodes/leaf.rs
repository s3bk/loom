use nodes::prelude::*;

pub struct Leaf {
    content: NodeList<NodeP>
}
impl Leaf {
    pub fn from(io: &Io, env: &GraphChain, items: Vec<source::Item>) -> Leaf {
        Leaf {
            content: NodeList::from(io,
                items.into_iter().map(|n| item_node(io, env, n))
            )
        }
    }
    pub fn get(&self, n: usize) -> Option<NodeP> {
        self.content.iter().nth(n).cloned()
    }
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=&'a NodeP> {
        self.content.iter()
    }
}
impl Node for Leaf {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.content.childs(out)
    }
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        if self.content.size() > 0 {
            self.content.layout(env, w);
            w.promote(Glue::Newline { fill: true });
        }
    }
}

