use nodes::prelude::*;

pub struct List {
    items: NodeList<Ptr<Leaf>>
}
impl List {
    pub fn from(io: &Io, env: &GraphChain, items: Vec<Vec<source::Item>>) -> List {
        List {
            items: NodeList::from(
                io,
                items.into_iter().map(|i| Ptr::new(Leaf::from(io, env, i))
            ))
        }
    }
}
impl Node for List {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.items.childs(out)
    }
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        for item in self.items.iter() {
            w.word(Atom {
                left:   Glue::space(),
                right:  Glue::nbspace(),
                text:   "· "
            });
            item.layout(env.clone(), w);
            w.promote(Glue::hfill());
        }
    }
}
