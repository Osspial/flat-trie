#![feature(conservative_impl_trait, splice, slice_rotate, range_contains, specialization)]
extern crate odds;
mod raw;

use raw::*;

use std::borrow::{Borrow, BorrowMut};
use std::marker::PhantomData;

use std::fmt::{self, Debug, Formatter};

fn main() {
    let mut tree: LongTree<_, i32> = LongTree(RawTree::new());
    {
        let mut cursor = tree.cursor_mut();
        cursor.child("a").or_insert(None).enter()
            .child("a.a").or_insert(Some(32)).enter()
            .child("a.a.a").or_insert(Some(48)).enter()
                .child("a.a.a.a").or_insert(None).cont().parent().enter()
                .child("a.a.b").or_insert(Some(83)).cont().parent().enter().parent().enter()
            .child("b").or_insert(Some(64)).cont();

        println!("{:#?}", cursor.tree);
        cursor.child("b").unwrap_occupied().prune();
        println!("{:#?}", cursor.tree);
    }

    {
        let mut cursor = tree.cursor();
        println!("{:?}", cursor.find_leaf_after_wrapping(83).unwrap());
    }

    let mut cursor = tree.cursor();
    'traverse: loop {
        let child_opt = cursor.direct_children().next().cloned();
        match child_opt {
            Some(child) => {cursor.child(child).unwrap_occupied().enter();},
            None => {
                while let Entry::Vacant(..) = cursor.sibling(1) {
                    cursor.parent().enter();
                    if cursor.at_root() {
                        break 'traverse;
                    }
                }
                cursor.sibling(1).unwrap_occupied().enter();
            }
        }
        for _ in 0..cursor.depth() {
            print!("    ");
        }
        println!("{:?} {:?}", cursor.node(), cursor.leaf());
    }
}

#[derive(Debug)]
pub struct LongTree<N: Eq, L>(RawTree<N, L>);

#[derive(Clone, Copy)]
pub struct Cursor<N, L, T>
    where N: Eq,
          T: Borrow<LongTree<N, L>>
{
    tree: T,
    raw: RawCursor,
    _marker: PhantomData<(N, L)>
}

