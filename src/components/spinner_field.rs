use std::borrow::Cow;

use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::component::{Component, ComponentId};

#[derive(Debug)]
pub struct Spinner<'a> {
    id: ComponentId,
    text: Cow<'a, str>,
}

impl<'a> Spinner<'a> {
    pub fn new(id: ComponentId, _tx: &UnboundedSender<Action>, text: Cow<'a, str>) -> Self
    where
        Self: Sized,
    {
        Self { id, text }
    }
}

impl<'a> Component for Spinner<'a> {
    fn get_id(&self) -> ComponentId {
        self.id
    }
}
