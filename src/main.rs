#![feature(conservative_impl_trait)]
// extern crate odds;
// use odds::vec::VecExt;

use std::cmp::Eq;
use std::ops::Range;

use std::fmt::{self, Debug, Formatter};

fn main() {
    let mut tree = LongTree::new("root");
    let mut cursor = tree.cursor_mut();
    cursor.insert_node("level_one");
    {
        let mut cursor = cursor.enter_node("level_one").unwrap();
        cursor.insert_node("goodbye");
        cursor.insert_node("hello");
        println!("{}", cursor.com.has_children(&cursor.tree));
        for child in cursor.direct_children() {
            println!("    {}", child);
        }
        println!("{}", cursor.com.selected_junc);
    }
    for child in cursor.direct_children() {
        println!("{}", child);
    }
    println!("\n{:?}\n{:?}", cursor.tree.nodes, cursor.tree.juncs);
}

pub struct LongTree<N: Eq + Copy> {
    nodes: Vec<N>,
    /// The junctions in the tree
    juncs: Vec<Junction>,
}

pub struct CursorMut<'a, N: 'a + Eq + Copy> {
    tree: &'a mut LongTree<N>,
    com: CursorCommon
}

#[derive(Clone, Copy)]
struct CursorCommon {
    selected_junc: usize,
    selected_node: usize,
    next_junc: Option<usize>,
    depth: usize
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Junction {
    depth: usize,
    node_junc_index: usize,
    node_resume_index: usize,
    /// Either the distance from this junction to the next junction (if there is a next junction),
    /// or the distance to the end of the tree.
    next_junc_dist: usize
}

impl<N: Eq + Copy> LongTree<N> {
    pub fn new(root: N) -> LongTree<N> {
        LongTree {
            nodes: vec![root],
            juncs: vec![Junction {
                depth: 0,
                node_junc_index: 0,
                node_resume_index: usize::max_value(),
                next_junc_dist: 0
            }],
        }
    }

    pub fn cursor_mut(&mut self) -> CursorMut<N> {
        CursorMut {
            tree: self,
            com: CursorCommon {
                selected_junc: 0,
                selected_node: 0,
                next_junc: None,
                depth: 0
            }
        }
    }
}

impl CursorCommon {
    fn at_next_junction<N: Eq + Copy>(self, tree: &LongTree<N>) -> Option<Range<usize>> {
        match self.next_junc.map(|i| (i, tree.juncs[i])) {
            Some((next_junc_index, next_junc)) if self.selected_node == next_junc.node_junc_index =>
                Some(
                    next_junc_index..tree.juncs[next_junc_index..]
                    .split(|j| j.node_junc_index != next_junc.node_junc_index)
                    .next().unwrap().len()
                ),
            _ => None
        }
    }

    fn has_children<N: Eq + Copy>(self, tree: &LongTree<N>) -> bool {
        self.next_junc.is_some() ||
        tree.juncs.get(self.selected_junc)
            .map(|selected_junc| self.depth < selected_junc.depth + selected_junc.next_junc_dist)
            .unwrap_or(false)
    }

    fn direct_children<'a, N: Eq + Copy>(self, tree: &'a LongTree<N>) -> impl 'a + Iterator<Item=N> {
        tree.nodes.get(self.selected_node + 1).cloned().into_iter().filter(move |_| self.has_children(tree)).chain(
            self.at_next_junction(tree).map(|r| r.start + 1..r.end).unwrap_or(0..0).map(move |i| tree.nodes[tree.juncs[i].node_resume_index])
        )
    }

    fn enter_node<N: Eq + Copy>(self, node: N, tree: &LongTree<N>) -> Option<CursorCommon> {
        match self.at_next_junction(tree) {
            Some(next_junc_range) => {
                tree.juncs[next_junc_range.clone()].iter().enumerate()
                    .filter(|&(_, j)| tree.nodes[j.node_resume_index] == node).next()
                    .map(|(i, j)| (
                        i + next_junc_range.start,
                        *j
                    ))
                    .map(move |(new_cursor_junc_index, new_cursor_junc)| CursorCommon {
                        selected_junc: new_cursor_junc_index,
                        selected_node: new_cursor_junc.node_resume_index,

                        next_junc: tree.juncs[new_cursor_junc_index..].iter().enumerate()
                            .filter(|&(_, j)| new_cursor_junc.node_resume_index <= j.node_junc_index)
                            .filter(|&(_, j)| j.depth - new_cursor_junc.depth == new_cursor_junc.next_junc_dist)
                            .map(|(i, _)| i).next(),

                        depth: self.depth + 1
                    })
            },
            _ => {
                let next_node_index = self.selected_node + 1;
                match tree.nodes.get(next_node_index).map(|node_in_tree| *node_in_tree == node) {
                    Some(true) => Some(CursorCommon {
                        selected_junc: self.selected_junc,
                        selected_node: next_node_index,
                        next_junc: self.next_junc,
                        depth: self.depth + 1
                    }),
                    _ => None
                }
            }
        }
    }
}

impl<'a, N: Eq + Copy> CursorMut<'a, N> {
    pub fn direct_children<'b>(&'b self) -> impl 'b + Iterator<Item=N> {
        self.com.direct_children(self.tree)
    }

    pub fn enter_node(&mut self, node: N) -> Option<CursorMut<N>> {
        self.com.enter_node(node, &self.tree).map(move |com| CursorMut{ tree: self.tree, com } )
    }

    pub fn insert_node(&mut self, node: N) {
        let com = self.com;
        let has_node: bool;
        let at_next_junction = com.at_next_junction(&self.tree);
        match at_next_junction.clone() {
            Some(next_junc_range) => {
                has_node = self.tree.juncs[next_junc_range].iter()
                    .map(|j| self.tree.nodes[j.node_resume_index])
                    .filter(|node_in_tree| *node_in_tree == node)
                    .next().is_some();
            },
            None => {
                has_node = self.tree.nodes.get(com.selected_node + 1).map(|node_in_tree| *node_in_tree == node).unwrap_or(false);
            }
        }

        if has_node {
            panic!("Attempted to insert node when node already exists");
        }
        println!("inserting node {}", com.has_children(&self.tree));
        match com.has_children(&self.tree) {
            true => {
                let selected_junc_dist = com.depth - self.tree.juncs[com.selected_junc].depth;
                let new_junc = Junction {
                    depth: com.depth,
                    node_junc_index: com.selected_node,
                    node_resume_index: self.tree.nodes.len(),
                    next_junc_dist: self.tree.juncs[com.selected_junc].next_junc_dist - selected_junc_dist
                };
                self.tree.nodes.push(node);
                self.tree.juncs[com.selected_junc].next_junc_dist = selected_junc_dist;

                let junc_insert_index =
                    self.tree.juncs[at_next_junction.unwrap_or(com.selected_junc..self.tree.juncs.len())]
                        .binary_search(&new_junc).unwrap_err();
                self.tree.juncs.insert(junc_insert_index, new_junc);
            },
            false => {
                let insert_index = com.selected_node + (0 < self.tree.nodes.len()) as usize;
                self.tree.nodes.insert(insert_index, node);
                for junc in &mut self.tree.juncs[com.selected_junc + 1..] {
                    junc.node_junc_index += 1;
                    junc.node_resume_index += 1;
                }
                self.tree.juncs[com.selected_junc].next_junc_dist += 1;
            }
        }
    }
}

// impl<'a, N: Eq + Copy + Debug> Debug for LongTree<N> {
//     fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {

//     }
// }
