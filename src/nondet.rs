use std::rc::Rc;

use crate::nonempty::NonEmpty;

/// Let's implement a non-deterministic computation.
/// The closure returns a non-empty vector that describes all the possible options the computation
/// can evaluate to.
pub struct NonDet<'a, A> {
    pub run : Rc<dyn Fn() -> NonEmpty<A> + 'a>
}

/// The witness that lets you extract values from a non-deterministic computation is a mutable
/// structure describing which choice to take for each generator (ector of indices), and how many
/// choices each generator has.
pub struct Witness {
    /// Which choice to make whenever we want to make a choice.
    /// This is like a (finite representation of) an infinite sequence of dice rolls that we will
    /// make in the future.
    /// Once you run past the end of this vector, we assume that the remaining indices are all zeroes.
    indices : Vec<usize>,
    /// Whenever we are asked to make a choice, record how many total choices there were.
    /// This is used to explore the number of choices.
    num_choices : Vec<usize>
}

/// Non-deterministic computations
impl<'a, A> NonDet<'a, A> {

    /// Create a non-deterministic computation from a function that returns a non-empty vector of
    /// choices.
    pub fn new<F>(f : F) -> NonDet<'a, A>
    where F : Fn() -> NonEmpty<A> + 'a,
          A : 'a {
        NonDet {
            run : Rc::new(f)
        }
    }

    /// Combine together multiple non-deterministic computations.
    pub fn combine<F>(f : F) -> NonDet<'a, A>
    where F : Fn(&mut Witness) -> A + 'a,
          A : 'a {
        NonDet::new(move || {
            // Run computation with all the choices as 0 indices, ie the first choice.
            // This gives us the initial value, as well as telling us how many other choices there
            // are.
            let mut w = Witness { indices : Vec::new(), num_choices : Vec::new() };
            let initial_value = f(&mut w);

            // Mutable vector to keep track of all the results
            let mut results = NonEmpty { zero : initial_value, vec : Vec::new() };

            // Loop over all the choices and run each one
            while let Some(next_choice) = Self::incr_choice_indices(&w.indices, &w.num_choices) {
                // Run with next seq of indices and empty num_choices
                // (consumer pushes onto end of num_choices to populate)
                w.indices = next_choice;
                w.num_choices = Vec::new();
                let value = f(&mut w);

                // Record result
                results.vec.push(value);
            }

            results
        })
    }

    /// Lexicographic ordering on indices. num_choices describes the exclusive range for each element.
    /// > incr_choice_indices([0, 0, 0], [3, 2, 1]) =...
    /// >   Some [0, 1, 0]
    /// >   Some [1, 0, 0]
    /// >   Some [1, 1, 0]
    /// >   Some [2, 0, 0]
    /// >   Some [2, 1, 0]
    /// >   None
    fn incr_choice_indices(indices : &Vec<usize>, num_choices : &Vec<usize>) -> Option<Vec<usize>> {
        // Copy and ensure length is same as choices, padding with zeroes as necessary
        let mut res = indices.clone();
        res.resize(num_choices.len(), 0);

        // Loop from the end of the vector, incrementing each index until the first that doesn't overflow
        for ix in (0..num_choices.len()).rev() {
            res[ix] += 1;
            if res[ix] < num_choices[ix] {
                // No overflow -> done
                return Some(res);
            } else {
                // Overflow -> set to zero and continue to previous digit
                res[ix] = 0;
            }
        }
        // Loop is over and all of them overflowed -> we must have exhausted the choices
        None
    }
}

impl Witness {
    /// Extract a value from a wrapped up non-deterministic computation
    pub fn of<'a, A>(&mut self, m : NonDet<'a, A>) -> A
    where A : Clone {

        // Run the computation to get the vector of choices
        let choices = (*m.run)();
        // m_ix tells us how many previous nested computations we have run.
        // This is used to know which computation this is, and therefore which choice we should use
        let m_ix = self.num_choices.len();
        let choice_ix = match self.indices.get(m_ix) {
            None => 0,
            Some(&i) => i
        };

        // Record the number of other choices this nested computation has
        self.num_choices.push(choices.len());

        // XXX: should be able to get rid of clone here?
        choices.index(choice_ix).clone()
    }
}


#[cfg(test)]
mod test {
    use crate::nonempty::NonEmpty;
    use crate::nondet::*;

    fn nondet<'a, A : Clone + 'a>(zero : A, vec : Vec<A>) -> NonDet<'a, A> {
        let nonempty = NonEmpty { zero, vec };
        NonDet::new(move || {
            nonempty.clone()
        })
    }

    #[test]
    fn ok() {
        let numbers = NonDet::combine(|c| {
            let u100 = nondet(0, vec![1, 2, 3]);
            let u10  = nondet(0, vec![1, 2]);
            let u1   = nondet(0, vec![1]);

            c.of(u100) * 100 +
                c.of(u10) * 10 +
                c.of(u1) * 1
        });

        let result = (*numbers.run)();
        assert_eq!(result.to_vec(),
            vec![
                000, 001, 010, 011, 020, 021,
                100, 101, 110, 111, 120, 121,
                200, 201, 210, 211, 220, 221,
                300, 301, 310, 311, 320, 321
            ]);
    }
}
