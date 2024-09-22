use super::Build;
use crate::elem;
use crate::list::{Append, BuildItem, BuildList, Concat, EditVec, Empty};

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

    pub fn child<E: BuildItem<RowItem>>(self, child: E) -> Row<Append<L, E>> {
        Row {
            spacing: self.spacing,
            children: Append(self.children, child),
        }
    }

    pub fn children<M: BuildList<RowItem>>(self, children: M) -> Row<Concat<L, M>> {
        Row {
            spacing: self.spacing,
            children: Concat(self.children, children),
        }
    }
}

impl<E: Build> BuildItem<RowItem> for E {
    fn build_item(self) -> RowItem {
        RowItem {
            offset: 0.0,
            hover: false,
            elem: Box::new(self.build()),
        }
    }

    fn rebuild_item(self, item: &mut RowItem) {
        self.rebuild(item.elem.downcast_mut().unwrap());
    }
}

impl<L> Build for Row<L>
where
    L: BuildList<RowItem>,
    L::State: 'static,
{
    type Elem = elem::Row;

    fn build(self) -> Self::Elem {
        let mut children = Vec::new();
        let list_state = self.children.build_list(&mut EditVec::new(&mut children));

        elem::Row {
            spacing: self.spacing,
            list_state: Box::new(list_state),
            children,
        }
    }

    fn rebuild(self, elem: &mut Self::Elem) {
        elem.spacing = self.spacing;

        if let Some(list_state) = elem.list_state.downcast_mut() {
            let mut children = EditVec::new(&mut elem.children);
            self.children.rebuild_list(&mut children, list_state);
        } else {
            let mut children = Vec::new();
            let list_state = self.children.build_list(&mut EditVec::new(&mut children));
            elem.list_state = Box::new(list_state);
            elem.children = children;
        }
    }
}
