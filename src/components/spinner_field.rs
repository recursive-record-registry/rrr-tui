use std::borrow::Cow;

use ratatui::text::{Line, Span};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::component::{Component, ComponentId, Drawable};

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

impl<'a> Drawable for Spinner<'a> {
    type Args<'b>
        = ()
    where
        Self: 'b;

    fn draw<'b>(
        &self,
        context: &mut crate::component::DrawContext,
        area: ratatui::prelude::Rect,
        (): Self::Args<'b>,
    ) -> color_eyre::eyre::Result<()>
    where
        Self: 'b,
    {
        context
            .frame()
            .render_widget(Line::from_iter([Span::raw(self.text.as_ref())]), area);
        Ok(())
    }
}
