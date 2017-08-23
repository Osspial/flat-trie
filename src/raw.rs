use std::cmp::Eq;

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
    branch_node_index: isize,
    depth: isize,
    jump_node_index: usize,
    next_major_node: MajorNode,
    /// The distance from the node at `jump_node_index` to to next node that's eithe a leaf or a
    /// split
    next_major_node_dist: usize,
}

#[derive(Clone, Copy)]
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
                });
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
            .skip_while(move |&(s, _)| s.branch_node_index != cursor.node_index as isize)
            .take_while(move |&(s, _)| s.branch_node_index == cursor.node_index as isize)
            .map(|(s, i)| RawCursor {
                node_index: s.jump_node_index,
                parent_jump_index: i
            });
        direct_child.into_iter().chain(jump_child_iter)
    }

    pub fn insert_node_after(&mut self, cursor: RawCursor, node: N) where N: ::std::fmt::Debug {
        let mut num_children = 0;
        for child_cursor in self.node_direct_children(cursor) {
            num_children += 1;
            if self.nodes[child_cursor.node_index] == node {
                panic!("Attempt to insert node when node already exists");
            }
        }

        let (insert_continue_jump, insert_split_jump): (bool, bool);
        println!("{:?} -> {:?} {}", node, self.nodes[cursor.node_index], num_children);
        println!("{:#?}", self);
        match num_children {
            0 => {
                self.nodes.insert(cursor.node_index + 1, node);
                insert_continue_jump = true;
                insert_split_jump = false;
            },
            1 => {
                self.nodes.push(node);
                insert_continue_jump = !self.jumps[cursor.parent_jump_index].cursor_at_next_major_node(cursor);
                insert_split_jump = true;
            },
            _ => {
                self.nodes.push(node);
                insert_continue_jump = false;
                insert_split_jump = true;
            }
        }


        let mut jump_offset_one_range = 0..0;
        let mut jump_offset_two_range = 0..0;

        if insert_continue_jump {
            let jump = self.jumps[cursor.parent_jump_index].new_child_continue(cursor.node_index);
            let insert_index = self.jumps[cursor.parent_jump_index..].binary_search(&jump)
                .unwrap_err() + cursor.parent_jump_index;
            self.jumps.insert(insert_index, jump);

            jump_offset_one_range = insert_index + 1..self.jumps.len();
        }

        if insert_split_jump {
            let jump = self.jumps[cursor.parent_jump_index]
                .new_child_jump(cursor.node_index, self.nodes.len() - 1);
            let insert_index = self.jumps[cursor.parent_jump_index..].binary_search(&jump)
                .unwrap_err() + cursor.parent_jump_index;
            self.jumps.insert(insert_index, jump);

            match jump_offset_one_range.end < insert_index {
                true => {
                    jump_offset_two_range = insert_index + 1..jump_offset_one_range.end;
                    jump_offset_one_range.end = insert_index;
                },
                false => jump_offset_one_range = insert_index + 1..self.jumps.len()
            };
        }

        for jump in &mut self.jumps[jump_offset_one_range] {
            jump.branch_node_index += 1;
            jump.jump_node_index += 1;
        }
        for jump in &mut self.jumps[jump_offset_two_range] {
            jump.branch_node_index += 2;
            jump.jump_node_index += 2;
        }

        #[cfg(debug)]
        {
            let mut jumps_sorted = self.jumps.clone();
            jumps_sorted.sort();
            assert!(self.jumps == jumps_sorted);
        }
    }
}

impl Jump {
    fn root() -> Jump {
        Jump {
            branch_node_index: -1,
            depth: -1,
            jump_node_index: 0,
            next_major_node: MajorNode::Leaf,
            next_major_node_dist: 0
        }
    }

    fn new_child_continue(&mut self, branch_node_index: usize) -> Jump {
        self.next_major_node_dist = branch_node_index - self.jump_node_index;
        let jump_node_index = branch_node_index + 1;

        let branch_node_index = branch_node_index as isize;
        let child_depth = (jump_node_index - self.jump_node_index) as isize + self.depth;

        let jump_next_major_node = self.next_major_node;
        self.next_major_node = MajorNode::Jump;

        assert!(self.depth < child_depth && child_depth <= self.depth + 1 + self.next_major_node_dist as isize);
        assert!(self.jump_node_index < jump_node_index && jump_node_index <= self.jump_node_index + 1 + self.next_major_node_dist);

        Jump {
            branch_node_index,
            depth: child_depth,
            jump_node_index,
            next_major_node: jump_next_major_node,
            next_major_node_dist: 0
        }
    }
    
    fn new_child_jump(&mut self, branch_node_index: usize, jump_node_index: usize) -> Jump {
        self.next_major_node_dist = branch_node_index - self.jump_node_index;

        let branch_node_index = branch_node_index as isize;
        let child_depth = branch_node_index - self.jump_node_index as isize + 1 + self.depth;

        self.next_major_node = MajorNode::Jump;

        assert!(self.depth < child_depth && child_depth <= self.depth + 1 + self.next_major_node_dist as isize);
        assert!(self.next_major_node_dist + self.jump_node_index < jump_node_index);

        Jump {
            branch_node_index,
            depth: child_depth,
            jump_node_index,
            next_major_node: MajorNode::Leaf,
            next_major_node_dist: 0
        }
    }

    fn cursor_at_next_major_node(&self, cursor: RawCursor) -> bool {
        let cursor_dist = cursor.node_index - self.jump_node_index;
        self.next_major_node_dist == cursor_dist
    }
}

impl RawCursor {
    pub fn root() -> RawCursor {
        RawCursor {
            node_index: 0,
            parent_jump_index: 0
        }
    }
}
