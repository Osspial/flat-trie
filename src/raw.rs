use std::cmp::Eq;
use std::borrow::Borrow;
use std::ops::Range;
use std::iter::ExactSizeIterator;
use odds::vec::VecExt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTrie<N: Eq, L> {
    nodes: Vec<N>,
    /// The jumptions in the tree
    jumps: Vec<Jump>,
    leaves: Vec<L>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawCursor {
    node_index: isize,
    parent_jump_index: usize,
    depth: isize
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MajorNode {
    LeafJump {
        leaf_index: usize,
        child_jump_index: usize
    },
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
    next_major_node: MajorNode,
    depth: isize
}

impl<N: Eq, L> RawTrie<N, L> {
    pub fn new() -> RawTrie<N, L> {
        RawTrie {
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
                        parent_jump_index: parent_jump.parent_jump_index as usize,
                        depth: cursor.depth - 1
                    })
            }
            false => Some(RawCursor {
                node_index: cursor.node_index - 1,
                parent_jump_index: cursor.parent_jump_index,
                depth: cursor.depth - 1
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
                    parent_jump_index: cursor.parent_jump_index,
                    depth: cursor.depth + 1
                }).into_iter().filter(|_| parent_jump.cursor_has_children(cursor)).next();
                child_jump_search = self.jumps.len()..;
            }
            (MajorNode::Leaf{..}, true) => {
                direct_child = None;
                child_jump_search = self.jumps.len()..;
            }
            (MajorNode::LeafJump{child_jump_index, ..}, true) |
            (MajorNode::Jump{child_jump_index}, true) => {
                direct_child = None;
                child_jump_search = child_jump_index..;
            }
        }


        let jump_child_iter = self.jumps[child_jump_search.clone()].iter().cloned()
            .zip(child_jump_search)
            .take_while(move |&(s, _)| s.parent_jump_index == cursor.parent_jump_index as isize)
            .map(move |(s, i)| RawCursor {
                node_index: s.jump_to_node,
                parent_jump_index: i,
                depth: cursor.depth + 1
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
                        parent_jump_index: sibling_jump_index,
                        depth: cursor.depth
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

        loop {
            match jump.next_major_node {
                MajorNode::Leaf{..} => break,
                MajorNode::LeafJump{child_jump_index, ..} |
                MajorNode::Jump{child_jump_index} => {
                    jump_index = self.jumps[child_jump_index + 1..].windows(2).zip(child_jump_index + 1..)
                        .find(|&(x, _)| x[0].parent_jump_index != x[1].parent_jump_index)
                        .map(|(_, i)| i).unwrap_or(self.jumps.len() - 1);

                    jump = self.jumps[jump_index];
                }
            }
        }

        RawCursor {
            node_index: jump.jump_to_node + jump.next_major_node_dist as isize,
            parent_jump_index: jump_index,
            depth: jump.depth + jump.next_major_node_dist as isize
        }
    }

    fn get_leaf_index(&self, cursor: RawCursor) -> Option<usize> {
        let parent_jump = self.jumps[cursor.parent_jump_index];
        match parent_jump.next_major_node_dist == (cursor.node_index - parent_jump.jump_to_node) as usize {
            true => match parent_jump.next_major_node {
                MajorNode::Leaf{leaf_index: -1} |
                MajorNode::Jump{..} => None,
                MajorNode::LeafJump{leaf_index, ..} => Some(leaf_index),
                MajorNode::Leaf{leaf_index} => Some(leaf_index as usize)
            }
            false => None
        }
    }

    pub fn get_leaf(&self, cursor: RawCursor) -> Option<&L> {
        self.get_leaf_index(cursor).map(|i| &self.leaves[i])
    }

    pub fn get_leaf_mut(&mut self, cursor: RawCursor) -> Option<&mut L> {
        self.get_leaf_index(cursor).map(move |i| &mut self.leaves[i])
    }

    /// Returns a cursor at the specified child if it exists, or follows the chain as far as possible
    /// and gets the farthest child and the distance traveled.
    pub fn node_enter_children<'b, I, O>(&self, mut cursor: RawCursor, children: I) -> Result<RawCursor, (RawCursor, usize)>
        where I: IntoIterator<Item=&'b O>,
              N: Borrow<O>,
              O: 'b + Eq + ?Sized
    {
        for (i, child) in children.into_iter().enumerate() {
            let parent_jump = self.jumps[cursor.parent_jump_index];

            if parent_jump.cursor_at_next_major_node(cursor) {
                let cursor_opt: Option<RawCursor>;
                match parent_jump.next_major_node {
                    MajorNode::Leaf{..} => return Err((cursor, i)),
                    MajorNode::LeafJump{..} |
                    MajorNode::Jump{..} => cursor_opt =
                        self.node_direct_children(cursor)
                            .map(|cursor| (&self.nodes[cursor.node_index as usize], cursor))
                            .find(|&(node, _)| child == node.borrow()).map(|(_, cursor)| cursor)
                }
                match cursor_opt {
                    Some(nc) => cursor = nc,
                    None => return Err((cursor, i))
                }
            } else if child == self.nodes[(cursor.node_index + 1) as usize].borrow() {
                cursor.node_index += 1;
            } else {
                return Err((cursor, i))
            }
        }

        Ok(cursor)
    }

    pub fn common_ancestor(&self, left: RawCursor, right: RawCursor) -> RawCursor {
        let mut left_parent_index = left.parent_jump_index as isize;
        let mut right_parent_index = right.parent_jump_index as isize;

        while left_parent_index != right_parent_index {
            while left_parent_index < right_parent_index {
                right_parent_index = self.jumps[right_parent_index as usize].parent_jump_index;
            }
            while right_parent_index < left_parent_index {
                left_parent_index = self.jumps[left_parent_index as usize].parent_jump_index;
            }
        }

        match self.jumps.get(left_parent_index as usize) {
            Some(common_parent) => RawCursor {
                node_index: common_parent.jump_to_node + common_parent.next_major_node_dist as isize,
                parent_jump_index: left_parent_index as usize,
                depth: common_parent.depth + common_parent.next_major_node_dist as isize
            },
            None => RawCursor::root()
        }
    }

    pub fn route_to_descendant<'a>(&'a self, node: RawCursor, descendant: RawCursor) -> impl 'a + Iterator<Item=&'a N> {
        use std::{cmp, iter};

        let mut parent_jump_index = node.parent_jump_index;
        let mut parent_jump = self.jumps[parent_jump_index];

        let node_range_end = move |jump: Jump| (1 + cmp::min(descendant.node_index, (jump.jump_to_node + jump.next_major_node_dist as isize))) as usize;

        self.nodes[(node.node_index + 1) as usize..node_range_end(parent_jump)].iter().chain(
            // This iterator is what happens when an iterator equivalent of a `while` loop doesn't exist.
            iter::repeat(()).scan((), move |_, _| {
                match parent_jump_index < descendant.parent_jump_index && parent_jump.next_major_node.is_jump() {
                    true => {
                        let child_jump_index = match parent_jump.next_major_node {
                            MajorNode::Jump{child_jump_index} => child_jump_index,
                            _ => unreachable!()
                        };

                        parent_jump_index = self.jumps[child_jump_index..].iter()
                            .take_while(|j| j.parent_jump_index == parent_jump_index as isize)
                            .take_while(|j| j.jump_to_node <= descendant.node_index)
                            .zip(child_jump_index..)
                            .last().expect("`descendant` is not a descendant of `node`").1;
                        parent_jump = self.jumps[parent_jump_index];
                        Some(self.nodes[parent_jump.jump_to_node as usize..node_range_end(parent_jump)].iter())
                    },
                    false => {
                        let node_in_jump_range =
                            parent_jump.jump_to_node <= descendant.node_index &&
                            descendant.node_index <= parent_jump.jump_to_node + parent_jump.next_major_node_dist as isize;
                        if !node_in_jump_range {
                            panic!("`descendant` is not a descendant of `node`")
                        }
                        None
                    }
                }
            }).flat_map(|nodes| nodes).fuse()
        )
    }

    fn find_leaf_by<F, I>(&self, mut by: F, ranges: I) -> Option<RawCursor>
        where F: FnMut(&L) -> bool,
              I: IntoIterator<Item=Range<usize>>
    {
        let mut leaf_jump_index_opt = None;
        for (jump, jump_index) in ranges.into_iter().flat_map(|r| self.jumps[r.clone()].iter().zip(r)) {
            match jump.next_major_node {
                MajorNode::Leaf{leaf_index} if leaf_index != -1 => {
                    if by(&self.leaves[leaf_index as usize]) {
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
                parent_jump_index: jump_index,
                depth: jump.depth + jump.next_major_node_dist as isize
            }
        })
    }

    pub fn find_leaf_after_wrapping_by<F>(&self, cursor: RawCursor, by: F) -> Option<RawCursor>
        where F: FnMut(&L) -> bool
    {
        if self.leaves.len() == 0 {
            return None;
        }

        self.find_leaf_by(by, [cursor.parent_jump_index + 1..self.jumps.len(), 0..cursor.parent_jump_index + 1].iter().cloned())
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
        // Insert continue jump: Insert a jump to the node directly after the selected node.
        // Insert split jump: Insert a jump to the node being inserted now
        let (insert_continue_jump, insert_split_jump): (bool, bool);
        let cursor_parent_jump = self.jumps[cursor.parent_jump_index];
        let parent_has_leaf = match cursor_parent_jump.next_major_node {
            MajorNode::Leaf{leaf_index: -1} |
            MajorNode::Jump{..}            => false,
            MajorNode::LeafJump{..}         |
            MajorNode::Leaf{..}            => true
        };
        let cursor_at_next_major_node = cursor_parent_jump.cursor_at_next_major_node(cursor);

        match num_children {
            0 => {
                insert_node_index = (cursor.node_index + 1) as usize;
                if let Some(jump) = self.jumps.get_mut(cursor.parent_jump_index) {
                    jump.next_major_node_dist += num_nodes_insert;
                }
                insert_continue_jump = false;
                insert_split_jump = parent_has_leaf;
            },
            1 => {
                insert_node_index = (self.last_child_node(cursor).node_index + 1) as usize;
                insert_continue_jump = !cursor_at_next_major_node;
                insert_split_jump = true;
            },
            _ => {
                insert_node_index = (self.last_child_node(cursor).node_index + 1) as usize;
                insert_continue_jump = false;
                insert_split_jump = true;
            }
        }

        let (insert_continue_jump_index, insert_split_jump_index): (usize, usize);
        let leaf_jump_index: usize;

        let jump_depth = cursor_parent_jump.depth + cursor.node_index - cursor_parent_jump.jump_to_node + 1;
        match insert_continue_jump {
            false => insert_continue_jump_index = usize::max_value(),
            true => {
                let jump = Jump {
                    depth: jump_depth,
                    parent_jump_index: cursor.parent_jump_index as isize,
                    jump_to_node: cursor.node_index + 1,
                    next_major_node_dist: (cursor_parent_jump.jump_to_node + cursor_parent_jump.next_major_node_dist as isize - (cursor.node_index + 1)) as usize,
                    next_major_node: match cursor_parent_jump.next_major_node {
                        MajorNode::Jump{child_jump_index} => MajorNode::Jump{child_jump_index: child_jump_index + 1},
                        MajorNode::Leaf{..} => cursor_parent_jump.next_major_node,
                        MajorNode::LeafJump{child_jump_index, leaf_index} => MajorNode::LeafJump {
                            child_jump_index: child_jump_index + 1,
                            leaf_index
                        }
                    }
                };

                insert_continue_jump_index = self.jumps[cursor.parent_jump_index..]
                    .binary_search(&jump).unwrap_err() + cursor.parent_jump_index;
                self.jumps.insert(insert_continue_jump_index, jump);
            }
        }
        match insert_split_jump {
            false => {
                insert_split_jump_index = usize::max_value();
                leaf_jump_index = cursor.parent_jump_index;
            },
            true => {
                let jump = Jump {
                    depth: jump_depth,
                    parent_jump_index: cursor.parent_jump_index as isize,
                    jump_to_node: insert_node_index as isize,
                    next_major_node_dist: num_nodes_insert - 1,
                    next_major_node: MajorNode::Leaf{ leaf_index: -1 }
                };

                insert_split_jump_index =
                    self.jumps[cursor.parent_jump_index..]
                        .binary_search(&jump).unwrap_err() + cursor.parent_jump_index;
                leaf_jump_index = insert_split_jump_index;
                self.jumps.insert(insert_split_jump_index, jump);
            }
        }

        let mut leaf_insert_index: usize = 0;
        let mut children_rotate_range: Option<Range<usize>> = None;
        let mut children_rotate_to = self.jumps.len();

        for jump_index in 0..self.jumps.len() {
            let incr_jump_indices: bool;
            {
                let jump = self.jumps[jump_index];

                incr_jump_indices = !(
                    jump.parent_jump_index == -1 ||
                    jump_index == cursor.parent_jump_index ||
                    jump_index == insert_continue_jump_index ||
                    jump_index == insert_split_jump_index
                );
            }

            if incr_jump_indices {
                let jump = &mut self.jumps[jump_index];
                match jump.next_major_node {
                    MajorNode::LeafJump{ref mut child_jump_index, ..} |
                    MajorNode::Jump{ref mut child_jump_index} => {
                        if insert_continue_jump_index <= *child_jump_index {
                            *child_jump_index += 1;
                        }
                        if insert_split_jump_index <= *child_jump_index {
                            *child_jump_index += 1;
                        }
                    },
                    MajorNode::Leaf{..} => ()
                }

                if insert_node_index <= jump.jump_to_node as usize && jump.jump_to_node != -1 {
                    jump.jump_to_node += num_nodes_insert as isize;
                }
                if insert_continue_jump_index <= jump.parent_jump_index as usize {
                    jump.parent_jump_index += 1;
                }
                if insert_split_jump_index <= jump.parent_jump_index as usize {
                    jump.parent_jump_index += 1;
                }

                // If a continue jump has been inserted, children of the original jump have to have
                // their `parent_jump_index`s updated and be shifted around in the `jumps` vec to
                // maintain sorted order. This block updates the `parent_jump_index` and defines the
                // range of jumps that must be shifted.
                if insert_continue_jump && cursor.parent_jump_index == jump.parent_jump_index as usize {
                    jump.parent_jump_index = insert_continue_jump_index as isize;
                    match children_rotate_range {
                        Some(ref mut range) => range.end += 1,
                        None => {
                            children_rotate_range = Some(jump_index..jump_index + 1);
                        }
                    }
                }
            }

            if let Some(range) = children_rotate_range.clone() {
                let jump = self.jumps[jump_index];

                match self.jumps[range.end - 1] < jump {
                    true => {
                        if children_rotate_to == self.jumps.len() {
                            children_rotate_to = jump_index;
                        }
                        if range.contains(jump.parent_jump_index as usize) {
                            self.jumps[jump_index].parent_jump_index += ((children_rotate_to - range.start) - range.len()) as isize;
                        }
                    },
                    false => ()
                }
            }

            {
                let jump = self.jumps[jump_index];
                let leaf_jump = self.jumps[leaf_jump_index];
                match self.jumps[jump_index].next_major_node {
                    MajorNode::LeafJump{ref mut leaf_index, ..} => {
                        match jump < leaf_jump {
                            true => leaf_insert_index = *leaf_index + 1,
                            false => *leaf_index += 1
                        }
                    },
                    MajorNode::Leaf{ref mut leaf_index} if *leaf_index != -1 && leaf_opt.is_some() => {
                        match jump < leaf_jump {
                            true => leaf_insert_index = (*leaf_index + 1) as usize,
                            false => *leaf_index += 1
                        }
                    },
                    MajorNode::Leaf{..}  |
                    MajorNode::Jump{..} => ()
                }
            }
        }
        self.nodes.splice(insert_node_index..insert_node_index, Some(first_node).into_iter().chain(nodes));

        if let Some(leaf) = leaf_opt {
            match self.jumps[leaf_jump_index].next_major_node {
                MajorNode::Leaf{ref mut leaf_index} => {
                    if *leaf_index != -1 {
                        panic!("unexpected leaf index: {}", leaf_index);
                    }
                    *leaf_index = leaf_insert_index as isize;
                    self.leaves.insert(*leaf_index as usize, leaf);
                },
                MajorNode::LeafJump{..} |
                MajorNode::Jump{..} => panic!("Unexpected next jump")
            }
        }

        // Update the parent jump's next major node
        {
            let parent_jump_mut = &mut self.jumps[cursor.parent_jump_index];

            let jump_inserted = insert_continue_jump || insert_split_jump;
            if jump_inserted {
                use std::cmp;
                let jump_to = cmp::min(insert_continue_jump_index, insert_split_jump_index);
                parent_jump_mut.next_major_node = match parent_jump_mut.next_major_node {
                    MajorNode::Jump{..} => parent_jump_mut.next_major_node,
                    MajorNode::Leaf{leaf_index: -1} => MajorNode::Jump {
                        child_jump_index: jump_to
                    },
                    MajorNode::Leaf{leaf_index} => MajorNode::LeafJump {
                        leaf_index: leaf_index as usize,
                        child_jump_index: jump_to
                    },
                    MajorNode::LeafJump{child_jump_index, ..} => {
                        if cursor_at_next_major_node {
                            parent_jump_mut.next_major_node
                        } else {
                            MajorNode::Jump{child_jump_index}
                        }
                    }
                };
            }
            parent_jump_mut.next_major_node_dist = (!jump_inserted) as usize + (cursor.node_index - parent_jump_mut.jump_to_node) as usize;
        }

        if let Some(range) = children_rotate_range {
            let jump = self.jumps[range.start];
            match self.jumps[jump.parent_jump_index as usize].next_major_node {
                MajorNode::LeafJump{ref mut child_jump_index, ..} |
                MajorNode::Jump{ref mut child_jump_index} => *child_jump_index += (children_rotate_to - range.start) - range.len(),
                MajorNode::Leaf{..} => panic!("tree corrupted")
            }

            self.jumps[range.start..children_rotate_to].rotate(range.len());
        }

        self.verify_tree_integrity();

        RawCursor {
            node_index: (insert_node_index + num_nodes_insert - 1) as isize,
            parent_jump_index: match self.jumps[cursor.parent_jump_index].next_major_node {
                MajorNode::Leaf{..} => cursor.parent_jump_index,
                MajorNode::LeafJump{child_jump_index, ..} |
                MajorNode::Jump{child_jump_index} => {
                    if insert_split_jump {
                        insert_split_jump_index
                    } else {
                        child_jump_index
                    }
                }
            },
            depth: cursor.depth + num_nodes_insert as isize
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

            let parent_jump = self.jumps[cursor.parent_jump_index];
            let first_child_jump_index = match parent_jump.next_major_node {
                MajorNode::LeafJump{child_jump_index, ..} |
                MajorNode::Jump{child_jump_index} => child_jump_index,
                MajorNode::Leaf{..} => usize::max_value()
            };

            let parent_leaf_index: isize;

            {
                let parent_jump_mut = &mut self.jumps[cursor.parent_jump_index];
                parent_leaf_index = match parent_jump_mut.next_major_node {
                    MajorNode::Leaf{leaf_index} => leaf_index,
                    MajorNode::LeafJump{leaf_index, ..} => leaf_index as isize,
                    MajorNode::Jump{..} => -1
                };

                parent_jump_mut.next_major_node = MajorNode::Leaf{ leaf_index: -1 };
                parent_jump_mut.next_major_node_dist = ((cursor.node_index - 1) - parent_jump.jump_to_node) as usize;
            }


            self.nodes.splice(cursor.node_index as usize..last_child_node.node_index as usize + 1, None);

            let nodes_removed = (last_child_node.node_index + 1) - cursor.node_index;
            let remove_parent_jump_index = match cursor.node_index == parent_jump.jump_to_node {
                true => cursor.parent_jump_index as isize,
                false => isize::max_value()
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
                        (remove_parent_jump_index < jump_index) as usize +
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
                            if parent_leaf_index != -1 && parent_leaf_index < *leaf_index {
                                *leaf_index -= 1;
                            }
                            if retain_index == cursor.parent_jump_index {
                                leaf_remove_range.start = *leaf_index as usize;
                                leaf_remove_range.end = leaf_remove_range.start;
                            } else if retain_index < first_child_jump_index {
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
                        MajorNode::LeafJump{ref mut child_jump_index, ref mut leaf_index} => {
                            *child_jump_index -= jump_offset(*child_jump_index as isize);
                            if parent_leaf_index != -1 && (parent_leaf_index as usize) < *leaf_index {
                                *leaf_index -= 1;
                            }
                            if retain_index + 1 == cursor.parent_jump_index {
                                leaf_remove_range.start = *leaf_index;
                                leaf_remove_range.end = leaf_remove_range.start;
                            } else if retain_index < first_child_jump_index {
                                // This branch is before the child jump range
                                leaf_remove_range.start = *leaf_index + 1;
                                leaf_remove_range.end = leaf_remove_range.start;
                            } else if jump.jump_to_node <= last_child_node.node_index {
                                // This branch is during the child jump range
                                leaf_remove_range.end = *leaf_index + 1;
                            } else {
                                // This branch is after the child jump range
                                *leaf_index -= leaf_remove_range.len();
                            }
                        }
                        MajorNode::Leaf{..} => ()
                    }
                }

                if in_child_jump_range {
                    child_jump_count += 1
                }
                retain_index += 1;

                retain
            });
            self.leaves.splice(leaf_remove_range, None);
            if parent_leaf_index != -1 {
                self.leaves.remove(parent_leaf_index as usize);
            }
            self.verify_tree_integrity();
        }
    }

    fn verify_tree_integrity(&self) {
        #[cfg(debug_assertions)]
        {
            // Sanity check to see if the jump list is sorted.
            assert!(self.jumps.windows(2).all(|x| x[0] < x[1]));
            let mut leaves_used = vec![];
            for (i, jump) in self.jumps.iter().enumerate() {
                match jump.next_major_node {
                    MajorNode::LeafJump{child_jump_index, ..} |
                    MajorNode::Jump{child_jump_index} => assert_eq!(i, self.jumps[child_jump_index].parent_jump_index as usize),
                    _ => ()
                }

                match jump.next_major_node {
                    MajorNode::LeafJump{leaf_index, ..} => leaves_used.push(leaf_index),
                    MajorNode::Leaf{leaf_index} if leaf_index != -1 => leaves_used.push(leaf_index as usize),
                    _ => ()
                }
            }

            assert!(leaves_used.windows(2).all(|x| x[0] < x[1]));
        }
    }
}

impl Jump {
    #[inline]
    fn default_root() -> Jump {
        Jump {
            depth: -1,
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
            parent_jump_index: 0,
            depth: -1
        }
    }

    pub fn depth(self) -> isize {
        self.depth
    }
}

impl MajorNode {
    #[inline]
    fn is_jump(self) -> bool {
        match self {
            MajorNode::LeafJump{..} |
            MajorNode::Jump{..} => true,
            MajorNode::Leaf{..} => false
        }
    }
}
