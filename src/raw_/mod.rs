use std::cmp::Eq;
// use retain_external::RetainExternal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTree<N: Eq> {
    nodes: Vec<N>,
    /// The jumptions in the tree
    jumps: Vec<Jump>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MajorNode {
    Leaf,
    Jump
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Jump {
    branch_from_node: isize,
    depth: isize,
    jump_to_node: usize,
    next_major_node: MajorNode,
    /// The distance from the node at `jump_to_node` to to next node that's eithe a leaf or a
    /// split
    next_major_node_dist: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RawCursor {
    node_index: usize,
    parent_jump_index: usize
}

impl<N: Eq> RawTree<N> {
    pub fn new(root: N) -> RawTree<N> {
        RawTree {
            nodes: vec![root],
            jumps: vec![Jump::root()]
        }
    }

    pub fn get_node(&self, cursor: RawCursor) -> &N {
        &self.nodes[cursor.node_index]
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
            (MajorNode::Jump, true) => {
                direct_child = None;
                child_jump_search = cursor.parent_jump_index..;
            }
        }


        let jump_child_iter = self.jumps[child_jump_search.clone()].iter().cloned()
            .zip(child_jump_search)
            .skip_while(move |&(s, _)| s.branch_from_node != cursor.node_index as isize)
            .take_while(move |&(s, _)| s.branch_from_node == cursor.node_index as isize)
            .map(|(s, i)| RawCursor {
                node_index: s.jump_to_node,
                parent_jump_index: i
            });
        direct_child.into_iter().chain(jump_child_iter)
    }

    pub fn node_right_leaf(&self, cursor: RawCursor) -> RawCursor {
        let mut jump_index = cursor.parent_jump_index;
        let mut jump = self.jumps[jump_index];

        while jump.next_major_node != MajorNode::Leaf {
            jump_index = self.jumps[jump_index + 1..].windows(2).zip(jump_index + 1..)
                .skip_while(|&(x, _)| x[0].branch_from_node != jump.branch_from_node)
                .find(|&(x, _)| x[0].branch_from_node != x[1].branch_from_node)
                .map(|(_, i)| i).unwrap_or(self.jumps.len() - 1);

            jump = self.jumps[jump_index];
        }

        RawCursor {
            node_index: jump.jump_to_node + jump.next_major_node_dist,
            parent_jump_index: jump_index
        }
    }

    pub fn insert_node_after(&mut self, cursor: RawCursor, node: N) {
        let mut num_children = 0;
        for child_cursor in self.node_direct_children(cursor) {
            num_children += 1;
            if self.nodes[child_cursor.node_index] == node {
                panic!("Attempt to insert node when node already exists");
            }
        }


        let insert_node_index: usize;
        let (insert_continue_jump, insert_split_jump): (bool, bool);

        match num_children {
            0 => {
                insert_node_index = cursor.node_index + 1;
                self.jumps[cursor.parent_jump_index].next_major_node_dist += 1;
                insert_continue_jump = false;
                insert_split_jump = false;
            },
            1 => {
                insert_node_index = self.node_right_leaf(cursor).node_index + 1;
                insert_continue_jump = !self.jumps[cursor.parent_jump_index].cursor_at_next_major_node(cursor);
                insert_split_jump = true;
            },
            _ => {
                insert_node_index = self.node_right_leaf(cursor).node_index + 1;
                insert_continue_jump = false;
                insert_split_jump = true;
            }
        }

        for jump in &mut self.jumps[cursor.parent_jump_index + 1..] {
            debug_assert!(jump.branch_from_node < jump.jump_to_node as isize);

            if (insert_node_index as isize) <= jump.branch_from_node {
                jump.branch_from_node += 1;
            }
            if insert_node_index <= jump.jump_to_node {
                jump.jump_to_node += 1;
            }
        }
        self.nodes.insert(insert_node_index, node);


        if insert_continue_jump {
            let jump = self.jumps[cursor.parent_jump_index].new_child_continue(cursor.node_index);
            let insert_index = self.jumps[cursor.parent_jump_index..].binary_search(&jump)
                .unwrap_err() + cursor.parent_jump_index;
            self.jumps.insert(insert_index, jump);
        }

        if insert_split_jump {
            let jump = self.jumps[cursor.parent_jump_index]
                .new_child_jump(cursor.node_index, insert_node_index);
            let insert_index = self.jumps[cursor.parent_jump_index..].binary_search(&jump)
                .unwrap_err() + cursor.parent_jump_index;
            self.jumps.insert(insert_index, jump);
        }

        // Sanity check to see if the jump list is sorted.
        debug_assert!(self.jumps.windows(2).all(|x| x[0] <= x[1]));
    }

    // pub fn prune_node(&mut self, cursor: RawCursor) {
    //     use std::vec::VecDeque;

    //     let node_right_leaf = self.node_right_leaf(cursor);
    //     self.nodes.splice(cursor.node_index..node_right_leaf.node_index + 1, None);

    //     {
    //         let mut parent_jump = &mut self.jumps[cursor.parent_jump_index];
    //         parent_jump.next_major_node = MajorNode::Leaf;
    //         parent_jump.next_major_node_dist = cursor.node_index - parent_jump.jump_to_node;
    //     }

    //     let mut remove_branch_from = VecDeque::new();
    //     remove_branch_from.push_back(cursor.node_index);

    //     let mut retain_external = RetainExternal::new(&mut self.jumps, cursor.parent_jump_index..);
    //     while let Some(retain_jump) = retain_external.next() {
    //         let jump = *retain_jump.element();
    //         if jump.branch_from_node == remove_branch_from.back().unwrap() {
    //             retain_jump.discard();
    //         } else {
    //             retain_jump.retain();
    //         }
    //     }
    // }
}

impl Jump {
    #[inline]
    fn root() -> Jump {
        Jump {
            branch_from_node: -1,
            depth: -1,
            jump_to_node: 0,
            next_major_node: MajorNode::Leaf,
            next_major_node_dist: 0
        }
    }

    fn new_child_continue(&mut self, branch_from_node: usize) -> Jump {
        let jump_to_node = branch_from_node + 1;

        let branch_from_node = branch_from_node as isize;
        let child_depth = (jump_to_node - self.jump_to_node) as isize + self.depth;

        let jump_next_major_node = self.next_major_node;
        self.next_major_node = MajorNode::Jump;

        assert!(self.depth < child_depth && child_depth <= self.depth + 1 + self.next_major_node_dist as isize);
        assert!(self.jump_to_node < jump_to_node && jump_to_node <= self.jump_to_node + 1 + self.next_major_node_dist);

        self.next_major_node_dist = branch_from_node as usize - self.jump_to_node;

        Jump {
            branch_from_node,
            depth: child_depth,
            jump_to_node,
            next_major_node: jump_next_major_node,
            next_major_node_dist: 0
        }
    }

    fn new_child_jump(&mut self, branch_from_node: usize, jump_to_node: usize) -> Jump {
        let branch_from_node = branch_from_node as isize;
        let child_depth = branch_from_node - self.jump_to_node as isize + 1 + self.depth;

        self.next_major_node = MajorNode::Jump;

        assert!(self.depth < child_depth && child_depth <= self.depth + 1 + self.next_major_node_dist as isize);
        assert!(self.next_major_node_dist + self.jump_to_node < jump_to_node);

        self.next_major_node_dist = branch_from_node as usize - self.jump_to_node;

        Jump {
            branch_from_node,
            depth: child_depth,
            jump_to_node,
            next_major_node: MajorNode::Leaf,
            next_major_node_dist: 0
        }
    }

    #[inline]
    fn cursor_at_next_major_node(&self, cursor: RawCursor) -> bool {
        let cursor_dist = cursor.node_index - self.jump_to_node;
        self.next_major_node_dist == cursor_dist
    }

    #[inline]
    fn cursor_has_children(&self, cursor: RawCursor) -> bool {
        let cursor_dist = cursor.node_index - self.jump_to_node;
        cursor_dist < self.next_major_node_dist || self.next_major_node == MajorNode::Jump
    }
}

impl RawCursor {
    #[inline]
    pub fn root() -> RawCursor {
        RawCursor {
            node_index: 0,
            parent_jump_index: 0
        }
    }
}
