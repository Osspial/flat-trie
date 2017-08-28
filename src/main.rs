#![feature(conservative_impl_trait, splice)]
extern crate odds;
mod raw;

use raw::*;

// use std::fmt::{self, Debug, Formatter};

fn main() {
    let mut tree: LongTree<_, i32> = LongTree(RawTree::example());
    {
        let mut cursor = tree.cursor_mut();
        {
            let mut cursor = cursor.enter_node("b").unwrap();
            cursor.insert_node("b.a", Some(16));
        }
        {
            let mut cursor = cursor.enter_node("c").unwrap();
            cursor.insert_node("c.a", Some(64));
        }
        {
            let mut cursor = cursor.enter_node("a").unwrap();
            let mut cursor = cursor.insert_node("a.0", Some(32));
            cursor.insert_node("a.0.d", None);
            cursor.insert_node("a.0.a", None);
        }
        {
            let mut cursor = cursor.insert_node("e", None);
            cursor.insert_node("e.q", None);
        }
    //     cursor.insert_node("a");
    //     cursor.insert_node("b");
    //     cursor.insert_node("c");
    //     cursor.insert_node("d");
    //     {
    //         let mut cursor = cursor.enter_node("a").unwrap();
    //         cursor.insert_node("a.a");
    //         cursor.insert_node("a.b");
    //         cursor.insert_node("a.c");

    //         {
    //             let mut cursor = cursor.enter_node("a.a").unwrap();
    //             cursor.insert_node("a.a.a");
    //         }
    //     }
    }
    let cursor = tree.cursor();
    for child in cursor.direct_children() {
        if let Some(cursor) = cursor.enter_node(child) {
            println!("{} {:?}", child, cursor.get_leaf());
            for child in cursor.direct_children() {
                if let Some(cursor) = cursor.enter_node(child) {
                    println!("\t{} {:?}", child, cursor.get_leaf());
                    for child in cursor.direct_children() {
                        println!("\t\t{} {:?}", child, cursor.enter_node(child).unwrap().get_leaf());
                    }
                }
            }
        }
    }

    // println!("{:#?}", tree);
}

#[derive(Debug)]
pub struct LongTree<N: Eq, L>(RawTree<N, L>);

pub struct Cursor<'a, N: 'a + Eq, L: 'a> {
    tree: &'a RawTree<N, L>,
    raw: RawCursor
}

pub struct CursorMut<'a, N: 'a + Eq, L: 'a> {
    tree: &'a mut RawTree<N, L>,
    raw: RawCursor
}

impl<N: Eq, L> LongTree<N, L> {
    pub fn new() -> LongTree<N, L> {
        LongTree(RawTree::new())
    }

    pub fn cursor(&self) -> Cursor<N, L> {
        Cursor {
            tree: &self.0,
            raw: RawCursor::root()
        }
    }

    pub fn cursor_mut(&mut self) -> CursorMut<N, L> {
        CursorMut {
            tree: &mut self.0,
            raw: RawCursor::root()
        }
    }
}

impl<'a, N: Eq, L> CursorMut<'a, N, L> {
    pub fn direct_children<'b>(&'b self) -> impl 'b + Iterator<Item=&'b N> {
        self.tree.node_direct_children(self.raw).map(move |rc| self.tree.get_node(rc).unwrap())
    }

    pub fn enter_node(&mut self, node: N) -> Option<CursorMut<N, L>> {
        let child = self.tree.node_direct_children(self.raw).find(|rc| &node == self.tree.get_node(*rc).unwrap());
        child.map(move |rc| CursorMut{ tree: self.tree, raw: rc })
    }

    pub fn get_leaf(&self) -> Option<&L> {
        self.tree.node_leaf(self.raw)
    }

    pub fn insert_node(&mut self, node: N, leaf: Option<L>) -> CursorMut<N, L> {
        let raw = self.tree.insert_node_after(self.raw, node, leaf);
        CursorMut {
            tree: self.tree,
            raw
        }
    }

    pub fn prune(self) {
        self.tree.prune_node(self.raw);
    }
}

impl<'a, N: Eq, L> Cursor<'a, N, L> {
    pub fn direct_children<'b>(&'b self) -> impl 'b + Iterator<Item=&'b N> {
        self.tree.node_direct_children(self.raw).map(move |rc| self.tree.get_node(rc).unwrap())
    }

    pub fn enter_node(&self, node: N) -> Option<Cursor<N, L>> {
        let child = self.tree.node_direct_children(self.raw).find(|rc| &node == self.tree.get_node(*rc).unwrap());
        child.map(move |rc| Cursor{ tree: self.tree, raw: rc })
    }

    pub fn get_leaf(&self) -> Option<&L> {
        self.tree.node_leaf(self.raw)
    }
}

// impl<'a, N: Eq + Copy + Debug> Debug for LongTree<N> {
//     fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {

//     }
// }
