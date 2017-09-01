use std::cmp::Eq;
use std::borrow::Borrow;
use std::ops::Range;
use std::iter::ExactSizeIterator;
use odds::vec::VecExt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTree<N: Eq, L> {
    nodes: Vec<N>,
    /// The jumptions in the tree
    jumps: Vec<Jump>,
    leaves: Vec<L>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawCursor {
    node_index: isize,
    parent_jump_index: usize
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MajorNode {
    Leaf {
        leaf_index: isize
    },
    Jump {
        child_jump_index: usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Jump {
    parent_jump_index: isize,
    jump_to_node: isize,
    /// The distance from the node at `jump_to_node` to to next node that's eithe a leaf or a
    /// split
    next_major_node_dist: usize,
    next_major_node: MajorNode
}

impl<N: Eq, L> RawTree<N, L> {
    pub fn new() -> RawTree<N, L> {
        RawTree {
            nodes: vec![],
            jumps: vec![Jump::default_root()],
            leaves: vec![]
        }
    }

    pub fn get_node(&self, cursor: RawCursor) -> Option<&N> {
        self.nodes.get(cursor.node_index as usize)
    }

    pub fn node_parent(&self, cursor: RawCursor) -> Option<RawCursor> {
        let parent_jump = self.jumps[cursor.parent_jump_index];
        match cursor.node_index == parent_jump.jump_to_node {
            true => {
                self.jumps.get(parent_jump.parent_jump_index as usize)
                    .map(|j| RawCursor {
                        node_index: j.jump_to_node + j.next_major_node_dist as isize,
                        parent_jump_index: parent_jump.parent_jump_index as usize
                    })
            }
            false => Some(RawCursor {
                node_index: cursor.node_index - 1,
                ..cursor
            })
        }
    }

    pub fn node_direct_children<'a>(&'a self, cursor: RawCursor) -> impl 'a + Iterator<Item=RawCursor> {
        use std::ops::RangeFrom;
        let parent_jump = self.jumps[cursor.parent_jump_index];

        let direct_child: Option<RawCursor>;
        let child_jump_search: RangeFrom<usize>;
        match (parent_jump.next_major_node, parent_jump.cursor_at_next_major_node(cursor)) {
            (_, false) => {
                direct_child = Some(RawCursor {
                    node_index: cursor.node_index + 1,
                    parent_jump_index: cursor.parent_jump_index
                }).into_iter().filter(|_| parent_jump.cursor_has_children(cursor)).next();
                child_jump_search = self.jumps.len()..;
            }
            (MajorNode::Leaf{..}, true) => {
                direct_child = None;
                child_jump_search = self.jumps.len()..;
            }
            (MajorNode::Jump{child_jump_index}, true) => {
                direct_child = None;
                child_jump_search = child_jump_index..;
            }
        }


        let jump_child_iter = self.jumps[child_jump_search.clone()].iter().cloned()
            .zip(child_jump_search)
            .take_while(move |&(s, _)| s.parent_jump_index == cursor.parent_jump_index as isize)
            .map(|(s, i)| RawCursor {
                node_index: s.jump_to_node,
                parent_jump_index: i
            });
        direct_child.into_iter().chain(jump_child_iter)
    }

    pub fn get_sibling(&self, cursor: RawCursor, dist: isize) -> Option<RawCursor> {
        let parent_jump = self.jumps[cursor.parent_jump_index];
        if parent_jump.jump_to_node == cursor.node_index {
            let sibling_jump_index = (cursor.parent_jump_index as isize + dist) as usize;
            match self.jumps.get(sibling_jump_index) {
                Some(potential_sibling_jump) if potential_sibling_jump.parent_jump_index == parent_jump.parent_jump_index =>
                    Some(RawCursor {
                        node_index: potential_sibling_jump.jump_to_node,
                        parent_jump_index: sibling_jump_index
                    }),
                _ => None
            }
        } else {
            None
        }
    }

    pub fn last_child_node(&self, cursor: RawCursor) -> RawCursor {
        let mut jump_index = cursor.parent_jump_index;
        let mut jump = self.jumps[jump_index];

        while let MajorNode::Jump{child_jump_index} = jump.next_major_node {
            jump_index = self.jumps[child_jump_index + 1..].windows(2).zip(child_jump_index + 1..)
                .find(|&(x, _)| x[0].parent_jump_index != x[1].parent_jump_index)
                .map(|(_, i)| i).unwrap_or(self.jumps.len() - 1);

            jump = self.jumps[jump_index];
        }

        RawCursor {
            node_index: jump.jump_to_node + jump.next_major_node_dist as isize,
            parent_jump_index: jump_index
        }
    }

    pub fn node_leaf(&self, cursor: RawCursor) -> Option<&L> {
        let parent_jump = self.jumps[cursor.parent_jump_index];
        match parent_jump.next_major_node_dist == (cursor.node_index - parent_jump.jump_to_node) as usize {
            true => match parent_jump.next_major_node {
                MajorNode::Leaf{ leaf_index: -1 } |
                MajorNode::Jump{..} => None,
                MajorNode::Leaf{ leaf_index } => Some(&self.leaves[leaf_index as usize])
            }
            false => None
        }
    }

    fn find_leaf<M, I>(&self, leaf: &M, ranges: I) -> Option<RawCursor>
        where M: Eq + ?Sized,
              L: Borrow<M>,
              I: IntoIterator<Item=Range<usize>>
    {
        let mut leaf_jump_index_opt = None;
        for (jump, jump_index) in ranges.into_iter().flat_map(|r| self.jumps[r.clone()].iter().zip(r)) {
            match jump.next_major_node {
                MajorNode::Leaf{leaf_index} if leaf_index != -1 => {
                    if self.leaves[leaf_index as usize].borrow() == leaf {
                        leaf_jump_index_opt = Some(jump_index);
                        break;
                    }
                },
                _ => ()
            }
        }

        leaf_jump_index_opt.map(|jump_index| {
            let jump = self.jumps[jump_index];
            RawCursor {
                node_index: jump.jump_to_node + jump.next_major_node_dist as isize,
                parent_jump_index: jump_index
            }
        })
    }

    pub fn find_leaf_after_wrapping<M>(&self, cursor: RawCursor, leaf: &M) -> Option<RawCursor>
        where M: Eq + ?Sized,
              L: Borrow<M>
    {
        if self.leaves.len() == 0 {
            return None;
        }

        self.find_leaf(leaf, [cursor.parent_jump_index + 1..self.jumps.len(), 0..cursor.parent_jump_index + 1].iter().cloned())
    }

    pub fn insert_nodes_after<I>(&mut self, cursor: RawCursor, nodes: I, leaf_opt: Option<L>) -> RawCursor
        where I: IntoIterator<Item=N>,
              I::IntoIter: ExactSizeIterator
    {
        let mut nodes = nodes.into_iter();
        if nodes.len() == 0 {
            return cursor;
        }

        let mut num_children = 0;
        let num_nodes_insert = nodes.len();
        let first_node = nodes.next().unwrap();
        for child_cursor in self.node_direct_children(cursor) {
            num_children += 1;
            if self.nodes[child_cursor.node_index as usize] == first_node {
                panic!("Attempt to insert node when node already exists");
            }
        }


        let insert_node_index: usize;
        let (insert_continue_jump, insert_split_jump): (bool, bool);
        let cursor_parent_jump = self.jumps[cursor.parent_jump_index];

        match num_children {
            0 => {
                insert_node_index = (cursor.node_index + 1) as usize;
                if let Some(jump) = self.jumps.get_mut(cursor.parent_jump_index) {
                    jump.next_major_node_dist += num_nodes_insert;
                }
                insert_continue_jump = false;
                insert_split_jump = false;
            },
            1 => {
                insert_node_index = (self.last_child_node(cursor).node_index + 1) as usize;
                insert_continue_jump = !cursor_parent_jump.cursor_at_next_major_node(cursor);
                insert_split_jump = true;
            },
            _ => {
                insert_node_index = (self.last_child_node(cursor).node_index + 1) as usize;
                insert_continue_jump = false;
                insert_split_jump = true;
            }
        }

        let (mut continue_jump, mut split_jump) = (None, None);
        let (insert_continue_jump_index, insert_split_jump_index): (usize, usize);
        let leaf_jump_index: usize;

        match insert_continue_jump {
            false => insert_continue_jump_index = usize::max_value(),
            true => {
                let jump = Jump {
                    parent_jump_index: cursor.parent_jump_index as isize,
                    jump_to_node: cursor.node_index + 1,
                    next_major_node_dist: 0,
                    next_major_node: cursor_parent_jump.next_major_node
                };
                continue_jump = Some(jump);

                insert_continue_jump_index = self.jumps[cursor.parent_jump_index..]
                    .binary_search(&jump).unwrap_err() + cursor.parent_jump_index;
            }
        }
        match insert_split_jump {
            false => {
                insert_split_jump_index = usize::max_value();
                leaf_jump_index = cursor.parent_jump_index;
            },
            true => {
                let jump = Jump {
                    parent_jump_index: cursor.parent_jump_index as isize,
                    jump_to_node: insert_node_index as isize,
                    next_major_node_dist: 0,
                    next_major_node: MajorNode::Leaf{ leaf_index: -1 }
                };
                split_jump = Some(jump);

                insert_split_jump_index =
                    self.jumps[cursor.parent_jump_index..]
                        .binary_search(&jump).unwrap_err() +
                        cursor.parent_jump_index +
                        // If a continue jump is being inserted at the same time as a split jump, the
                        // index generated by the `binary_search` call will be one off from where the
                        // split jump actually needs to be inserted to maintain sorted order. This
                        // next line corrects for that.
                        insert_continue_jump as usize;
                leaf_jump_index = insert_split_jump_index;
            }
        }

        let mut leaf_insert_index = 0;
        for (jump_index, jump) in self.jumps[..].iter_mut().enumerate() {
            match jump.next_major_node {
                MajorNode::Jump{ref mut child_jump_index} => {
                    if insert_continue_jump_index <= *child_jump_index {
                        *child_jump_index += 1;
                    }
                    if insert_split_jump_index <= *child_jump_index {
                        *child_jump_index += 1;
                    }
                },
                MajorNode::Leaf{ref mut leaf_index} if *leaf_index != -1 => {
                    match jump_index < leaf_jump_index {
                        true => leaf_insert_index = *leaf_index + 1,
                        false => *leaf_index += leaf_opt.is_some() as isize
                    }
                }
                _ => ()
            }

            if insert_continue_jump_index <= jump.parent_jump_index as usize && jump.parent_jump_index != -1 {
                jump.parent_jump_index += 1;
            }
            if insert_split_jump_index <= jump.parent_jump_index as usize && jump.parent_jump_index != -1 {
                jump.parent_jump_index += 1;
            }

            if insert_node_index <= jump.jump_to_node as usize && jump.jump_to_node != -1 {
                jump.jump_to_node += num_nodes_insert as isize;
            }
        }
        self.nodes.splice(insert_node_index..insert_node_index, Some(first_node).into_iter().chain(nodes));

        // Insert the jumps into the jump vec
        if insert_continue_jump {
            self.jumps.insert(insert_continue_jump_index, continue_jump.unwrap());
        }

        if insert_split_jump {
            self.jumps.insert(insert_split_jump_index, split_jump.unwrap());
        }

        if let Some(leaf) = leaf_opt {
            match self.jumps[leaf_jump_index].next_major_node {
                MajorNode::Leaf{ref mut leaf_index} => {
                    if *leaf_index != -1 {
                        self.leaves.remove(*leaf_index as usize);
                    }
                    *leaf_index = leaf_insert_index;
                    self.leaves.insert(*leaf_index as usize, leaf);
                },
                MajorNode::Jump{..} => panic!("Unexpected next jump")
            }
        }

        // Update the parent jump's next major node
        {
            let parent_jump_mut = &mut self.jumps[cursor.parent_jump_index];

            let jump_inserted = insert_continue_jump || insert_split_jump;
            if jump_inserted {
                parent_jump_mut.next_major_node = MajorNode::Jump {
                    child_jump_index: match parent_jump_mut.next_major_node {
                        MajorNode::Jump{child_jump_index} => child_jump_index,
                        MajorNode::Leaf{..} => insert_continue_jump_index
                    }
                };
            }
            parent_jump_mut.next_major_node_dist = (!jump_inserted) as usize + (cursor.node_index - parent_jump_mut.jump_to_node) as usize;
        }

        // Sanity check to see if the jump list is sorted.
        debug_assert!(self.jumps.windows(2).all(|x| x[0] < x[1]));
        for (i, jump) in self.jumps.iter().enumerate() {
            match jump.next_major_node {
                MajorNode::Jump{child_jump_index} => assert_eq!(i, self.jumps[child_jump_index].parent_jump_index as usize),
                _ => ()
            }
        }

        RawCursor {
            node_index: insert_node_index as isize,
            parent_jump_index: match self.jumps[cursor.parent_jump_index].next_major_node {
                MajorNode::Leaf{..} => cursor.parent_jump_index,
                MajorNode::Jump{..} if insert_split_jump => insert_split_jump_index,
                MajorNode::Jump{child_jump_index} => child_jump_index
            }
        }
    }

    pub fn prune_node(&mut self, cursor: RawCursor) {
        if cursor == RawCursor::root() {
            self.nodes.clear();
            self.jumps.clear();
            self.jumps.extend(Some(Jump::default_root()));
            self.leaves.clear();
        } else {
            let last_child_node = self.last_child_node(cursor);
            let first_child_jump_index: usize;

            let parent_jump = self.jumps[cursor.parent_jump_index];
            first_child_jump_index = match parent_jump.next_major_node {
                MajorNode::Jump{child_jump_index} => child_jump_index,
                MajorNode::Leaf{..} => usize::max_value()
            };

            {
                let parent_jump_mut = &mut self.jumps[cursor.parent_jump_index];
                parent_jump_mut.next_major_node = MajorNode::Leaf{ leaf_index: -1 };
                parent_jump_mut.next_major_node_dist = (cursor.node_index - parent_jump.jump_to_node) as usize;
            }


            self.nodes.splice(cursor.node_index as usize..last_child_node.node_index as usize + 1, None);

            let nodes_removed = (last_child_node.node_index + 1) - cursor.node_index;
            let remove_parent_jump_index = match cursor.node_index == parent_jump.jump_to_node {
                true => cursor.parent_jump_index,
                false => usize::max_value()
            };

            let mut retain_index = 0;
            let mut child_jump_count = 0;
            let mut leaf_remove_range = 0usize..0;
            self.jumps.retain_mut(|jump| {
                let in_child_jump_range =
                    first_child_jump_index <= retain_index &&
                    jump.jump_to_node <= last_child_node.node_index;
                let retain = !(
                    in_child_jump_range ||
                    retain_index == cursor.parent_jump_index && cursor.node_index == parent_jump.jump_to_node
                );

                if last_child_node.node_index <= jump.jump_to_node {
                    jump.jump_to_node -= nodes_removed;
                }

                {
                    let jump_offset = |jump_index: isize| {
                        ((remove_parent_jump_index as isize) < jump_index) as usize +
                        match first_child_jump_index <= retain_index && last_child_node.node_index < jump_index {
                            false => 0,
                            true => child_jump_count
                        }
                    };

                    jump.parent_jump_index -= jump_offset(jump.parent_jump_index) as isize;
                    match jump.next_major_node {
                        MajorNode::Jump{ref mut child_jump_index} => {
                            *child_jump_index -= jump_offset(*child_jump_index as isize);
                        }
                        MajorNode::Leaf{ref mut leaf_index} if *leaf_index != -1 => {
                            if retain_index < first_child_jump_index {
                                // This branch is before the child jump range
                                leaf_remove_range.start = (*leaf_index + 1) as usize;
                                leaf_remove_range.end = leaf_remove_range.start;
                            } else if jump.jump_to_node <= last_child_node.node_index {
                                // This branch is during the child jump range
                                leaf_remove_range.end = (*leaf_index + 1) as usize;
                            } else {
                                // This branch is after the child jump range
                                *leaf_index -= leaf_remove_range.len() as isize;
                            }
                        },
                        _ => ()
                    }
                }

                if in_child_jump_range {
                    child_jump_count += 1
                }
                retain_index += 1;

                retain
            });
            self.leaves.splice(leaf_remove_range, None);
            debug_assert!(self.jumps.windows(2).all(|x| x[0] < x[1]));
        }
    }
}

impl Jump {
    #[inline]
    fn default_root() -> Jump {
        Jump {
            parent_jump_index: -1,
            jump_to_node: -1,
            next_major_node_dist: 0,
            next_major_node: MajorNode::Leaf{ leaf_index: -1 }
        }
    }
    #[inline]
    fn cursor_at_next_major_node(&self, cursor: RawCursor) -> bool {
        let cursor_dist = cursor.node_index - self.jump_to_node;
        self.next_major_node_dist == cursor_dist as usize
    }

    #[inline]
    fn cursor_has_children(&self, cursor: RawCursor) -> bool {
        let cursor_dist = cursor.node_index - self.jump_to_node;
        cursor_dist < self.next_major_node_dist as isize || self.next_major_node.is_jump()
    }
}

impl RawCursor {
    pub fn root() -> RawCursor {
        RawCursor {
            node_index: -1,
            parent_jump_index: 0
        }
    }
}

impl MajorNode {
    #[inline]
    fn is_jump(self) -> bool {
        match self {
            MajorNode::Jump{..} => true,
            MajorNode::Leaf{..} => false
        }
    }
}
