use std::ops::Range;
use std::rc::Rc;

use crate::random::Random;


/// A strict rose tree for holding a generate value and its possible shrinks.
/// The children should probably be lazy
#[derive(Clone)]
pub struct Tree<'a, A> {
    pub value : A,
    pub children : Rc<dyn Fn() -> Vec<Tree<'a, A>> + 'a>,
}

impl<'a, A> Tree<'a, A> {
    /// Try to look up a given path to a child subtree.
    /// If at any point the path leads to a child that does not exist, return instead the deepest tree
    /// in the path that does exist.
    pub fn get_path_or_closest(&self, path : &TreePath) -> Tree<'a, A>
    where A : Clone {
        let mut here = Tree::clone(self);
        for &ix in &path.indices {
            let children = (*here.children)();
            if ix < children.len() {
                here = Tree::clone(&children[ix]);
            } else {
                // Out of bounds - give up and return the last leaf we got to
                return here;
            }
        }
        here
    }
}

/// Path to a value in a rose tree - each element of indices is the index of a child node
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TreePath {
    indices : Vec<usize>
}

impl TreePath {
    pub fn empty() -> TreePath {
        TreePath { indices : Vec::new() }
    }
}

/// Generator is a function from RNG and gen size to a tree
#[derive(Clone)]
pub struct Gen<'a, A> {
    pub run : Rc<dyn Fn(Random, usize) -> Tree<'a, A> + 'a>
}

impl<'a, A> Gen<'a, A> {
    /// Helper for constructing Gen<A> from closure
    pub fn new<F>(f : F) -> Gen<'a, A>
    where F : Fn(Random, usize) -> Tree<'a, A> + 'a {
        Gen { run : Rc::new(f) }
    }

    /// Joining together generators, comparable to the monad and applicative instances.
    /// Monads are about making sure values that depend on effects are encapsulated and don't escape.
    /// Linear types can achieve similar things without monads, so borrowing should be able to do the same thing.
    /// 
    /// As an example, here is how you might generate a date struct with separate year, month and day:
    /// > Gen::combine(|c| {
    /// >  Date {
    /// >    year:  c.of(Gen::u64(0..2021)),
    /// >    month: c.of(Gen::u64(0..12)),
    /// >    day:   c.of(Gen::u64(0..32)),
    /// >  }
    /// > })
    /// 
    /// The combine function takes a closure of type "&mut Chooser -> A", and wraps the result in a Gen<A>.
    /// This "Chooser" represents the ability to extract a value from a Gen, using the Chooser method
    /// > Chooser::of<A>(&mut self, gen : Gen<A>) -> A
    /// Having a Chooser is a bit like being inside a monadic bind, in that you can use the chooser
    /// to extract a value from a Gen<A> and then manipulate it.
    /// The main complication is that a Chooser can only be used within the context of this 'combine' function,
    /// because otherwise one could arbitrarily pull values out of a Gen.
    /// The trick here is that the borrow checker ensures that the mutable reference to the Chooser
    /// cannot escape the closure: it cannot be stored in a larger structure or returned or even used
    /// in a nested call to combine, because these cases might allow the Chooser to be used after
    /// combine has dropped it.
    ///
    /// The closure to combine needs to implement Clone --- presumably this means all values in its
    /// closure environment need to also implement Clone. This is required because the closure is
    /// stored in the lazy children of the tree.
    pub fn combine<F>(f : F) -> Gen<'a, A>
    where F : Fn(&mut Chooser) -> A + 'a + Clone,
    A : 'a {
        Gen::new(move |r, s| {
            Self::combine_go(f.clone(), r, s, Vec::new())
        })
    }

    /// Worker function for combine, recursively generates the shrink tree
    fn combine_go<F>(f : F, r : Random, s : usize, mut paths : Vec<TreePath>) -> Tree<'a, A>
    where F : Fn(&mut Chooser) -> A + 'a + Clone,
    A : 'a {
        // println!("Gen::combine_go {:#?}", paths);
        // Run with given shrink paths to get result value & check how many further shrinks are possible
        let mut c = Chooser::new(r, s, paths.clone());
        let value = f(&mut c);

        // Make sure that the paths vector contains an empty shrink path for each generator
        let gen_count = c.gen_child_count.len();
        while paths.len() < gen_count {
            paths.push(TreePath::empty());
        }

        let children_clo = move || {
            let mut children : Vec<Tree<A>> = Vec::new();

            // Loop through all the generators that the closure used.
            // child_count denotes how many options this generator has for shrinking.
            for (gen_ix, &child_count) in c.gen_child_count.iter().enumerate() {
                // println!("Gen::combine_go.gen_ix: {}", gen_ix);
                // Loop over the shrink options for this generator
                for child_ix in 0 .. child_count {
                    // println!("Gen::combine_go.child_ix: {}/{}", child_ix, child_count);
                    // Add this shrink option to the path and compute it
                    // This does a bunch more clones than really necessary, but whatever
                    let mut paths_copy = paths.clone();
                    paths_copy[gen_ix].indices.push(child_ix);
                    children.push(Self::combine_go(f.clone(), r, s, paths_copy));
                }
            }

            children
        };

        Tree {
            value, children : Rc::new(children_clo)
        }
    }

}

