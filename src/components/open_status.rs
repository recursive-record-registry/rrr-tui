use std::borrow::Cow;
use std::time::{Duration, Instant};

use color_eyre::eyre::Result;
use easing_function::{Easing, EasingFunction};
use ratatui::layout::{Position, Rect, Size};
use ratatui::text::Span;
use ratatui::widgets::Padding;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::color::{Lerp, TextColor};
use crate::component::{Component, ComponentId, DrawContext, Drawable};
use crate::rect::{LineAlignment, PlaneAlignment, RectExt};

#[derive(Debug)]
pub enum Animation {
    ProgressIndeterminate {
        period: Duration,
        highlight: TextColor,
    },
    Ease {
        easing_function: EasingFunction,
        instant_start: Instant,
        instant_end: Instant,
        color_start: TextColor,
        color_end: TextColor,
    },
}

impl Animation {
    fn apply(&self, context: &mut DrawContext, area: Rect) {
        match self {
            Animation::ProgressIndeterminate { period, highlight } => {
                let cos = (context.elapsed_time().as_secs_f32() * std::f32::consts::TAU
                    / period.as_secs_f32())
                .cos();
                let highlight_index =
                    (0.5 * (1.0 + cos) * area.width.saturating_sub(1) as f32 + 0.5) as u16;
                let position = Position::new(area.x + highlight_index, area.y);

                if let Some(cell) = context.frame().buffer_mut().cell_mut(position) {
                    cell.set_style(highlight);
                }
            }
            Animation::Ease {
                easing_function,
                instant_start,
                instant_end,
                color_start,
                color_end,
            } => {
                let style = if context.now() <= *instant_start {
                    color_start.clone()
                } else if context.now() >= *instant_end {
                    color_end.clone()
                } else {
                    let period = instant_end.duration_since(*instant_start).as_secs_f32();
                    let elapsed = context.now().duration_since(*instant_start).as_secs_f32();
                    let normalized = elapsed / period;
                    let eased = easing_function.ease(normalized);

                    Lerp::lerp(color_start, color_end, eased)
                };

                context.frame().buffer_mut().set_style(area, style);
            }
        }
    }
}

#[derive(Debug)]
pub struct SpinnerContent<'a> {
    pub text: Cow<'a, str>,
    pub padding: Padding,
    pub animation: Option<Animation>,
    pub color: TextColor,
}

impl<'a> Default for SpinnerContent<'a> {
    fn default() -> Self {
        Self {
            text: "".into(),
            padding: Default::default(),
            animation: Default::default(),
            color: Default::default(),
        }
    }
}

impl<'a> SpinnerContent<'a> {
    pub fn with_text(self, text: Cow<'a, str>) -> Self {
        Self { text, ..self }
    }

    pub fn with_padding(self, padding: Padding) -> Self {
        Self { padding, ..self }
    }

    pub fn with_animation(self, animation: Option<Animation>) -> Self {
        Self { animation, ..self }
    }

    pub fn with_color(self, color: TextColor) -> Self {
        Self { color, ..self }
    }
}

#[derive(Debug)]
pub struct OpenStatus<'a> {
    pub id: ComponentId,
    pub content: SpinnerContent<'a>,
}

impl<'a> OpenStatus<'a> {
    pub fn new(id: ComponentId, _tx: &UnboundedSender<Action>, content: SpinnerContent<'a>) -> Self
    where
        Self: Sized,
    {
        Self { id, content }
    }
}

impl<'a> Component for OpenStatus<'a> {
    fn get_id(&self) -> ComponentId {
        self.id
    }
}

impl<'a> Drawable for OpenStatus<'a> {
    type Args<'b>
        = ()
    where
        Self: 'b;

    fn draw<'b>(&self, context: &mut DrawContext, mut area: Rect, (): Self::Args<'b>) -> Result<()>
    where
        Self: 'b,
    {
        area = area.without_padding(self.content.padding);
        let line = Span::styled(self.content.text.as_ref(), &self.content.color);
        let width = line.width() as u16;
        area = area.align(
            Size::new(width, 1),
            PlaneAlignment {
                x: LineAlignment::End,
                y: LineAlignment::Start,
            },
        );

        context.frame().render_widget(line, area);

        if let Some(animation) = self.content.animation.as_ref() {
            animation.apply(context, area);
        }

        Ok(())
    }
}
