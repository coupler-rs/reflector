use std::any::Any;

use super::{Context, Elem, Event, Response};
use crate::graphics::{Affine, Canvas};
use crate::{Point, ProposedSize, Size};

pub struct RowItem {
    pub(crate) offset: f32,
    pub(crate) hover: bool,
    pub(crate) elem: Box<dyn Elem>,
}

pub struct Row {
    pub(crate) spacing: f32,
    pub(crate) list_state: Box<dyn Any>,
    pub(crate) children: Vec<RowItem>,
}

impl Elem for Row {
    fn update(&mut self, cx: &mut Context) {
        for child in &mut self.children {
            child.elem.update(cx);
        }
    }

    fn hit_test(&mut self, cx: &mut Context, point: Point) -> bool {
        for child in self.children.iter_mut().rev() {
            if child.elem.hit_test(cx, point - Point::new(child.offset, 0.0)) {
                return true;
            }
        }

        false
    }

    fn handle(&mut self, cx: &mut Context, event: &Event) -> Response {
        match event {
            Event::MouseEnter => {}
            Event::MouseExit => {
                for child in &mut self.children {
                    if child.hover {
                        child.hover = false;
                        child.elem.handle(cx, &Event::MouseExit);
                        break;
                    }
                }
            }
            Event::MouseMove(pos) => {
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
                            child.elem.handle(cx, &Event::MouseExit);
                            break;
                        }
                    }
                }

                if let Some(hover) = hover {
                    let child = &mut self.children[hover];
                    if !child.hover {
                        child.hover = true;
                        child.elem.handle(cx, &Event::MouseEnter);
                    }

                    let pos = *pos - Point::new(child.offset, 0.0);
                    return child.elem.handle(cx, &Event::MouseMove(pos));
                }
            }
            Event::MouseDown(..) | Event::MouseUp(..) | Event::Scroll(..) => {
                for child in &mut self.children {
                    if child.hover {
                        return child.elem.handle(cx, event);
                    }
                }
            }
        }

        Response::Ignore
    }

    fn measure(&mut self, cx: &mut Context, proposal: ProposedSize) -> Size {
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

    fn place(&mut self, cx: &mut Context, size: Size) {
        let proposal = ProposedSize::new(None, Some(size.height));

        let mut offset = 0.0;
        for child in &mut self.children {
            let child_size = child.elem.measure(cx, proposal);
            child.elem.place(cx, child_size);

            child.offset = offset;

            offset += child_size.width + self.spacing;
        }
    }

    fn render(&mut self, cx: &mut Context, canvas: &mut Canvas) {
        for child in &mut self.children {
            let transform = Affine::translate(child.offset, 0.0);
            canvas.with_transform(transform, |canvas| {
                child.elem.render(cx, canvas);
            });
        }
    }
}
