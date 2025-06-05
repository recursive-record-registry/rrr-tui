use std::fmt::Debug;

use color_eyre::eyre::Result;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    text::{Line, Span, Text},
    widgets::{Row, Table, WidgetRef},
};
use taffy::AvailableSpace;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    component::{Component, ComponentExt, ComponentId, Drawable},
    layout::TaffyNodeData,
};

pub trait MeasurableWidget: WidgetRef + Debug {
    fn measure(
        &self,
        known_dimensions: taffy::Size<Option<f32>>,
        available_space: taffy::Size<taffy::AvailableSpace>,
    ) -> taffy::Size<f32>;
}

#[derive(Debug)]
pub struct StyledWidget<T: MeasurableWidget> {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    pub widget: T,
}

impl<T> StyledWidget<T>
where
    T: MeasurableWidget,
{
    pub fn new(id: ComponentId, _action_tx: &UnboundedSender<Action>, widget: T) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            taffy_node_data: Default::default(),
            widget,
        }
    }
}

impl<T> Component for StyledWidget<T>
where
    T: MeasurableWidget,
{
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
        known_dimensions: taffy::Size<Option<f32>>,
        available_space: taffy::Size<taffy::AvailableSpace>,
    ) -> taffy::Size<f32> {
        self.widget.measure(known_dimensions, available_space)
    }
}

impl<T> Drawable for StyledWidget<T>
where
    T: MeasurableWidget,
{
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
        let area = self.absolute_layout().content_rect();
        context.draw_widget(&self.widget, area);
        Ok(())
    }
}

impl MeasurableWidget for Span<'_> {
    fn measure(
        &self,
        _known_dimensions: taffy::Size<Option<f32>>,
        _available_space: taffy::Size<taffy::AvailableSpace>,
    ) -> taffy::Size<f32> {
        taffy::Size {
            width: self.width() as f32,
            height: 1.0,
        }
    }
}

impl MeasurableWidget for Line<'_> {
    fn measure(
        &self,
        _known_dimensions: taffy::Size<Option<f32>>,
        _available_space: taffy::Size<taffy::AvailableSpace>,
    ) -> taffy::Size<f32> {
        taffy::Size {
            width: self.width() as f32,
            height: 1.0,
        }
    }
}

impl MeasurableWidget for Text<'_> {
    fn measure(
        &self,
        _known_dimensions: taffy::Size<Option<f32>>,
        _available_space: taffy::Size<taffy::AvailableSpace>,
    ) -> taffy::Size<f32> {
        taffy::Size {
            width: self.width() as f32,
            height: self.height() as f32,
        }
    }
}

#[derive(Debug, Default)]
pub struct TableProxy<'a> {
    pub rows: Vec<Row<'a>>,
    pub constraints: Vec<Constraint>,
}

impl WidgetRef for TableProxy<'_> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        Table::new(self.rows.iter().cloned(), self.constraints.iter().cloned())
            .render_ref(area, buf);
    }
}

impl MeasurableWidget for TableProxy<'_> {
    fn measure(
        &self,
        known_dimensions: taffy::Size<Option<f32>>,
        available_space: taffy::Size<taffy::AvailableSpace>,
    ) -> taffy::Size<f32> {
        taffy::Size {
            width: match available_space.width {
                AvailableSpace::Definite(space) => space,
                AvailableSpace::MaxContent | AvailableSpace::MinContent => 0.0,
            },
            height: self.rows.len() as f32,
        }
    }
}