/// At an abstract level, Chooser is a capability or evidence that you're allowed to
/// get the values out of a Gen.
/// In terms of implementation, a particular Chooser describes how much to shrink each generator
/// and records how each generator can be shrunk further.
pub struct Chooser {
    /// State: random generator with seed
    rand : Random,
    /// Input argument to Gen: generator size
    // XXX: is there any way to specify const/immutable fields in Rust?
    size : usize,
    /// Input: path describing how to shrink each generator
    gen_paths : Vec<TreePath>,
    /// State: how many children (ie potential shrinks) for each generator we've seen so far
    gen_child_count : Vec<usize>,
}

impl Chooser {
    fn new(rand : Random, size : usize, gen_paths: Vec<TreePath>) -> Chooser {
        Chooser {
            rand, size, gen_paths,
            gen_child_count: Vec::new()
        }
    }

    pub fn of<A>(&mut self, gen : Gen<A>) -> A
    where A : Clone {
        // println!("Chooser::of");
        let child_rand = self.rand.split();
        let tree = (*gen.run)(child_rand, self.size);

        let ix = self.gen_child_count.len();
        let shrunk = match self.gen_paths.get(ix) {
            None => tree,
            Some(p) => tree.get_path_or_closest(p)
        };

        // XXX: maybe could put length as strict field in Tree to avoid forcing here, probably not a big deal
        let children = (*shrunk.children)();

        self.gen_child_count.push(children.len());

        shrunk.value.clone()
    }
}





impl<'a> Gen<'a, u64> {
    pub fn u64(range : Range<u64>) -> Gen<'a, u64> {
        // println!("Gen::u64 {:#?}", range);
        // XXX: Range is not Copy (for reasons), so need to clone it.
        // Maybe shrink_u64 should be by-ref.
        // Probably want a different Range type with more information anyway (eg midpoint/shrink-to)
        Gen::new(move |mut r, _s| {
            // println!("Gen::u64.new {:#?}", range);
            let value = r.u64_range(range.clone());
            Self::shrink_u64(range.clone(), value)
        })
    }

    fn shrink_u64(range : Range<u64>, value : u64) -> Tree<'a, u64> {
        let v0 = value - range.start;
        // println!("Gen::shrink_u64 {:#?} {} diff {}", range, value, v0);
        let children = move || {
            if v0 > 4 {
                vec![
                    Self::shrink_u64(range.clone(), range.start + v0 / 2),
                    Self::shrink_u64(range.clone(), value - 1)
                ]
            } else if v0 > 0 {
                vec![Self::shrink_u64(range.clone(), value - 1)]
            } else {
                vec![]
            }
        };
        Tree { value, children: Rc::new(children) }
    }

    pub fn usize(range : Range<usize>) -> Gen<'a, usize> {
        // println!("Gen::usize {:#?}", range);
        Gen::combine(move |c| {
            // println!("Gen::usize.combine {:#?}", range);
            c.of(Gen::u64(range.start as u64 .. range.end as u64)) as usize
        })
    }
}

impl<'a, A> Gen<'a, A> {
    pub fn choose(v : Vec<A>) -> Gen<'a, A>
    where A : 'a + Clone {
        Gen::combine(move |c| {
            let ix = c.of(Gen::usize(0..v.len()));
            v[ix].clone()
        })
    }

    pub fn vec(self, gen_len : Gen<'a, usize>) -> Gen<'a, Vec<A>>
    where A : 'a + Clone {
        Gen::combine(move |c| {
            let len = c.of(gen_len.clone());
            let mut vec = Vec::new();
            for _ in 0..len {
                vec.push(c.of(self.clone()));
            }
            vec
        })
    }
}

