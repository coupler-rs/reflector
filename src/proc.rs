use std::any::Any;

use crate::{AsAny, Response};

pub struct ProcContext {}

#[derive(Clone, Debug)]
pub enum ProcEvent {}

pub trait BuildProc {
    type Proc: Proc;

    fn build(self, cx: &mut ProcContext) -> Self::Proc;
    fn rebuild(self, cx: &mut ProcContext, proc: &mut Self::Proc);
}

pub trait Proc: AsAny {
    fn update(&mut self, cx: &mut ProcContext);
    fn handle(&mut self, cx: &mut ProcContext, event: &ProcEvent) -> Response;
}

impl dyn Proc {
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: Any,
    {
        self.as_any().downcast_ref()
    }

    pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any,
    {
        self.as_mut_any().downcast_mut()
    }
}
