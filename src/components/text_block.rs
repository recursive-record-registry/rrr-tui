use std::borrow::Cow;

use color_eyre::eyre::Result;
use itertools::Itertools;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::Widget,
};
use taffy::AvailableSpace;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    color::{ColorU8Rgb, TextColor},
    component::{Component, ComponentId, Drawable},
    layout::TaffyNodeData,
    tracing_dbg,
};

#[derive(Debug)]
pub struct TextBlock {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    // pub unwrapped_lines: Vec<Line<'static>>,
    pub text: Cow<'static, str>,
}

impl TextBlock {
    pub fn new(id: ComponentId, _action_tx: &UnboundedSender<Action>) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            taffy_node_data: Default::default(),
            // unwrapped_lines: Default::default(),
            text: "".into(),
        }
    }

    pub fn with_text(self, text: impl Into<Cow<'static, str>>) -> Self {
        Self {
            text: text.into(),
            ..self
        }
    }

    // pub fn with_lines(self, unwrapped_lines: Vec<Line<'static>>) -> Self {
    //     Self {
    //         unwrapped_lines,
    //         ..self
    //     }
    // }

    pub fn wrapped_lines_width(&self, available_space_width: AvailableSpace) -> usize {
        match available_space_width {
            // Length of the longest word.
            AvailableSpace::MinContent => self
                .text
                .split_whitespace()
                .map(str::len)
                .max()
                .unwrap_or(0),
            // Length of the longest line.
            AvailableSpace::MaxContent => self.text.lines().map(str::len).max().unwrap_or(0),
            AvailableSpace::Definite(width) => width as usize,
        }
    }

    pub fn wrapped_lines<'a>(
        &'a self,
        available_space_width: AvailableSpace,
    ) -> impl Iterator<Item = Cow<'a, str>> {
        if matches!(available_space_width, AvailableSpace::MaxContent) {
            return Box::new(self.text.lines().map(Cow::Borrowed))
                as Box<dyn Iterator<Item = Cow<'a, str>>>;
        }

        let width = self.wrapped_lines_width(available_space_width);
        // Handle both "\r" and "\r\n" line endings using `str::lines`, as the `textwrap` crate only
        // allows handling one of them.
        Box::new(
            self.text.lines().flat_map(move |line| {
                textwrap::wrap(line, textwrap::Options::new(width)).into_iter()
            }),
        )
    }
}

impl Component for TextBlock {
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
        available_space: taffy::Size<taffy::AvailableSpace>,
    ) -> taffy::Size<f32> {
        let wrapped_lines = self.wrapped_lines(available_space.width);
        let mut width = 0;
        let mut height = 0;

        for line in wrapped_lines {
            width = std::cmp::max(width, Span::raw(line).width());
            height += 1;
        }

        tracing_dbg!(taffy::Size {
            width: width as f32,
            height: height as f32,
        })
    }
}

impl Drawable for TextBlock {
    type Args<'a>
        = ()
    where
        Self: 'a;

    fn draw<'a>(
        &self,
        context: &mut crate::component::DrawContext,
        (): Self::Args<'a>,
    ) -> Result<()>
    where
        Self: 'a,
    {
        let content_rect = self.get_taffy_node_data().absolute_layout().content_rect();
        let lines = self.wrapped_lines(AvailableSpace::Definite(content_rect.width as f32));

        for (line, y) in lines.zip(content_rect.y..) {
            debug_assert!(
                !line.as_ref().chars().any(|c| c == '\r'),
                "Carriage returns mess with style rendering."
            );

            let span = Span::raw(line);
            let rect = Rect {
                x: content_rect.x,
                y,
                // width: content_rect.width,
                width: span.width() as u16,
                height: 1,
            };
            context.frame().render_widget(span, rect);
        }

        Ok(())
    }
}
