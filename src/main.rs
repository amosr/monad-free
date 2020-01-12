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

    fn gen_many<'a>() -> Gen<'a, Vec<Date>> {
        Date::gen().vec(Gen::usize(0..20))
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

use std::time::Instant;

fn time_force_to_depth<'a, A : Debug>(tree : &Tree<'a, A>, max_depth : usize) -> usize {
    let instant = Instant::now();
    let count = force_to_depth_go(tree, max_depth, 0);
    println!("Forced {} nodes, took {:?}", count, instant.elapsed());
    count
}

fn force_to_depth_go<'a, A : Debug>(tree : &Tree<'a, A>, max_depth : usize, current_depth : usize) -> usize {
    let mut count = 1;
    let children = (*tree.children)();

    if current_depth < max_depth {
        for c in &children {
            count += force_to_depth_go(c, max_depth, current_depth + 1);
        }
    }

    count
}

fn main() -> Result<(), Box<dyn Error>> {
    let rand = Random::new_from_seed(1);
    println!("Random: {:?}", rand);
    let size = 0;
    let tree = (*Date::gen_many().run)(rand, size);

    print_to_depth(&tree, 1);

    println!("Timing shrinking ie failing case");
    time_force_to_depth(&tree, 2);

    println!("Timing non-shrink ie passing case");
    let instant = Instant::now();
    let count = 1000;
    for i in 0..count {
        let rand = Random::new_from_seed(i);
        let _tree = (*Date::gen_many().run)(rand, size);
        // println!("Generator test {} get value {:?}", i, _tree.value);
    }
    println!("Generated {} values, took {:?}", count, instant.elapsed());

    Ok(())
}

