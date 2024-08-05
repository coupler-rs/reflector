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

pub trait ForEach<V> {
    fn for_each(&mut self, cx: &mut Context, visitor: &mut V);
}

pub trait Visit<T> {
    fn visit(&mut self, cx: &mut Context, value: &mut T);
}

pub struct Empty;

impl<B> BuildList<B> for Empty {
    type List = Empty;

    fn build_list(self, _cx: &mut Context, _builder: &mut B) -> Self::List {
        Empty
    }

    fn rebuild_list(self, _cx: &mut Context, _builder: &mut B, _list: &mut Self::List) {}
}

impl<V> ForEach<V> for Empty {
    fn for_each(&mut self, _cx: &mut Context, _visitor: &mut V) {}
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

impl<H, T, V> ForEach<V> for Append<H, T>
where
    H: ForEach<V>,
    V: Visit<T>,
{
    fn for_each(&mut self, cx: &mut Context, visitor: &mut V) {
        self.0.for_each(cx, visitor);
        visitor.visit(cx, &mut self.1);
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

impl<A, B, V> ForEach<V> for Concat<A, B>
where
    A: ForEach<V>,
    B: ForEach<V>,
{
    fn for_each(&mut self, cx: &mut Context, visitor: &mut V) {
        self.0.for_each(cx, visitor);
        self.1.for_each(cx, visitor);
    }
}
