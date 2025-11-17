use core::cell::Cell;
use core::pin::Pin;
use core::ptr::{null, null_mut, NonNull};
use core::marker::{PhantomData, PhantomPinned};

/// A queue of elements of type Elem
#[derive(Debug)]
pub struct Head<Elem> {
    queueFront: *const Elem,
    queueBack: *const Elem
}

unsafe impl<Elem: Send> Send for Head<Elem> {}

///  A link within a structure, allowing that structure to be
///  collected into a queue owned by Head<Elem>
#[derive(Debug)]
pub struct Link<Elem> {
    next: *const Elem,
    prev: *const Elem,
    inQueue: bool,
    phantomPinned: PhantomPinned
}


impl<Elem> Head<Elem> {
    /// Creates the head of a queue
    pub const fn new() -> Self {
        Head {
            queueFront: null_mut(),
            queueBack: null_mut()
        }
    }
}

impl<Elem> Link<Elem> {
    /// Creates a link
    pub const fn new() -> Self {
        Link {
            next: Cell::new(null_mut()),
            prev: Cell::new(null_mut()),
            inQueue: Cell::new(false),
            phantomPinned: PhantomPinned
        }
    }
}

impl<Elem> Head<Elem> {
    /// Returns a pointer to the first element in the queue,
    /// or None if the queue is empty.
    pub fn front(&self) -> Option<&Elem> {
        unsafe { self.queueFront.as_ref() }
    }

    pub fn front_ptr(&self) -> Option<*const Elem> {
        Some(self.queueFront).filter(|p| !p.is_null())
    }

    /// Returns a pointer to the last element in the queue,
    /// which is null if the queue is empty.
    pub fn tail(&self) -> Option<&Elem> {
        unsafe { self.queueBack.as_ref() }
    }

    pub fn tail_ptr(&self) -> Option<*const Elem> {
        Some(self.queueBack).filter(|p| !p.is_null())
    }
}

impl<Elem> Link<Elem> {
    /// Returns a pointer to the next element in the queue.
    ///
    /// If not in a queue or is the last element in the queue,
    /// returns None.
    pub fn next(&self) -> Option<&Elem> {
        unsafe { self.next.get().as_ref() }
    }

    pub fn next_ptr(&self) -> Option<*const Elem> {
        Some(self.next.get()).filter(|p| !p.is_null())
    }

    /// Returns a pointer to the previous element in the queue.
    ///
    /// If not in a queue or is the first element in the queue,
    /// returns None.
    pub fn prev(&self) -> Option<&Elem> {
        unsafe { self.prev.get().as_ref() }
    }

    pub fn prev_ptr(&self) -> Option<*const Elem> {
        Some(self.prev.get()).filter(|p| !p.is_null())
    }

    /// Checks whether an element is in a queue.
    /// This only checks whether a given link is being used
    pub fn in_queue(&self) -> bool {
        self.inQueue.get()
    }
}

impl<Elem> Head<Elem> {
    pub unsafe fn insert_front<'a, F>(&mut self, elem: Pin<&'a Elem>, link_name: F) -> &'a Elem
    where F: Fn(&Elem) -> &Link<Elem> {
        let elem = elem.get_ref();

        if !link_name(elem).in_queue() {
            link_name(elem).prev.set(null());
            link_name(elem).next.set(self.queueFront);

            match self.front() {
                None => {
                    self.queueFront = elem;
                    self.queueBack = elem;
                },
                Some(front) => {
                    link_name(front).prev.set(elem);
                    self.queueFront = elem;
                }
            }

            link_name(elem).inQueue.set(true);
        }

        elem
    }

    pub unsafe fn insert_tail<'a, F>(&mut self, elem: Pin<&'a Elem>, link_name: F) -> &'a Elem
    where F: Fn(&Elem) -> &Link<Elem> {
        let elem = elem.get_ref();

        if !link_name(elem).in_queue() {
            link_name(elem).next.set(null_mut());
            link_name(elem).prev.set(self.queueFront);

            match self.tail() {
                None => {
                    self.queueFront = elem;
                    self.queueBack = elem;
                },
                Some(tail) => {
                    link_name(tail).next.set(elem);
                    self.queueBack = elem;
                }
            }

            link_name(elem).inQueue.set(true);
        }

        elem
    }

    pub unsafe fn insert_after<'a, F>(&mut self, inQ: &Elem, toinsert: Pin<&'a Elem>, link_name: F) -> &'a Elem
    where F: Fn(&Elem) -> &Link<Elem> {
        let toinsert = toinsert.get_ref();

        if link_name(inQ).in_queue() && !link_name(toinsert).in_queue() {
            link_name(toinsert).prev.set(inQ);
            link_name(toinsert).next.set(link_name(inQ).next.get());

            match link_name(inQ).next() {
                None => {
                    link_name(inQ).next.set(toinsert);
                    self.queueBack = toinsert;
                },
                Some(next) => {
                    link_name(next).prev.set(toinsert);
                    link_name(inQ).next.set(toinsert);
                }
            }

            link_name(toinsert).inQueue.set(true);
        }

        toinsert
    }

    pub unsafe fn insert_before<'a, F>(&mut self, inQ: &Elem, toinsert: Pin<&'a Elem>, link_name: F) -> &'a Elem
    where F: Fn(&Elem) -> &Link<Elem> {
        let toinsert = toinsert.get_ref();

        if link_name(inQ).in_queue() && !link_name(toinsert).in_queue() {
            link_name(toinsert).prev.set(link_name(inQ).prev.get());
            link_name(toinsert).next.set(inQ);

            match link_name(inQ).prev() {
                None => {
                    link_name(inQ).prev.set(toinsert);
                    self.queueFront = toinsert;
                },
                Some(prev) => {
                    link_name(prev).next.set(toinsert);
                    link_name(inQ).prev.set(toinsert);
                }
            }

            link_name(toinsert).inQueue.set(true);
        }

        toinsert
    }

    pub fn remove<F>(&mut self, elem: &Elem, link_name: F)
    where F: Fn(&Elem) -> &Link<Elem> {
        if link_name(elem).in_queue() {
            link_name(elem).inQueue.set(false);

            // Redirect pointer from prev
            match link_name(elem).prev() {
                None => {
                    // If prev is null, you're at the front of the queue
                    self.queueFront = link_name(elem).next.get();
                },
                Some(prev) => {
                    link_name(prev).next.set(link_name(elem).next.get());
                }
            }

            // Redirect pointer from next
            match link_name(elem).next() {
                None => {
                    // If next is null, you're at the back of the queue
                    self.queueBack = link_name(elem).prev.get();
                },
                Some(next) => {
                    link_name(next).prev.set(link_name(elem).prev.get());
                }
            }
        }
    }
}