#[derive(Debug)]
pub enum Entry<'a, N, O, L, T>
    where N: 'a + Eq,
          L: 'a,
          T: 'a + Borrow<LongTree<N, L>>
{
    Occupied(OccupiedEntry<'a, N, L, T>),
    Vacant(VacantEntry<'a, N, O, L, T>)
}

pub struct OccupiedEntry<'a, N, L, T>
    where N: 'a + Eq,
          L: 'a,
          T: 'a + Borrow<LongTree<N, L>>
{
    cursor: &'a mut Cursor<N, L, T>,
    move_to: RawCursor
}

pub struct VacantEntry<'a, N, O, L, T>
    where N: 'a + Eq,
          L: 'a,
          T: 'a + Borrow<LongTree<N, L>>
{
    cursor: &'a mut Cursor<N, L, T>,
    node: O,
    insert_after: RawCursor
}

#[derive(Debug, Clone, Copy)]
pub enum CursorMove<'a, N: 'a> {
    Child(&'a N),
    Parent
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

    pub fn cursor(&self) -> Cursor<N, L, &Self> {
        Cursor {
            tree: self,
            raw: RawCursor::root(),
            _marker: PhantomData
        }
    }

    pub fn cursor_mut(&mut self) -> Cursor<N, L, &mut Self> {
        Cursor {
            tree: self,
            raw: RawCursor::root(),
            _marker: PhantomData
        }
    }
}

impl<N, L, T> Cursor<N, L, T>
    where N: Eq,
          T: Borrow<LongTree<N, L>>
{
    pub fn at_root(&self) -> bool {
        self.raw == RawCursor::root()
    }

    pub fn depth(&self) -> isize {
        self.raw.depth()
    }

    /// # Panics
    /// Panics if the cursor is at the root node.
    pub fn node(&self) -> &N {
        self.tree.borrow().0.get_node(self.raw).expect("Attempted to take node of root")
    }

    pub fn leaf(&self) -> Option<&L> {
        self.tree.borrow().0.get_leaf(self.raw)
    }

    // pub fn leaf_mut(&self) -> Option<&mut L> {
    //     self.tree.borrow().0.get_leaf_mut(self.raw)
    // }

    pub fn direct_children<'b>(&'b self) -> impl 'b + Iterator<Item=&'b N> {
        let tree = &self.tree.borrow().0;
        tree.node_direct_children(self.raw).map(move |rc| tree.get_node(rc).unwrap())
    }

    pub fn sibling(&mut self, sibling_dist: isize) -> Entry<N, (), L, T> {
        match self.tree.borrow().0.get_sibling(self.raw, sibling_dist) {
            Some(sibling) => Entry::Occupied(OccupiedEntry {
                cursor: self,
                move_to: sibling
            }),
            None => Entry::Vacant(VacantEntry {
                insert_after: self.tree.borrow().0.node_parent(self.raw).expect("Attempted to take sibling of root"),
                node: (),
                cursor: self
            })
        }
    }

    pub fn child<O>(&mut self, node: O) -> Entry<N, O, L, T>
        where N: PartialEq<O>
    {
        let child = {
            let tree = &self.tree.borrow().0;
            tree.node_direct_children(self.raw).find(|rc| *tree.get_node(*rc).unwrap() == node)
        };
        match child {
            Some(child) => Entry::Occupied(OccupiedEntry {
                cursor: self,
                move_to: child
            }),
            None => Entry::Vacant(VacantEntry {
                insert_after: self.raw,
                node: node,
                cursor: self
            })
        }
    }

    // pub fn child_through<I, O>(&mut self, nodes: I) -> Entry<N, O, L, T>
    //     where I: IntoIterator<Item=&'b O>,
    //           N: Borrow<O>,
    //           O: 'b + Eq + ?Sized
    // {
    //     let child = self.tree.borrow().0.node_enter_children(self.raw, nodes);
    //     match child {
    //         Some(child) => Entry::Occupied(OccupiedEntry {
    //             cursor: self,
    //             move_to: child
    //         }),
    //         None => Entry::Vacant(VacantEntry {
    //             insert_after: self.raw,
    //             node: node,
    //             cursor: self
    //         })
    //     }
    // }

    pub fn parent(&mut self) -> OccupiedEntry<N, L, T> {
        let parent = self.tree.borrow().0.node_parent(self.raw).expect("Attempted to take parent of root");
        OccupiedEntry {
            cursor: self,
            move_to: parent
        }
    }

    pub fn find_leaf_after_wrapping<'a, M>(&'a mut self, leaf: M) -> Result<OccupiedEntry<'a, N, L, T>, &mut Self>
        where L: PartialEq<M>
    {
        let cursor_opt = self.tree.borrow().0.find_leaf_after_wrapping(self.raw, leaf);
        match cursor_opt {
            Some(raw) => {
                self.raw = raw;
                Ok(OccupiedEntry {
                    cursor: self,
                    move_to: raw
                })
            },
            None => Err(self)
        }
    }
}

impl<'a, N, L, T> Entry<'a, N, N, L, T>
    where N: Eq,
          T: BorrowMut<LongTree<N, L>>
{
    pub fn or_insert(self, leaf: Option<L>) -> OccupiedEntry<'a, N, L, T> {
        match self {
            Entry::Occupied(occupied) => occupied,
            Entry::Vacant(vacant) => vacant.insert(leaf)
        }
    }
}

impl<'a, N, O, L, T> Entry<'a, N, O, L, T>
    where N: Eq,
          T: Borrow<LongTree<N, L>>
{
    pub fn unwrap_occupied(self) -> OccupiedEntry<'a, N, L, T> {
        match self {
            Entry::Occupied(occupied) => occupied,
            Entry::Vacant(..) => panic!("called `Entry::unwrap_occupied()` on a `Vacant` value")
        }
    }

    pub fn unwrap_vacant(self) -> VacantEntry<'a, N, O, L, T> {
        match self {
            Entry::Vacant(vacant) => vacant,
            Entry::Occupied(..) => panic!("called `Entry::unwrap_vacant()` on an `Occupied` value")
        }
    }
}

