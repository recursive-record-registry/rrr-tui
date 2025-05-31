use std::borrow::Cow;

use color_eyre::eyre::Result;
use ratatui::layout::Size;
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::animation::RectAnimation;
use crate::color::TextColor;
use crate::component::{Component, ComponentExt, ComponentId, DrawContext, Drawable};
use crate::layout::TaffyNodeData;
use crate::rect::{LineAlignment, PlaneAlignment, RectExt};

#[derive(Debug)]
pub struct SpinnerContent<'a> {
    pub text: Cow<'a, str>,
    pub animation: Option<RectAnimation>,
    pub color: TextColor,
}

impl<'a> Default for SpinnerContent<'a> {
    fn default() -> Self {
        Self {
            text: "".into(),
            animation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<'a> SpinnerContent<'a> {
    pub fn with_text(self, text: Cow<'a, str>) -> Self {
        Self { text, ..self }
    }

    pub fn with_animation(self, animation: Option<RectAnimation>) -> Self {
        Self { animation, ..self }
    }

    pub fn with_color(self, color: TextColor) -> Self {
        Self { color, ..self }
    }
}

#[derive(Debug)]
pub struct OpenStatus<'a> {
    pub id: ComponentId,
    pub taffy_node_data: TaffyNodeData,
    content: SpinnerContent<'a>,
}

impl<'a> OpenStatus<'a> {
    pub fn new(id: ComponentId, _tx: &UnboundedSender<Action>, content: SpinnerContent<'a>) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            taffy_node_data: Default::default(),
            content,
        }
    }

    pub fn set_content(&mut self, content: SpinnerContent<'a>) {
        self.content = content;
        self.mark_cached_layout_dirty();
    }
}

impl<'a> Component for OpenStatus<'a> {
    fn get_id(&self) -> ComponentId {
        self.id
    }

    fn get_taffy_node_data(&self) -> &TaffyNodeData {
        &self.taffy_node_data
    }

    fn get_taffy_node_data_mut(&mut self) -> &mut TaffyNodeData {
        &mut self.taffy_node_data
    }

    fn measure(
        &self,
        _known_dimensions: taffy::Size<Option<f32>>,
        _available_space: taffy::Size<taffy::AvailableSpace>,
    ) -> taffy::Size<f32> {
        taffy::Size {
            width: Span::raw(self.content.text.as_ref()).width() as f32,
            height: 1.0,
        }
    }
}

impl<'a> Drawable for OpenStatus<'a> {
    type Args<'b>
        = ()
    where
        Self: 'b;

    fn draw<'b>(&self, context: &mut DrawContext, (): Self::Args<'b>) -> Result<()>
    where
        Self: 'b,
    {
        // let padding_area = self.absolute_layout().padding_rect();
        let content_area = self.absolute_layout().content_rect();
        // area = area.without_padding(self.content.padding);
        let line = Span::styled(self.content.text.as_ref(), &self.content.color);
        let width = line.width() as u16;
        let area = content_area.align(
            Size::new(width, 1),
            PlaneAlignment {
                x: LineAlignment::End,
                y: LineAlignment::Start,
            },
        );

        context.draw_widget(&line, area);

        if let Some(animation) = self.content.animation.as_ref() {
            animation.apply(context, area);
        }

        Ok(())
    }
}
