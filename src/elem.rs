use std::cell::{Cell, RefCell};
use std::ptr;
use std::rc::Rc;

unsafe fn drop_node<T: 'static>(node: *const Node) {
    drop(Rc::from_raw(node as *const ElemInner<T>));
}

struct Node {
    drop: unsafe fn(*const Node),
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

            unsafe {
                ((*node).drop)(node);
            }
        }
    }
}

#[derive(Clone)]
pub struct Elem<T> {
    inner: Rc<ElemInner<T>>,
}

#[repr(C)]
struct ElemInner<T: ?Sized> {
    node: Node,
    data: RefCell<T>,
}

impl<T: ?Sized> Drop for ElemInner<T> {
    fn drop(&mut self) {
        while !self.node.first_child.get().is_null() {
            unsafe {
                Node::unlink(self.node.first_child.get());
            }
        }
    }
}

impl<T: 'static> Elem<T> {
    pub fn new(data: T) -> Elem<T> {
        Elem {
            inner: Rc::new(ElemInner {
                node: Node {
                    drop: drop_node::<T>,
                    parent: Cell::new(ptr::null()),
                    first_child: Cell::new(ptr::null()),
                    last_child: Cell::new(ptr::null()),
                    prev_sibling: Cell::new(ptr::null()),
                    next_sibling: Cell::new(ptr::null()),
                },
                data: RefCell::new(data),
            }),
        }
    }

    pub fn add<U>(&self, child: &Elem<U>) {
        unsafe {
            Node::unlink(Rc::as_ptr(&child.inner) as *const Node);
        }

        child.inner.node.parent.set(Rc::as_ptr(&self.inner) as *const Node);
        child.inner.node.prev_sibling.set(self.inner.node.last_child.get());
        child.inner.node.next_sibling.set(ptr::null());

        let child_ptr = Rc::into_raw(Rc::clone(&child.inner)) as *const Node;

        let last_child = self.inner.node.last_child.replace(child_ptr);
        if let Some(last_child) = unsafe { last_child.as_ref() } {
            last_child.next_sibling.set(child_ptr);
        } else {
            self.inner.node.first_child.set(child_ptr);
        }
    }

    pub fn remove<U>(&self, child: &Elem<U>) {
        if child.inner.node.parent.get() != Rc::as_ptr(&self.inner) as *const Node {
            panic!("not a child of this element");
        }

        unsafe {
            Node::unlink(Rc::as_ptr(&child.inner) as *const Node);
        }
    }
}

#[cfg(test)]
mod test {
    use super::Elem;
    use std::cell::Cell;
    use std::rc::Rc;

    struct MyElem(Rc<Cell<u32>>);

    impl Drop for MyElem {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn cleanup() {
        let dropped = Rc::new(Cell::new(0));

        let elem1 = Elem::new(MyElem(Rc::clone(&dropped)));
        let elem2 = Elem::new(MyElem(Rc::clone(&dropped)));
        let elem3 = Elem::new(MyElem(Rc::clone(&dropped)));
        let elem4 = Elem::new(MyElem(Rc::clone(&dropped)));

        assert_eq!(dropped.get(), 0);

        elem1.add(&elem2);
        elem1.add(&elem3);
        elem2.add(&elem4);

        drop(elem2);
        drop(elem3);
        drop(elem4);

        assert_eq!(dropped.get(), 0);

        drop(elem1);

        assert_eq!(dropped.get(), 4);
    }

    #[test]
    fn remove() {
        let dropped = Rc::new(Cell::new(0));

        let elem1 = Elem::new(MyElem(Rc::clone(&dropped)));
        let elem2 = Elem::new(MyElem(Rc::clone(&dropped)));

        assert_eq!(dropped.get(), 0);

        elem1.add(&elem2);
        elem1.remove(&elem2);

        drop(elem2);

        assert_eq!(dropped.get(), 1);

        drop(elem1);

        assert_eq!(dropped.get(), 2);
    }
}
