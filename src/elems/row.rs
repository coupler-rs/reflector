use crate::graphics::{Affine, Canvas};
use crate::list::{Append, BuildItem, BuildList, Concat, EditVec, Empty};
use crate::{BuildElem, Elem, ElemContext, ElemEvent, Point, ProposedSize, Response, Size};

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

    pub fn child<E: BuildItem<ElemContext, RowItem>>(self, child: E) -> Row<Append<L, E>> {
        Row {
            spacing: self.spacing,
            children: Append(self.children, child),
        }
    }

    pub fn children<M: BuildList<ElemContext, RowItem>>(self, children: M) -> Row<Concat<L, M>> {
        Row {
            spacing: self.spacing,
            children: Concat(self.children, children),
        }
    }
}

impl<E: BuildElem> BuildItem<ElemContext, RowItem> for E {
    fn build_item(self, cx: &mut ElemContext) -> RowItem {
        RowItem {
            offset: 0.0,
            hover: false,
            elem: Box::new(self.build(cx)),
        }
    }

    fn rebuild_item(self, cx: &mut ElemContext, item: &mut RowItem) {
        self.rebuild(cx, item.elem.downcast_mut().unwrap());
    }
}

impl<L> BuildElem for Row<L>
where
    L: BuildList<ElemContext, RowItem>,
    L::State: 'static,
{
    type Elem = RowElem<L::State>;

    fn build(self, cx: &mut ElemContext) -> Self::Elem {
        let mut children = Vec::new();
        let list_state = self.children.build_list(cx, &mut EditVec::new(&mut children));

        RowElem {
            spacing: self.spacing,
            list_state,
            children,
        }
    }

    fn rebuild(self, cx: &mut ElemContext, elem: &mut Self::Elem) {
        elem.spacing = self.spacing;
        let mut children = EditVec::new(&mut elem.children);
        self.children.rebuild_list(cx, &mut children, &mut elem.list_state);
    }
}

pub struct RowItem {
    offset: f32,
    hover: bool,
    elem: Box<dyn Elem>,
}

pub struct RowElem<L> {
    spacing: f32,
    list_state: L,
    children: Vec<RowItem>,
}

impl<L: 'static> Elem for RowElem<L> {
    fn update(&mut self, cx: &mut ElemContext) {
        for child in &mut self.children {
            child.elem.update(cx);
        }
    }

    fn hit_test(&mut self, cx: &mut ElemContext, point: Point) -> bool {
        for child in self.children.iter_mut().rev() {
            if child.elem.hit_test(cx, point - Point::new(child.offset, 0.0)) {
                return true;
            }
        }

        false
    }

    fn handle(&mut self, cx: &mut ElemContext, event: &ElemEvent) -> Response {
        match event {
            ElemEvent::MouseEnter => {}
            ElemEvent::MouseExit => {
                for child in &mut self.children {
                    if child.hover {
                        child.hover = false;
                        child.elem.handle(cx, &ElemEvent::MouseExit);
                        break;
                    }
                }
            }
            ElemEvent::MouseMove(pos) => {
                let mut hover = None;
                let mut hover_changed = false;
                for (index, child) in self.children.iter_mut().enumerate().rev() {
                    if child.elem.hit_test(cx, *pos - Point::new(child.offset, 0.0)) {
                        hover = Some(index);
                        if !child.hover {
                            hover_changed = true;
                        }
                        break;
                    }
                }

                if hover_changed {
                    for child in &mut self.children {
                        if child.hover {
                            child.hover = false;
                            child.elem.handle(cx, &ElemEvent::MouseExit);
                            break;
                        }
                    }
                }

                if let Some(hover) = hover {
                    let child = &mut self.children[hover];
                    if !child.hover {
                        child.hover = true;
                        child.elem.handle(cx, &ElemEvent::MouseEnter);
                    }

                    let pos = *pos - Point::new(child.offset, 0.0);
                    return child.elem.handle(cx, &ElemEvent::MouseMove(pos));
                }
            }
            ElemEvent::MouseDown(..) | ElemEvent::MouseUp(..) | ElemEvent::Scroll(..) => {
                for child in &mut self.children {
                    if child.hover {
                        return child.elem.handle(cx, event);
                    }
                }
            }
        }

        Response::Ignore
    }

    fn measure(&mut self, cx: &mut ElemContext, proposal: ProposedSize) -> Size {
        let proposal = ProposedSize::new(None, proposal.height);

        let mut size = Size::new(0.0, 0.0);
        for child in &mut self.children {
            let child_size = child.elem.measure(cx, proposal);

            size.width += child_size.width + self.spacing;
            size.height = size.height.max(child_size.height);
        }

        size.width -= self.spacing;

        size
    }

    fn place(&mut self, cx: &mut ElemContext, size: Size) {
        let proposal = ProposedSize::new(None, Some(size.height));

        let mut offset = 0.0;
        for child in &mut self.children {
            let child_size = child.elem.measure(cx, proposal);
            child.elem.place(cx, child_size);

            child.offset = offset;

            offset += child_size.width + self.spacing;
        }
    }

    fn render(&mut self, cx: &mut ElemContext, canvas: &mut Canvas) {
        for child in &mut self.children {
            let transform = Affine::translate(child.offset, 0.0);
            canvas.with_transform(transform, |canvas| {
                child.elem.render(cx, canvas);
            });
        }
    }
}
