use std::cell::Cell;
use std::ptr;
use std::rc::Rc;

#[derive(Clone)]
pub struct Elem {
    inner: Rc<Node>,
}

struct Node {
    parent: Cell<*const Node>,
    first_child: Cell<*const Node>,
    last_child: Cell<*const Node>,
    prev_sibling: Cell<*const Node>,
    next_sibling: Cell<*const Node>,
}

impl Node {
    unsafe fn unlink(node: *const Node) {
        let parent = (*node).parent.replace(ptr::null());
        if let Some(parent) = unsafe { parent.as_ref() } {
            let prev_sibling = (*node).prev_sibling.replace(ptr::null());
            let next_sibling = (*node).next_sibling.replace(ptr::null());

            if let Some(prev_sibling) = unsafe { prev_sibling.as_ref() } {
                prev_sibling.next_sibling.set(next_sibling);
            } else {
                parent.first_child.set(next_sibling);
            }

            if let Some(next_sibling) = unsafe { next_sibling.as_ref() } {
                next_sibling.prev_sibling.set(prev_sibling);
            } else {
                parent.last_child.set(prev_sibling);
            }

            unsafe { drop(Rc::from_raw(node)) };
        }
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        while !self.first_child.get().is_null() {
            unsafe {
                Node::unlink(self.first_child.get());
            }
        }
    }
}

impl Elem {
    pub fn new() -> Elem {
        Elem {
            inner: Rc::new(Node {
                parent: Cell::new(ptr::null()),
                first_child: Cell::new(ptr::null()),
                last_child: Cell::new(ptr::null()),
                prev_sibling: Cell::new(ptr::null()),
                next_sibling: Cell::new(ptr::null()),
            }),
        }
    }

    pub fn add(&self, child: &Elem) {
        unsafe {
            Node::unlink(Rc::as_ptr(&child.inner));
        }

        child.inner.parent.set(Rc::as_ptr(&self.inner));
        child.inner.prev_sibling.set(self.inner.last_child.get());
        child.inner.next_sibling.set(ptr::null());

        let child_ptr = Rc::into_raw(Rc::clone(&child.inner));

        let last_child = self.inner.last_child.replace(child_ptr);
        if let Some(last_child) = unsafe { last_child.as_ref() } {
            last_child.next_sibling.set(child_ptr);
        } else {
            self.inner.first_child.set(child_ptr);
        }
    }

    pub fn remove(&self, child: &Elem) {
        if child.inner.parent.get() != Rc::as_ptr(&self.inner) {
            panic!("not a child of this element");
        }

        unsafe {
            Node::unlink(Rc::as_ptr(&child.inner));
        }
    }
}

#[cfg(test)]
#[test]
fn test() {
    let elem = Elem::new();
    let elem2 = Elem::new();

    elem.add(&elem2);
    elem.remove(&elem2);
}
