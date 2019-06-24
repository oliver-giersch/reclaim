//! An implementation of Treiber's stack that is fully generic over the used
//! memory reclamation scheme.

use std::mem::ManuallyDrop;
use std::ptr;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};

use reclaim::prelude::*;
use reclaim::typenum::U0;

type Atomic<T, R> = reclaim::Atomic<T, R, U0>;
type Owned<T, R> = reclaim::Owned<T, R, U0>;

#[derive(Debug)]
pub struct Stack<T, R: GlobalReclaim> {
    head: Atomic<Node<T, R>, R>,
}

impl<T, R: GlobalReclaim> Stack<T, R> {
    #[inline]
    pub fn new() -> Self {
        Self { head: Atomic::null() }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.head.load_unprotected(Acquire).is_none()
    }

    #[inline]
    pub fn push(&self, elem: T) {
        let mut node = Owned::new(Node::new(elem));
        let mut guard = R::guard();

        loop {
            let head = self.head.load(Acquire, &mut guard);
            node.next.store(head, Relaxed);

            match self.head.compare_exchange_weak(head, node, Release, Relaxed) {
                Ok(_) => return,
                Err(fail) => node = fail.input,
            };
        }
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        let mut guard = R::guard();

        while let Some(head) = self.head.load(Relaxed, &mut guard) {
            let next = head.next.load_unprotected(Relaxed);
            if let Ok(unlinked) = self.head.compare_exchange_weak(head, next, Release, Relaxed) {
                unsafe {
                    // the `Drop` code for T is never called for retired nodes, so it is
                    // safe to use `retire_unchecked` and not require that `T: 'static`.
                    let elem = ptr::read(&*unlinked.elem);
                    unlinked.retire_unchecked();
                    return Some(elem);
                }
            }
        }

        None
    }
}

impl<T, R: GlobalReclaim> Drop for Stack<T, R> {
    #[inline]
    fn drop(&mut self) {
        let mut curr = self.head.take();
        while let Some(mut node) = curr {
            unsafe { ManuallyDrop::drop(&mut node.elem) };
            curr = node.next.take();
        }
    }
}

#[derive(Debug)]
struct Node<T, R: GlobalReclaim> {
    elem: ManuallyDrop<T>,
    next: Atomic<Node<T, R>, R>,
}

impl<T, R: GlobalReclaim> Node<T, R> {
    #[inline]
    fn new(elem: T) -> Self {
        Self { elem: ManuallyDrop::new(elem), next: Atomic::null() }
    }
}

fn main() {}
