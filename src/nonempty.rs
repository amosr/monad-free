
/// Non-empty vectors have at least one element
#[derive(Debug, Clone)]
pub struct NonEmpty<A> {
    pub zero : A,
    pub vec  : Vec<A>
}

impl<A> NonEmpty<A> {
    pub fn index(&self, ix : usize) -> &A {
        if ix == 0 {
            &self.zero
        } else {
            &self.vec[ix - 1]
        }
    }

    pub fn len(&self) -> usize {
        self.vec.len() + 1
    }

    pub fn to_vec(self) -> Vec<A> {
        let mut vec = Vec::new();
        vec.push(self.zero);
        for v in self.vec {
            vec.push(v);
        }
        vec
    }
}


