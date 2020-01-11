use std::ops::Range;

use oorandom::Rand64;


#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Random {
    rand : Rand64
}

impl Random {
    pub fn new(rand : Rand64) -> Random {
        Random { rand }
    }

    pub fn new_from_seed(seed : u128) -> Random {
        Random::new(Rand64::new(seed))
    }

    pub fn u64_range(&mut self, range : Range<u64>) -> u64 {
        self.rand.rand_range(range)
    }

    /// Split generator in two. The returned generator will have a different seed than the updated self.
    /// Mutates self, so that repeated splits have different seeds:
    /// > let mut r1 = Random::new(<seed>);
    /// > let mut child1 = r1.split();
    /// > let mut child2 = r1.split();
    /// > assert!(r1.rand != child1.rand != child2.rand);
    /// 
    /// This implementation is probably not statistically good.
    pub fn split(&mut self) -> Random {
        // get current state.
        let (state, inc) = self.rand.state();
        // generate amount to perturb child state - this also modifies current state so that
        // consecutive splits have different seeds.
        let growth = self.rand.rand_u64() as u128;
        // create child with perturbed state and increment.
        // increment needs to be an odd number, so add an even amount.
        let rand = Rand64::from_state((state + growth, inc + 2));
        Self::new(rand)
    }
}