impl<'a, N, L, T> OccupiedEntry<'a, N, L, T>
    where N: Eq,
          T: Borrow<LongTree<N, L>>
{
    pub fn cont(self) -> &'a mut Cursor<N, L, T> {
        self.cursor
    }

    pub fn node(&self) -> &N {
        self.cursor.tree.borrow().0.get_node(self.move_to).unwrap()
    }

    pub fn leaf(&self) -> Option<&L> {
        self.cursor.tree.borrow().0.get_leaf(self.move_to)
    }

    pub fn enter(self) -> &'a mut Cursor<N, L, T> {
        self.cursor.raw = self.move_to;
        self.cursor
    }

    // pub fn enter_route(self) -> impl 'a + Iterator<Item=CursorMove<'a, N>> {
    //     let old_raw = self.cursor.raw;
    //     self.cursor.raw = self.move_to;
    //     let tree = &self.cursor.tree.borrow().0;
    //     let (ancestor, elevate_dist) = tree.common_ancestor(old_raw, self.move_to);
    //     iter::repeat(()).map(|_| CursorMove::Parent).take(elevate_dist)
    //         .chain(tree.route_to_descendant(ancestor, self.move_to)
    //         .map(|n| CursorMove::Child(n)))
    // }
}

impl<'a, N, L, T> OccupiedEntry<'a, N, L, T>
    where N: Eq,
          T: BorrowMut<LongTree<N, L>>
{
    pub fn leaf_mut(&mut self) -> Option<&mut L> {
        self.cursor.tree.borrow_mut().0.get_leaf_mut(self.move_to)
    }

    pub fn prune(&mut self) {
        self.cursor.tree.borrow_mut().0.prune_node(self.move_to);
    }
}

impl<'a, N, L, T> VacantEntry<'a, N, N, L, T>
    where N: Eq,
          T: BorrowMut<LongTree<N, L>>
{
    pub fn insert(self, leaf: Option<L>) -> OccupiedEntry<'a, N, L, T> {
        let insert_cursor = self.cursor.tree.borrow_mut().0.insert_nodes_after(self.insert_after, Some(self.node), leaf);
        OccupiedEntry {
            cursor: self.cursor,
            move_to: insert_cursor
        }
    }
}

impl<'a, N, O, L, T> VacantEntry<'a, N, O, L, T>
    where N: Eq + Borrow<O>,
          O: ToOwned<Owned=N>,
          T: BorrowMut<LongTree<N, L>>
{
    pub fn insert_cloned(self, leaf: Option<L>) -> OccupiedEntry<'a, N, L, T> {
        let insert_cursor = self.cursor.tree.borrow_mut().0.insert_nodes_after(self.insert_after, Some(self.node.to_owned() ), leaf);
        OccupiedEntry {
            cursor: self.cursor,
            move_to: insert_cursor
        }
    }
}

impl<'a, N, O, L, T> VacantEntry<'a, N, O, L, T>
    where N: Eq,
          T: BorrowMut<LongTree<N, L>>
{
    pub fn insert_node(self, node: N, leaf: Option<L>) -> OccupiedEntry<'a, N, L, T> {
        let insert_cursor = self.cursor.tree.borrow_mut().0.insert_nodes_after(self.insert_after, Some(node), leaf);
        OccupiedEntry {
            cursor: self.cursor,
            move_to: insert_cursor
        }
    }
}

impl<N: Eq + Debug, L: Debug, T: Borrow<LongTree<N, L>>> Debug for Cursor<N, L, T> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        f.debug_struct("Cursor")
            .field("depth", &self.raw.depth())
            .field("node", &self.tree.borrow().0.get_node(self.raw))
            .field("leaf", &self.tree.borrow().0.get_leaf(self.raw))
            .finish()
    }
}

impl<'a, N: Eq + Debug, L: Debug, T: Borrow<LongTree<N, L>>> Debug for OccupiedEntry<'a, N, L, T> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        f.debug_struct("OccupiedEntry")
            .field("depth", &self.cursor.depth())
            .field("node", &self.cursor.tree.borrow().0.get_node(self.move_to))
            .field("leaf", &self.cursor.tree.borrow().0.get_leaf(self.move_to))
            .finish()
    }
}

impl<'a, N: Eq + Debug, O: Debug, L: Debug, T: Borrow<LongTree<N, L>>> Debug for VacantEntry<'a, N, O, L, T> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        f.debug_struct("VacantEntry")
            .field("depth", &self.cursor.depth())
            .field("insert_after", &self.cursor.tree.borrow().0.get_node(self.insert_after))
            .field("insert_with", &self.node)
            .finish()
    }
}
