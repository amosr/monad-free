use std::ops::Range;
use std::rc::Rc;

use crate::random::Random;

#[derive(Clone)]
pub struct Gen<'a, A> {
    run : Rc<dyn Fn(Random, Shrink) -> A + 'a>
}

impl<'a, A> Gen<'a, A> {
    pub fn new<F>(f : F) -> Gen<'a, A>
    where F : Fn(Random, Shrink) -> A + 'a {
        Gen { run : Rc::new(f) }
    }

    pub fn of<F>(f : F) -> Gen<'a, A>
    where F : Fn(&mut Extract) -> A + 'a,
    A : 'a {
        // assume that all invocations of 'f' make the same number of calls to Extract::of
        // in the same or similar circumstances -- ie 'f' is mostly data-independent control flow.
        let count = {
            let mut x = Extract::new(
                Random::new_from_seed(0),
                Shrink {
                    size: 0,
                    shrinks: 0,
                },
                0
            );
            f(&mut x);
            x.child_shrinks.len()
        };

        Gen::new(move |r, s| {
            let mut x = Extract::new(r, s, count);
            f(&mut x)
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Shrink {
    size : usize,
    shrinks : usize,
}

pub struct Extract {
    /// Random generator with seed
    rand : Random,
    /// Default shrink for children
    extract_shrink : Shrink,
    /// Specific shrink for children
    child_shrinks : Vec<Shrink>,
    /// Next unused index
    index : usize,
}

impl Extract {
    fn new(rand : Random, shrink : Shrink, expected_extraction_count : usize) -> Extract {
        Extract {
            rand : rand,
            extract_shrink : shrink,
            child_shrinks : Self::shrink_vec(shrink, expected_extraction_count),
            index : 0,
        }
    }

    fn shrink_vec(base : Shrink, count : usize) -> Vec<Shrink> {
        let size = base.size;
        let shrinks = base.shrinks;
        let divv = shrinks / count;
        let modd = shrinks % count;
        let mut v = Vec::with_capacity(count);
        for i in 0..count {
            let s = divv + if i == modd {
                1
            } else {
                0
            };
            v.push(Shrink { size: size, shrinks: s});
        }
        v
    }


    pub fn of<A>(&mut self, gen : Gen<A>) -> A {
        assert!(self.index < self.child_shrinks.len());

        let ix = self.index;
        let shrink = if self.child_shrinks.len() < ix {
            self.child_shrinks[ix]
        } else {
            self.child_shrinks.push(self.extract_shrink);
            self.extract_shrink
        };

        assert!(self.index <= self.child_shrinks.len());

        self.index += 1;

        let child_rand = self.rand.split();

        (*gen.run)(child_rand, shrink)
    }
}





impl<'a> Gen<'a, u64> {
    pub fn usize_range(range : Range<usize>) -> Gen<'a, usize> {
        Gen::new(move |mut r, _s| {
            r.u64_range(range.start as u64 .. range.end as u64) as usize
        })
    }
}

impl<'a, A> Gen<'a, A> {
    pub fn choose(v : Vec<A>) -> Gen<'a, A>
    where A : 'a + Clone {
        Gen::of(move |x| {
            let ix = x.of(Gen::usize_range(0..v.len()));
            v[ix].clone()
        })
    }
}

