extern crate flat_trie;
use flat_trie::*;

fn main() {
    let mut tree: FlatTrie<_, i32> = FlatTrie::new();
    {
        let mut cursor = tree.cursor_mut();
        cursor.child("a").or_insert(None).enter()
            .child("a.a").or_insert(Some(32)).enter()
            .child("a.a.a").or_insert(Some(48)).enter()
                .child("a.a.a.a").or_insert(None).cont().parent().enter()
                .child("a.a.b").or_insert(Some(83)).cont().parent().enter().parent().enter()
            .child("b").or_insert(Some(64)).cont();

        cursor.child("a").unwrap_occupied().enter().set_leaf(1);
        // println!("{:#?}", cursor.tree);
        // cursor.child("b").unwrap_occupied().prune();
        // println!("{:#?}", cursor.tree);
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
