use std::ops::ControlFlow;

use crate::graphics::{Affine, Canvas};
use crate::list::{Append, BuildItem, BuildList, Concat, Empty, List};
use crate::{Build, Context, Elem, Event, ProposedSize, Response, Size};

pub struct Row<L> {
    children: L,
}

impl Row<Empty> {
    pub fn new() -> Row<Empty> {
        Row { children: Empty }
    }
}

impl<L> Row<L> {
    pub fn child<E: Build>(self, child: E) -> Row<Append<L, E>> {
        Row {
            children: Append(self.children, child),
        }
    }

    pub fn children<M: BuildList<RowBuilder>>(self, children: M) -> Row<Concat<L, M>> {
        Row {
            children: Concat(self.children, children),
        }
    }
}

pub struct RowBuilder;

impl<E: Elem + 'static> AsMut<RowItem<dyn Elem>> for RowItem<E> {
    fn as_mut(&mut self) -> &mut RowItem<dyn Elem> {
        self
    }
}

impl<E: Build> BuildItem<E> for RowBuilder {
    type Item = RowItem<E::Elem>;

    fn build_item(&mut self, cx: &mut Context, value: E) -> Self::Item {
        RowItem {
            offset: 0.0,
            elem: value.build(cx),
        }
    }

    fn rebuild_item(&mut self, cx: &mut Context, value: E, item: &mut Self::Item) {
        value.rebuild(cx, &mut item.elem);
    }
}

impl<L> Build for Row<L>
where
    L: BuildList<RowBuilder>,
    L::List: List<RowItem<dyn Elem>>,
{
    type Elem = RowElem<L::List>;

    fn build(self, cx: &mut Context) -> Self::Elem {
        RowElem {
            children: self.children.build_list(cx, &mut RowBuilder),
        }
    }

    fn rebuild(self, cx: &mut Context, elem: &mut Self::Elem) {
        self.children.rebuild_list(cx, &mut RowBuilder, &mut elem.children);
    }
}

pub struct RowItem<E: ?Sized> {
    offset: f32,
    elem: E,
}

pub struct RowElem<L> {
    children: L,
}

impl<L> Elem for RowElem<L>
where
    L: List<RowItem<dyn Elem>>,
{
    fn update(&mut self, cx: &mut Context) {
        self.children.for_each(|child| {
            child.elem.update(cx);
        });
    }

    fn handle(&mut self, cx: &mut Context, event: &Event) -> Response {
        let result = self.children.try_for_each_rev(|child| {
            if child.elem.handle(cx, event) == Response::Capture {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });

        if result.is_break() {
            Response::Capture
        } else {
            Response::Ignore
        }
    }

    fn measure(&mut self, cx: &mut Context, proposal: ProposedSize) -> Size {
        let proposal = ProposedSize::new(None, proposal.height);

        let mut size = Size::new(0.0, 0.0);
        self.children.for_each(|child| {
            let child_size = child.elem.measure(cx, proposal);

            size.width += child_size.width;
            size.height = size.height.max(child_size.height);
        });

        size
    }

    fn place(&mut self, cx: &mut Context, size: Size) {
        let proposal = ProposedSize::new(None, Some(size.height));

        let mut offset = 0.0;
        self.children.for_each(|child| {
            let child_size = child.elem.measure(cx, proposal);
            child.elem.place(cx, child_size);

            child.offset = offset;

            offset += child_size.width;
        });
    }

    fn render(&mut self, cx: &mut Context, canvas: &mut Canvas) {
        self.children.for_each(|child| {
            let transform = Affine::translate(child.offset, 0.0);
            canvas.with_transform(transform, |canvas| {
                child.elem.render(cx, canvas);
            });
        });
    }
}
