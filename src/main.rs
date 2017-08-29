#![feature(conservative_impl_trait, splice)]
extern crate odds;
mod raw;

use raw::*;

use std::borrow::Borrow;

fn main() {
    let mut tree: LongTree<_, i32> = LongTree(RawTree::new());
    {
        let mut cursor = tree.cursor_mut();
        cursor.insert_node("a", None);
        cursor.insert_node("b", None);
        cursor.insert_node("c", None);
        cursor.insert_node("d", None);
        {
            cursor.enter_child("a").unwrap();
            cursor.insert_node("a.a", None);
            cursor.insert_node("a.b", None);
            cursor.insert_node("a.c", None);

            {
                cursor.enter_child("a.a").unwrap();
                cursor.insert_node("a.a.a", None);
                cursor.enter_parent().unwrap();
            }
            cursor.enter_parent().unwrap();
        }
        {
            cursor.enter_child("b").unwrap();
            cursor.insert_node("b.a", Some(16));
            cursor.enter_parent().unwrap();
        }
        {
            cursor.enter_child("c").unwrap();
            cursor.insert_node("c.a", Some(64));
            cursor.enter_parent().unwrap();
        }
        {
            cursor.enter_child("a").unwrap();
            cursor.insert_node("a.0", Some(32));
            cursor.insert_node("a.0.d", None);
            cursor.insert_node("a.0.a", None);
            cursor.enter_parent().unwrap();
        }
        {
            cursor.insert_node("e", None);
        }
    }

    println!("{:#?}", tree);
}

#[derive(Debug)]
pub struct LongTree<N: Eq, L>(RawTree<N, L>);

#[derive(Clone, Copy)]
pub struct Cursor<'a, N: 'a + Eq, L: 'a> {
    tree: &'a RawTree<N, L>,
    raw: RawCursor
}

pub struct CursorMut<'a, N: 'a + Eq, L: 'a> {
    tree: &'a mut RawTree<N, L>,
    raw: RawCursor
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindError {
    NodeNotFound
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnterParentError {
    AtRoot
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

macro_rules! impl_cursor_common {
    ($(impl $Cursor:ident;)+) => {$(
        impl<'a, N: Eq, L> $Cursor<'a, N, L> {
            pub fn at_root(&self) -> bool {
                self.raw == RawCursor::root()
            }

            pub fn node(&self) -> &N {
                self.tree.get_node(self.raw).unwrap()
            }

            pub fn leaf(&self) -> Option<&L> {
                self.tree.node_leaf(self.raw)
            }

            pub fn direct_children<'b>(&'b self) -> impl 'b + Iterator<Item=&'b N> {
                self.tree.node_direct_children(self.raw).map(move |rc| self.tree.get_node(rc).unwrap())
            }

            pub fn enter_child(&mut self, node: N) -> Result<(), FindError> {
                let child = self.tree.node_direct_children(self.raw).find(|rc| &node == self.tree.get_node(*rc).unwrap());
                match child {
                    Some(child) => {
                        self.raw = child;
                        Ok(())
                    },
                    None => Err(FindError::NodeNotFound)
                }
            }

            pub fn enter_parent(&mut self) -> Result<(), EnterParentError> {
                match self.tree.node_parent(self.raw) {
                    Some(raw) => {
                        self.raw = raw;
                        Ok(())
                    },
                    None => Err(EnterParentError::AtRoot)
                }
            }

            pub fn find_leaf_after_wrapping<M>(&mut self, leaf: &M) -> Result<(), FindError>
                where M: Eq,
                      L: Borrow<M>
            {
                let cursor_opt = self.tree.find_leaf_after_wrapping(self.raw, leaf);
                match cursor_opt {
                    Some(raw) => {
                        self.raw = raw;
                        Ok(())
                    },
                    None => Err(FindError::NodeNotFound)
                }
            }
        }
    )+}
}

impl_cursor_common!{
    impl Cursor;
    impl CursorMut;
}

impl<'a, N: Eq, L> CursorMut<'a, N, L> {

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

// impl<'a, N: Eq + Copy + Debug> Debug for LongTree<N> {
//     fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {

//     }
// }
