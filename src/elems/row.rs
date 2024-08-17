use std::ops::ControlFlow;

use crate::graphics::{Affine, Canvas};
use crate::list::{Append, BuildItem, BuildList, Concat, Empty, List};
use crate::{Build, Context, Elem, Event, Point, ProposedSize, Response, Size};

pub struct Row<L> {
    spacing: f32,
    children: L,
}

impl Row<Empty> {
    pub fn new(spacing: f32) -> Row<Empty> {
        Row {
            spacing,
            children: Empty,
        }
    }
}

impl<L> Row<L> {
    pub fn spacing(mut self, spacing: f32) -> Row<L> {
        self.spacing = spacing;
        self
    }

    pub fn child<E: Build>(self, child: E) -> Row<Append<L, E>> {
        Row {
            spacing: self.spacing,
            children: Append(self.children, child),
        }
    }

    pub fn children<M: BuildList<RowBuilder>>(self, children: M) -> Row<Concat<L, M>> {
        Row {
            spacing: self.spacing,
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
            hover: false,
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
    L::List: List<RowItem<dyn Elem>> + 'static,
{
    type Elem = RowElem<L::List>;

    fn build(self, cx: &mut Context) -> Self::Elem {
        RowElem {
            spacing: self.spacing,
            children: self.children.build_list(cx, &mut RowBuilder),
        }
    }

    fn rebuild(self, cx: &mut Context, elem: &mut Self::Elem) {
        elem.spacing = self.spacing;
        self.children.rebuild_list(cx, &mut RowBuilder, &mut elem.children);
    }
}

pub struct RowItem<E: ?Sized> {
    offset: f32,
    hover: bool,
    elem: E,
}

pub struct RowElem<L> {
    spacing: f32,
    children: L,
}

impl<L> Elem for RowElem<L>
where
    L: List<RowItem<dyn Elem>> + 'static,
{
    fn update(&mut self, cx: &mut Context) {
        self.children.for_each(|child| {
            child.elem.update(cx);
        });
    }

    fn hit_test(&mut self, cx: &mut Context, point: Point) -> bool {
        let result = self.children.try_for_each_rev(|child| {
            if child.elem.hit_test(cx, point - Point::new(child.offset, 0.0)) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });

        result.is_break()
    }

    fn handle(&mut self, cx: &mut Context, event: &Event) -> Response {
        match event {
            Event::MouseEnter => {}
            Event::MouseExit => {
                self.children.try_for_each(|child| {
                    if child.hover {
                        child.hover = false;
                        child.elem.handle(cx, &Event::MouseExit);
                        return ControlFlow::Break(());
                    }

                    ControlFlow::Continue(())
                });
            }
            Event::MouseMove(pos) => {
                let mut index = 0;
                let mut hover = None;
                let mut hover_changed = false;
                self.children.try_for_each_rev(|child| {
                    if child.elem.hit_test(cx, *pos - Point::new(child.offset, 0.0)) {
                        hover = Some(index);
                        if !child.hover {
                            hover_changed = true;
                        }
                        return ControlFlow::Break(());
                    }

                    index += 1;
                    ControlFlow::Continue(())
                });

                if hover_changed {
                    self.children.try_for_each(|child| {
                        if child.hover {
                            child.hover = false;
                            child.elem.handle(cx, &Event::MouseExit);
                            return ControlFlow::Break(());
                        }

                        ControlFlow::Continue(())
                    });
                }

                if let Some(hover) = hover {
                    let mut index = 0;
                    let result = self.children.try_for_each_rev(|child| {
                        if index == hover {
                            if !child.hover {
                                child.hover = true;
                                child.elem.handle(cx, &Event::MouseEnter);
                            }

                            let pos = *pos - Point::new(child.offset, 0.0);
                            let response = child.elem.handle(cx, &Event::MouseMove(pos));
                            return ControlFlow::Break(response);
                        }

                        index += 1;
                        ControlFlow::Continue(())
                    });

                    return match result {
                        ControlFlow::Break(response) => response,
                        ControlFlow::Continue(()) => Response::Ignore,
                    };
                }
            }
            Event::MouseDown(..) | Event::MouseUp(..) | Event::Scroll(..) => {
                let result = self.children.try_for_each(|child| {
                    if child.hover {
                        let response = child.elem.handle(cx, &event);
                        return ControlFlow::Break(response);
                    }

                    ControlFlow::Continue(())
                });

                return match result {
                    ControlFlow::Break(response) => response,
                    ControlFlow::Continue(()) => Response::Ignore,
                };
            }
        }

        Response::Ignore
    }

    fn measure(&mut self, cx: &mut Context, proposal: ProposedSize) -> Size {
        let proposal = ProposedSize::new(None, proposal.height);

        let mut size = Size::new(0.0, 0.0);
        self.children.for_each(|child| {
            let child_size = child.elem.measure(cx, proposal);

            size.width += child_size.width + self.spacing;
            size.height = size.height.max(child_size.height);
        });

        size.width -= self.spacing;

        size
    }

    fn place(&mut self, cx: &mut Context, size: Size) {
        let proposal = ProposedSize::new(None, Some(size.height));

        let mut offset = 0.0;
        self.children.for_each(|child| {
            let child_size = child.elem.measure(cx, proposal);
            child.elem.place(cx, child_size);

            child.offset = offset;

            offset += child_size.width + self.spacing;
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
