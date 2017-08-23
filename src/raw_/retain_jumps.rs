use std::ops::RangeFrom
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
    // };

pub struct RetainExternal<'a, T: 'a> {
    source_vec: &'a mut Vec<T>,
    end: usize,
    del: usize,
    index: usize
}

pub struct RetainElement<'a: 'b, 'b, T: 'a + 'b>(&'b mut RetainExternal<'a, T>);

impl<'a, T> RetainExternal<'a, T> {
    #[inline]
    pub fn new(source_vec: &mut Vec<T>, range: RangeFrom<usize>) -> RetainExternal<T> {
        assert!(range.start <= source_vec.len());

        RetainExternal {
            end: source_vec.len(),
            del: 0,
            index: range.start,
            source_vec
        }
    }

    #[inline]
    pub fn next<'b>(&'b mut self) -> Option<RetainElement<'a, 'b, T>> {
        if self.index < self.end {
            Some(RetainElement(self))
        } else {
            None
        }
    }
}

impl<'a, 'b, T> RetainElement<'a, 'b, T> {
    #[inline]
    pub fn element(&self) -> &T {
        &self.0.source_vec[self.0.index]
    }

    // Dropping this struct retains the element, so this function doesn't have
    // to do anything.
    #[inline]
    pub fn retain(self) {}

    #[inline]
    pub fn discard(self) {
        use std::mem;

        self.0.del += 1;
        self.0.index += 1;
        // Don't run the retain destructor
        mem::forget(self);
    }
}

impl<'a, T> Drop for RetainExternal<'a, T> {
    fn drop(&mut self) {
        if self.del > 0 {
            self.source_vec.splice(self.index - self.del..self.index, None);
        }
    }
}

impl<'a, 'b, T> Drop for RetainElement<'a, 'b, T> {
    /// Dropping this struct retains the element by default.
    fn drop(&mut self) {
        self.0.source_vec.swap(self.0.index - self.0.del, self.0.index);
        self.0.index += 1;
    }
}
