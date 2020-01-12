# monad-free shrinking

This repo contains some rough experiments with implementing a Hedgehog-style testing library in Rust.

I wanted to be able to write Hedgehog-style generators, but didn't want to have to desugar the monadic binds by hand. The idea here is to try to use the borrow stuff to allow the same sort of structure as monads, but hopefully with a less unwieldy syntax. It's not terribly surprising that we can do this: linear types can enforce similar safety properties as monads, so there should be some connection with the borrow checking stuff.

Roughly, instead of writing this:
```
fn gen_date() -> Gen<Date> {
  Gen::u64(0..3000).and_then(|year| {
    Gen::u64(0..12).and_then(|month| {
      Gen::u64(0..32).and_then(|day| {
        Gen::unit(Date { year, month, day })
      })
    })
  })
}
```

I want a way to combine multiple generators together at a time, which looks like this:

```
fn gen_date() -> Gen<Date> {
    Gen::combine(|c| {
        Date {
            year:  c.of(Gen::u64(0..3000)),
            month: c.of(Gen::u64(0..12)),
            day:   c.of(Gen::u64(0..32)),
        }
    })
}
```


Anyway, so monads have `return` and `bind`:

```
return :: a -> m a

bind :: (a -> m b) -> m a -> m b
```

Intuitively, I think of return as wrapping up the value `a` in some computational context (the monad `m a`), and I think of bind as temporarily allowing you to unwrap a value `a` from inside the context `m a` --- but only if you promise to wrap it back up at the end.

The key idea here is that the bind temporarily gives you permission to unwrap values from inside the monad. Instead of translating these operations in Rust as-is, we can make this idea of temporary permission explicit as a *capability* or *witness*. This witness is a value that lets you unwrap values from the computational context, but you can only get a witness if you promise to wrap everything up again at the end.

So to do something similar to a non-deterministic monad (like list in Haskell) we'd have two types:

```
type NonDet<A> = ...;
type Witness = ...;
```

Importantly, the `Witness` must be an opaque type and it cannot implement the `Copy` or `Clone` traits. The intuition here is that if I give you permission to tousle my hair, you cannot share that permission with your friends.

We still have two operations for managing the wrapping. First, if you have a `Witness`, you can unwrap the value from a `NonDet<A>`:

```
fn of<A>(&mut Witness, computation : NonDet<A>) -> A
```

This operation takes a mutable reference to the witness and the wrapped computation and retrieves the value from inside. I don't know if there is a standard name for this operation - `of` is nice and short, but other candidates are get, unwrap, extract.

For the second operation, you can get a temporary witness whenever you want, and the result will be wrapped up:

```
fn combine<F>(f : F) -> NonDet<A>
where
  F : Fn(&mut Witness) -> A
```

The corresponding type in Haskell would be something like `combine :: (MutRef Witness -> a) -> NonDet a`. The user function is given a mutable reference to a witness, but the witness is owned by `combine`. That means that the user function can use the witness to unwrap values with the `of` function, but the user function cannot save the witness for later by storing it in a mutable variable or hiding it in some other structure. This ensures that only calls to `combine` can unwrap the values, and that the result will always be wrapped back up at the end.

To implement these operations, the witness has to contain a lot of the monadic state. This is why it is a mutable reference. For a state computation like `State<S, A>` (state monad in Haskell) the witness would contain the mutable state `S`. For non-deterministic computations it includes information such as how many choices are available for each sub-computation and which choice to use.

## issues and assumptions

For the generator one, and for the non-deterministic one, the implementation of the `combine` operation calls the closure a whole bunch of times. Here, the witness is a data structure describing how to shrink each generator, or which value to extract from the list of choices in the non-deterministic computation. This assumes that the closure is more or less pure, so that if the generator returns the same values then the closure will make the same calls to the `of` operation.

This will also perform more repeated computation than a monadic encoding would. I'm not sure whether there's a way around that except by splitting them into smaller subcomputations (ie more calls to `combine`). I suspect / hope that this won't be an issue for a testing library, because it doesn't have to be too fast --- just fast enough to not be annoying. The repeated computation is also only necessary for shrinking failed tests, not in the ordinary case of passing tests.

I am not sure whether it's possible to implement continuation-style monads such as exception handling with this. The "trick" here is really just to use Rust's ambient mutable state in a limited way, but unfortunately I don't think there are ambient continuations or exceptions to use. For cases like non-determinism and Hedgehog-style generators, we can sort of fake it by calling the computation lots of times. But we can't, say, abort the computation half-way through and return to the `combine` function, as would be required for an exception handler.

## implementation

Simpler examples are in [src/state.rs] and [src/nondet.rs] for state and non-deterministic computations respectively.

See [src/hh2_tree.rs] for an example of a Hedgehog-style tree-based generator. This uses a strict tree, so it eagerly constructs the whole shrink tree. That ends up being pretty bad and uses lots of memory.

The file [src/hh3_lazy_tree.rs] is not so eager and delays computation of the children of the rose tree at each level. This is much better, but all the extra closures somewhat obscure the main thrust. (Contrary to its name it is not actually lazy, as the children must be recomputed every time they are required.)

