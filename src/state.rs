use std::rc::Rc;

/// Wrapping up a "State" computation.
/// This is very simple, but also a bit silly, since we can just use the ambient state to implement
/// the computation.
pub struct State<'a, S, A> {
    pub run : Rc<dyn Fn(S) -> (S, A) + 'a>
}

/// A witness that says you're allowed to extract values from state computations.
/// In this case, the witness is just a mutable reference to the current state.
/// It is important that values of this type cannot be constructed outside of this module --- only
/// the State::combine function is so blessed.
pub struct Witness<S> {
    /// Mutable state
    state : S
}

impl<'a, S, A> State<'a, S, A> {
    /// Creating new State computations from closures
    pub fn new<F>(f : F) -> State<'a, S, A>
    where F : Fn(S) -> (S, A) + 'a,
          A : 'a {
        State {
            run : Rc::new(f)
        }
    }

    /// Combining State computations: closure takes a witness (ie the state) and can use this to
    /// run other State computations.
    pub fn combine<F>(f : F) -> State<'a, S, A>
    where F : Fn(&mut Witness<S>) -> A + 'a,
          A : 'a {
        State::new(move |s| {
            let mut w = Witness { state : s };
            let result = f(&mut w);
            (w.state, result)
        })
    }
}

impl<S> Witness<S> {
    /// Extract the value from a State computation if you have a witness.
    /// This requires the state type S to implement Clone --- not sure if this can be removed
    /// somehow.
    pub fn of<'a, A>(&mut self, m : State<'a, S, A>) -> A
    where S : Clone {
        let state = self.state.clone();
        let (new_state, result) = (*m.run)(state);
        self.state = new_state;
        result
    }
}

