use reclaim::{Reclaim, U0};

type Atomic<T, R> = reclaim::Atomic<T, U0, R>;
type Owned<T, R> = reclaim::Owned<T, U0, R>;

pub struct TreiberStack<T, R: Reclaim> {
    head: Atomic<T, R>,
}

impl<T, R: Reclaim> TreiberStack<T, R> {
    pub fn new() -> Self {
        Self { head: Atomic::null() }
    }

    pub fn push(elem: T) {
        let owned = Owned::<_, R>::new(Node::<_, R> {
            elem,
            next: Atomic::null(),
        });

        //let guarded = R::guarded::<Node<T, R>, U0>();
    }
}

pub struct Node<T, R: Reclaim> {
    pub elem: T,
    pub next: Atomic<T, R>,
}

fn main() {}