#[derive(Debug)]
struct Iter<'a, F, Elem> {
    next: Option<&'a Elem>,
    link_name: F
}

impl<'a, F, Elem> Iterator for Iter<'a, F, Elem>
where F: Fn(&'a Elem) -> &'a Link<Elem> {
    type Item = &'a Elem;

    fn next(&mut self) -> Option<&'a Elem> {
        let current_elem = self.next?;
        self.next = (self.link_name)(current_elem).next();
        Some(current_elem)
    }
}

impl<Elem> Head<Elem> {
    pub fn iter<F>(&self, link_name: F) -> impl Iterator<Item=&Elem>
    where F: Fn(&Elem) -> &Link<Elem> {
        Iter {
            next: self.front(),
            link_name: link_name
        }
    }
}

/// Inserts the queue element pointed to by elem at the front of the
/// queue headed by the head.
///
/// The link identified by link_name will be used to organize the element and
/// record its location in the queue.
///
/// This macro is unsafe and must be used in an unsafe block;
/// the caller must guarantee $elem remains pinned while in the queue.
macro_rules! insert_front {
    ( $head:expr, $elem:expr, $link_name:ident ) => {{
        ($head).insert_front($elem, |e| &e.$link_name)
    }}
}

/// Inserts the queue element pointed to by elem at the end of the
/// queue headed by the head.
///
/// The link identified by link_name will be used to organize the element and
/// record its location in the queue.
///
/// This macro is unsafe and must be used in an unsafe block;
/// the caller must guarantee $elem remains pinned while in the queue.
macro_rules! insert_tail {
    ( $head:expr, $elem:expr, $link_name:ident ) => {{
        ($head).insert_tail($elem, |e| &e.$link_name)
    }}
}

/// Inserts the queue element toinsert after the element inQ
/// in the queue.
///
/// Inserts an element into a queue after a given element. If the given
/// element is the last element, head should be updated appropriately
/// (so that toinsert becomes the tail element)
///
/// This macro is unsafe and must be used in an unsafe block;
/// the caller must guarantee $toinsert remains pinned while in the queue.
macro_rules! insert_after {
    ( $head:expr, $inQ:expr, $toinsert:expr, $link_name:ident ) => {{
        ($head).insert_after($inQ, $toinsert, |e| &e.$link_name)
    }}
}

/// Inserts the queue element toinsert before the element inQ
/// in the queue.
///
/// Inserts an element into a queue before a given element. If the given
/// element is the first element, head should be updated appropriately
/// (so that toinsert becomes the front element)
///
/// This macro is unsafe and must be used in an unsafe block;
/// the caller must guarantee $toinsert remains pinned while in the queue.
macro_rules! insert_before {
    ( $head:expr, $inQ:expr, $toinsert:expr, $link_name:ident ) => {{
        ($head).insert_before($inQ, $toinsert, |e| &e.$link_name)
    }}
}

 /// Detaches the element elem from the queue organized by link_name.
 ///
 /// If head does not use the link named link_name to organize its elements or
 /// if elem is not a member of head's queue, the behavior of this macro
 /// is undefined.
macro_rules! remove {
    ( $head:expr, $elem:expr, $link_name:ident ) => {{
        ($head).remove($elem, |e| &e.$link_name)
    }}
}

/// Constructs an iterator block (like a for block) that operates
/// on each element in head, in order.
///
/// foreach constructs the head of a block of code that will iterate through
/// each element in the queue headed by head. Each time through the loop,
/// the variable named by current_elem will be set to point to a subsequent
/// element in the queue.
///
///  Usage:
///  foreach!(current_elem,head,link_name, {
///    ... operate on the variable current_elem ...
///  }
///
///  If link_name is not used to organize the queue headed by head, then
///  the behavior of this macro is undefined.
macro_rules! foreach {
    ( $current_elem:ident, $head:expr, $link_name:ident, $block:block) => {{
        for $current_elem in ($head).iter(|e| &e.$link_name) {
            $block
        }
    }};
}
