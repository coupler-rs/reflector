use std::ops::ControlFlow;

use crate::Context;

pub trait BuildList<B> {
    type List;

    fn build_list(self, cx: &mut Context, builder: &mut B) -> Self::List;
    fn rebuild_list(self, cx: &mut Context, builder: &mut B, list: &mut Self::List);
}

pub trait BuildItem<T> {
    type Item;

    fn build_item(&mut self, cx: &mut Context, value: T) -> Self::Item;
    fn rebuild_item(&mut self, cx: &mut Context, value: T, item: &mut Self::Item);
}

pub trait List<T: ?Sized> {
    fn for_each<F>(&mut self, f: F)
    where
        F: FnMut(&mut T);

    fn try_for_each<F, B>(&mut self, f: F) -> ControlFlow<B>
    where
        F: FnMut(&mut T) -> ControlFlow<B>;

    fn for_each_rev<F>(&mut self, f: F)
    where
        F: FnMut(&mut T);

    fn try_for_each_rev<F, B>(&mut self, f: F) -> ControlFlow<B>
    where
        F: FnMut(&mut T) -> ControlFlow<B>;
}

pub struct Empty;

impl<B> BuildList<B> for Empty {
    type List = Empty;

    fn build_list(self, _cx: &mut Context, _builder: &mut B) -> Self::List {
        Empty
    }

    fn rebuild_list(self, _cx: &mut Context, _builder: &mut B, _list: &mut Self::List) {}
}

impl<T: ?Sized> List<T> for Empty {
    fn for_each<F>(&mut self, _f: F)
    where
        F: FnMut(&mut T),
    {
    }

    fn try_for_each<F, B>(&mut self, _f: F) -> ControlFlow<B>
    where
        F: FnMut(&mut T) -> ControlFlow<B>,
    {
        ControlFlow::Continue(())
    }

    fn for_each_rev<F>(&mut self, _f: F)
    where
        F: FnMut(&mut T),
    {
    }

    fn try_for_each_rev<F, B>(&mut self, _f: F) -> ControlFlow<B>
    where
        F: FnMut(&mut T) -> ControlFlow<B>,
    {
        ControlFlow::Continue(())
    }
}

pub struct Append<H, T>(pub H, pub T);

impl<H, T, B> BuildList<B> for Append<H, T>
where
    H: BuildList<B>,
    B: BuildItem<T>,
{
    type List = Append<H::List, B::Item>;

    fn build_list(self, cx: &mut Context, builder: &mut B) -> Self::List {
        Append(
            self.0.build_list(cx, builder),
            builder.build_item(cx, self.1),
        )
    }

    fn rebuild_list(self, cx: &mut Context, builder: &mut B, list: &mut Self::List) {
        self.0.rebuild_list(cx, builder, &mut list.0);
        builder.rebuild_item(cx, self.1, &mut list.1);
    }
}

impl<H, U, T: ?Sized> List<T> for Append<H, U>
where
    H: List<T>,
    U: AsMut<T>,
{
    fn for_each<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T),
    {
        self.0.for_each(|x| f(x));
        f(self.1.as_mut());
    }

    fn try_for_each<F, B>(&mut self, mut f: F) -> ControlFlow<B>
    where
        F: FnMut(&mut T) -> ControlFlow<B>,
    {
        self.0.try_for_each(|x| f(x))?;
        f(self.1.as_mut())?;

        ControlFlow::Continue(())
    }

    fn for_each_rev<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T),
    {
        f(self.1.as_mut());
        self.0.for_each_rev(|x| f(x));
    }

    fn try_for_each_rev<F, B>(&mut self, mut f: F) -> ControlFlow<B>
    where
        F: FnMut(&mut T) -> ControlFlow<B>,
    {
        f(self.1.as_mut())?;
        self.0.try_for_each_rev(|x| f(x))?;

        ControlFlow::Continue(())
    }
}

pub struct Concat<L, M>(pub L, pub M);

impl<L, M, B> BuildList<B> for Concat<L, M>
where
    L: BuildList<B>,
    M: BuildList<B>,
{
    type List = Concat<L::List, M::List>;

    fn build_list(self, cx: &mut Context, builder: &mut B) -> Self::List {
        Concat(
            self.0.build_list(cx, builder),
            self.1.build_list(cx, builder),
        )
    }

    fn rebuild_list(self, cx: &mut Context, builder: &mut B, list: &mut Self::List) {
        self.0.rebuild_list(cx, builder, &mut list.0);
        self.1.rebuild_list(cx, builder, &mut list.1);
    }
}

impl<L, M, T: ?Sized> List<T> for Concat<L, M>
where
    L: List<T>,
    M: List<T>,
{
    fn for_each<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T),
    {
        self.0.for_each(|x| f(x));
        self.1.for_each(|x| f(x));
    }

    fn try_for_each<F, B>(&mut self, mut f: F) -> ControlFlow<B>
    where
        F: FnMut(&mut T) -> ControlFlow<B>,
    {
        self.0.try_for_each(|x| f(x))?;
        self.1.try_for_each(|x| f(x))?;

        ControlFlow::Continue(())
    }

    fn for_each_rev<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T),
    {
        self.1.for_each(|x| f(x));
        self.0.for_each(|x| f(x));
    }

    fn try_for_each_rev<F, B>(&mut self, mut f: F) -> ControlFlow<B>
    where
        F: FnMut(&mut T) -> ControlFlow<B>,
    {
        self.1.try_for_each(|x| f(x))?;
        self.0.try_for_each(|x| f(x))?;

        ControlFlow::Continue(())
    }
}
