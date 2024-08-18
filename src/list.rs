use std::ops::{Bound, RangeBounds};

use crate::Context;

pub trait Edit<T> {
    fn len(&self) -> usize;
    fn push(&mut self, item: T);
    fn insert(&mut self, index: usize, item: T);
    fn remove(&mut self, index: usize) -> T;
    fn get(&self, index: usize) -> Option<&T>;
    fn get_mut(&mut self, index: usize) -> Option<&mut T>;
}

pub struct EditVec<'a, T> {
    vec: &'a mut Vec<T>,
}

impl<'a, T> EditVec<'a, T> {
    pub fn new(vec: &'a mut Vec<T>) -> EditVec<'a, T> {
        EditVec { vec }
    }
}

impl<'a, T> Edit<T> for EditVec<'a, T> {
    fn len(&self) -> usize {
        self.vec.len()
    }

    fn push(&mut self, item: T) {
        self.vec.push(item);
    }

    fn insert(&mut self, index: usize, item: T) {
        self.vec.insert(index, item);
    }

    fn remove(&mut self, index: usize) -> T {
        self.vec.remove(index)
    }

    fn get(&self, index: usize) -> Option<&T> {
        self.vec.get(index)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.vec.get_mut(index)
    }
}

pub struct EditRange<'a, E> {
    edit: &'a mut E,
    from_start: usize,
    from_end: usize,
}

impl<'a, E> EditRange<'a, E> {
    pub fn new<T, R>(edit: &'a mut E, range: R) -> EditRange<E>
    where
        E: Edit<T>,
        R: RangeBounds<usize>,
    {
        let len = edit.len();

        let start = match range.start_bound() {
            Bound::Included(&index) => index,
            Bound::Excluded(&index) => index + 1,
            Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            Bound::Included(&index) => index + 1,
            Bound::Excluded(&index) => index,
            Bound::Unbounded => len,
        };

        assert!(start <= end);
        assert!(end <= len);

        EditRange {
            edit,
            from_start: start,
            from_end: len - end,
        }
    }
}

impl<'a, T, E> Edit<T> for EditRange<'a, E>
where
    E: Edit<T>,
{
    fn len(&self) -> usize {
        let start = self.from_start;
        let end = self.edit.len() - self.from_end;
        end - start
    }

    fn push(&mut self, item: T) {
        self.edit.insert(self.edit.len() - self.from_end, item);
    }

    fn insert(&mut self, index: usize, item: T) {
        assert!(index <= self.len());

        self.edit.insert(self.from_start + index, item);
    }

    fn remove(&mut self, index: usize) -> T {
        assert!(index <= self.len());

        self.edit.remove(self.from_start + index)
    }

    fn get(&self, index: usize) -> Option<&T> {
        assert!(index <= self.len());

        self.edit.get(index)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        assert!(index <= self.len());

        self.edit.get_mut(index)
    }
}

pub trait BuildList<T> {
    type State;

    fn build_list(self, cx: &mut Context, list: &mut impl Edit<T>) -> Self::State;
    fn rebuild_list(self, cx: &mut Context, list: &mut impl Edit<T>, state: &mut Self::State);
}

pub trait BuildItem<T> {
    fn build_item(self, cx: &mut Context) -> T;
    fn rebuild_item(self, cx: &mut Context, item: &mut T);
}

pub struct Empty;

impl<T> BuildList<T> for Empty {
    type State = ();

    fn build_list(self, _cx: &mut Context, _list: &mut impl Edit<T>) -> Self::State {
        ()
    }

    fn rebuild_list(self, _cx: &mut Context, _list: &mut impl Edit<T>, _state: &mut Self::State) {}
}

pub struct Append<L, I>(pub L, pub I);

impl<L, I, T> BuildList<T> for Append<L, I>
where
    L: BuildList<T>,
    I: BuildItem<T>,
{
    type State = L::State;

    fn build_list(self, cx: &mut Context, list: &mut impl Edit<T>) -> Self::State {
        let state = self.0.build_list(cx, list);
        list.push(self.1.build_item(cx));
        state
    }

    fn rebuild_list(self, cx: &mut Context, list: &mut impl Edit<T>, state: &mut Self::State) {
        let last = list.len() - 1;
        self.0.rebuild_list(cx, &mut EditRange::new(list, ..last), state);
        self.1.rebuild_item(cx, list.get_mut(last).unwrap());
    }
}

pub struct Concat<L, M>(pub L, pub M);

pub struct ConcatState<L, M> {
    split: usize,
    first: L,
    second: M,
}

impl<L, M, T> BuildList<T> for Concat<L, M>
where
    L: BuildList<T>,
    M: BuildList<T>,
{
    type State = ConcatState<L::State, M::State>;

    fn build_list(self, cx: &mut Context, list: &mut impl Edit<T>) -> Self::State {
        let first = self.0.build_list(cx, list);
        let split = list.len();
        let second = self.1.build_list(cx, &mut EditRange::new(list, split..));

        ConcatState {
            split,
            first,
            second,
        }
    }

    fn rebuild_list(self, cx: &mut Context, list: &mut impl Edit<T>, state: &mut Self::State) {
        let mut first = EditRange::new(list, ..state.split);
        self.0.rebuild_list(cx, &mut first, &mut state.first);

        state.split = first.len();

        let mut second = EditRange::new(list, state.split..);
        self.1.rebuild_list(cx, &mut second, &mut state.second);
    }
}
