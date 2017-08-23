#![feature(conservative_impl_trait)]
// extern crate odds;
// use odds::vec::VecExt;
mod raw;

use raw::*;

// use std::fmt::{self, Debug, Formatter};

fn main() {
    let mut tree = LongTree::new("root");
    let mut cursor = tree.cursor_mut();
    cursor.insert_node("level_one");
    cursor.insert_node("level_two");
    {
        let mut cursor = cursor.enter_node("level_one").unwrap();
        cursor.insert_node("goodbye");
        cursor.insert_node("hello");
        for child in cursor.direct_children() {
            println!("    {}", child);
        }
    }
    {
        let mut cursor = cursor.enter_node("level_two").unwrap();
        cursor.insert_node("a");
        cursor.insert_node("b");
        cursor.insert_node("c");

        {
            let mut cursor = cursor.enter_node("a").unwrap();
            cursor.insert_node("furthermore");
            for child in cursor.direct_children() {
                println!("      {}", child);
            }
        }

        for child in cursor.direct_children() {
            println!("     {}", child);
        }
    }
    for child in cursor.direct_children() {
        println!("{}", child);
    }

    println!("{:#?}", cursor.tree);
}

#[derive(Debug)]
pub struct LongTree<N: Eq>(RawTree<N>);

pub struct CursorMut<'a, N: 'a + Eq> {
    tree: &'a mut RawTree<N>,
    raw: RawCursor
}

impl<N: Eq> LongTree<N> {
    pub fn new(root: N) -> LongTree<N> {
        LongTree(RawTree::new(root))
    }

    pub fn cursor_mut(&mut self) -> CursorMut<N> {
        CursorMut {
            tree: &mut self.0,
            raw: RawCursor::root()
        }
    }
}

impl<'a, N: Eq> CursorMut<'a, N> {
    pub fn direct_children<'b>(&'b self) -> impl 'b + Iterator<Item=&'b N> {
        self.tree.node_direct_children(self.raw).map(move |rc| self.tree.get_node(rc))
    }

    pub fn enter_node(&mut self, node: N) -> Option<CursorMut<N>> {
        let child = self.tree.node_direct_children(self.raw).find(|rc| &node == self.tree.get_node(*rc));
        child.map(move |rc| CursorMut{ tree: self.tree, raw: rc })
    }

    pub fn insert_node(&mut self, node: N) {
        self.tree.insert_node_after(self.raw, node);
    }
}

// impl<'a, N: Eq + Copy + Debug> Debug for LongTree<N> {
//     fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {

//     }
// }
