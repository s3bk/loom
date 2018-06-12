use nodes::prelude::*;

pub struct Group {
    target:     GroupRef,
    fields:     Fields
}

impl Group {
    pub fn from(io: &Io, env: &GraphChain, g: source::Group) -> Ptr<Group> {
        let content = Ptr::new(NodeList::from(io,
            g.content.into_iter().map(|n| item_node(io, env, n))
        ));
        
        let mut g = Ptr::new(Group {
            target:     GroupRef::new(g.opening, g.closing),
            fields:     Fields {
                args:   None,
                body:   Some(content),
            }
        });
        {
            let mut gp: &mut Group = g.get_mut().unwrap();
            gp.target.resolve(env);
        }
        g
    }
}

impl Node for Group {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.fields.childs(out)
    }
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        if let Some(target) = self.target.get() {
            target.layout(env.with_fields(Some(&self.fields)), w)
        } else {
            let open_close = self.target.key();
            
            w.word(Atom {
                left:   Glue::space(),
                right:  Glue::None,
                text:   &open_close.0
            });
            
            match self.fields.body {
                Some(ref n) => n.layout(env, w),
                None => unreachable!()
            }
            
            w.word(Atom {
                left:   Glue::None,
                right:  Glue::space(),
                text:   &open_close.1
            });
        }
    }
}
