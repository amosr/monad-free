use std::error::Error;
use std::fmt::Debug;

pub mod random;
pub mod hh1_no_tree;
pub mod hh2_tree;
pub mod hh3_lazy_tree;

pub mod state;
pub mod nondet;
pub mod nonempty;

use crate::random::Random;
use hh3_lazy_tree::*;

#[derive(Debug, Copy, Clone)]
struct Date {
    year : u64, month : u64, day : u64
}

impl Date {
    fn gen<'a>() -> Gen<'a, Date> {
        Gen::combine(|c| {
            Date {
                year:  c.of(Gen::u64(0..3000)),
                month: c.of(Gen::u64(0..12)),
                day:   c.of(Gen::u64(0..32)),
            }
        })
    }
}

fn print_to_depth<'a, A : Debug>(tree : &Tree<'a, A>, max_depth : usize) -> () {
    print_to_depth_go(tree, max_depth, 0)
}

fn print_to_depth_go<'a, A : Debug>(tree : &Tree<'a, A>, max_depth : usize, current_depth : usize) -> () {
    let indent = "  ".repeat(current_depth);
    println!("{}{:?}", indent, tree.value);
    let children = (*tree.children)();

    if current_depth < max_depth {
        for c in &children {
            print_to_depth_go(c, max_depth, current_depth + 1);
        }
    } else {
        println!("{}...{} shrinks not shown...", indent, children.len());
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let rand = Random::new_from_seed(1);
    println!("Random: {:?}", rand);
    let size = 0;
    let tree = (*Date::gen().run)(rand, size);

    print_to_depth(&tree, 2);

    Ok(())
}

