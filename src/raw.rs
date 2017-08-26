use std::cmp::Eq;
use odds::vec::VecExt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTree<N: Eq> {
    pub nodes: Vec<N>,
    /// The jumptions in the tree
    jumps: Vec<Jump>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MajorNode {
    Leaf,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawCursor {
    node_index: isize,
    parent_jump_index: usize
}

impl<N: Eq> RawTree<N> {
    pub fn new() -> RawTree<N> {
        RawTree {
            nodes: vec![],
            jumps: vec![Jump::default_root()]
        }
    }

    pub fn get_node(&self, cursor: RawCursor) -> Option<&N> {
        self.nodes.get(cursor.node_index as usize)
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
            (MajorNode::Leaf, true) => {
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

    pub fn node_last_leaf(&self, cursor: RawCursor) -> RawCursor {
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

    pub fn insert_node_after(&mut self, cursor: RawCursor, node: N) {
        let mut num_children = 0;
        for child_cursor in self.node_direct_children(cursor) {
            num_children += 1;
            if self.nodes[child_cursor.node_index as usize] == node {
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
                    jump.next_major_node_dist += 1;
                }
                insert_continue_jump = false;
                insert_split_jump = false;
            },
            1 => {
                insert_node_index = (self.node_last_leaf(cursor).node_index + 1) as usize;
                insert_continue_jump = !cursor_parent_jump.cursor_at_next_major_node(cursor);
                insert_split_jump = true;
            },
            _ => {
                insert_node_index = (self.node_last_leaf(cursor).node_index + 1) as usize;
                insert_continue_jump = false;
                insert_split_jump = true;
            }
        }

        let (mut continue_jump, mut split_jump) = (None, None);
        let (insert_continue_jump_index, insert_split_jump_index): (usize, usize);

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
            false => insert_split_jump_index = usize::max_value(),
            true => {
                let jump = Jump {
                    parent_jump_index: cursor.parent_jump_index as isize,
                    jump_to_node: insert_node_index as isize,
                    next_major_node_dist: 0,
                    next_major_node: MajorNode::Leaf
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
            }
        }

        for jump in &mut self.jumps[..] {
            match jump.next_major_node {
                MajorNode::Jump{ref mut child_jump_index} => {
                    if insert_continue_jump_index <= *child_jump_index {
                        *child_jump_index += 1;
                    }
                    if insert_split_jump_index <= *child_jump_index {
                        *child_jump_index += 1;
                    }
                },
                _ => ()
            }

            if insert_continue_jump_index < jump.parent_jump_index as usize && jump.parent_jump_index != -1 {
                jump.parent_jump_index += 1;
            }
            if insert_split_jump_index < jump.parent_jump_index as usize && jump.parent_jump_index != -1 {
                jump.parent_jump_index += 1;
            }

            if insert_node_index <= jump.jump_to_node as usize && jump.jump_to_node != -1 {
                jump.jump_to_node += 1;
            }
        }
        self.nodes.insert(insert_node_index, node);

        if insert_continue_jump {
            self.jumps.insert(insert_continue_jump_index, continue_jump.unwrap());
        }

        if insert_split_jump {
            self.jumps.insert(insert_split_jump_index, split_jump.unwrap());
        }

        {
            let parent_jump_mut = &mut self.jumps[cursor.parent_jump_index];

            let jump_inserted = insert_continue_jump || insert_split_jump;
            if jump_inserted {
                parent_jump_mut.next_major_node = MajorNode::Jump {
                    child_jump_index: match parent_jump_mut.next_major_node {
                        MajorNode::Jump{child_jump_index} => child_jump_index,
                        MajorNode::Leaf => insert_continue_jump_index
                    }
                };
            }
            parent_jump_mut.next_major_node_dist = (!jump_inserted) as usize + (cursor.node_index - parent_jump_mut.jump_to_node) as usize;
        }

        // Sanity check to see if the jump list is sorted.
        debug_assert!(self.jumps.windows(2).all(|x| x[0] < x[1]));
    }

    pub fn prune_node(&mut self, cursor: RawCursor) {
        if cursor == RawCursor::root() {
            self.nodes.clear();
            self.jumps.clear();
            self.jumps.extend(Some(Jump::default_root()));
        } else {
            let node_last_leaf = self.node_last_leaf(cursor);
            let first_child_jump_index: usize;

            let parent_jump = self.jumps[cursor.parent_jump_index];
            first_child_jump_index = match parent_jump.next_major_node {
                MajorNode::Jump{child_jump_index} => child_jump_index,
                MajorNode::Leaf => usize::max_value()
            };

            {
                let parent_jump_mut = &mut self.jumps[cursor.parent_jump_index];
                parent_jump_mut.next_major_node = MajorNode::Leaf;
                parent_jump_mut.next_major_node_dist = (cursor.node_index - parent_jump.jump_to_node) as usize;
            }


            self.nodes.splice(cursor.node_index as usize..node_last_leaf.node_index as usize + 1, None);

            let nodes_removed = (node_last_leaf.node_index + 1) - cursor.node_index;
            let remove_parent_jump_index = match cursor.node_index == parent_jump.jump_to_node {
                true => cursor.parent_jump_index,
                false => usize::max_value()
            };

            let mut retain_index = 0;
            let mut child_jump_count = 0;
            self.jumps.retain_mut(|jump| {
                let in_child_jump_range =
                    first_child_jump_index <= retain_index &&
                    jump.jump_to_node <= node_last_leaf.node_index;
                let retain = !(
                    in_child_jump_range ||
                    retain_index == cursor.parent_jump_index && cursor.node_index == parent_jump.jump_to_node
                );

                if node_last_leaf.node_index <= jump.jump_to_node {
                    jump.jump_to_node -= nodes_removed;
                }

                {
                    let jump_offset = |jump_index: isize| {
                        ((remove_parent_jump_index as isize) < jump_index) as usize +
                        match first_child_jump_index <= retain_index && node_last_leaf.node_index < jump_index {
                            false => 0,
                            true => child_jump_count
                        }
                    };

                    jump.parent_jump_index -= jump_offset(jump.parent_jump_index) as isize;
                    if let MajorNode::Jump{ref mut child_jump_index} = jump.next_major_node {
                        *child_jump_index -= jump_offset(*child_jump_index as isize);
                    }
                }

                if in_child_jump_range {
                    child_jump_count += 1
                }
                retain_index += 1;

                retain
            });
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
            next_major_node: MajorNode::Leaf
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
            MajorNode::Leaf     => false
        }
    }
}
