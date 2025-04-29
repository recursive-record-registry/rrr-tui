use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use nalgebra::SVector;
use ratatui::{
    buffer::Cell,
    layout::{Position, Rect},
};
use taffy::Overflow;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    color::{Color, ColorU8Rgb, TextColor},
    component::{
        Component, ComponentExt, ComponentId, DefaultDrawableComponent, Drawable,
        HandleEventSuccess,
    },
    layout::{TaffyNodeData, ext::ratatui::SizeExt},
    tui::Event,
};

enum ScrollAxis {
    Horizontal,
    Vertical,
}

enum ScrollDirection {
    Backward,
    Forward,
}

#[derive(Debug)]
pub struct ScrollPane<T: DefaultDrawableComponent> {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    pub child: T,
    scroll_position: Position,
}

impl<T> ScrollPane<T>
where
    T: DefaultDrawableComponent,
{
    pub fn new(id: ComponentId, _action_tx: &UnboundedSender<Action>, child: T) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            taffy_node_data: TaffyNodeData::new(taffy::Style {
                overflow: taffy::Point {
                    x: Overflow::Hidden,
                    y: Overflow::Hidden,
                },
                ..Default::default()
            }),
            child,
            scroll_position: Default::default(),
        }
    }

    fn scroll_size(&self) -> SVector<u16, 2> {
        let absolute_layout = self.absolute_layout();
        let overflow_size = absolute_layout.overflow_size().into_nalgebra();
        let content_size = absolute_layout.content_rect().as_size().into_nalgebra();

        // TODO: How to make this a saturating sub?
        overflow_size - content_size
    }

    fn scroll(
        &mut self,
        axis: ScrollAxis,
        direction: ScrollDirection,
    ) -> Result<HandleEventSuccess> {
        let scroll_size_2d = self.scroll_size();
        let (component, scroll_size) = match axis {
            ScrollAxis::Horizontal => (&mut self.scroll_position.x, scroll_size_2d.x),
            ScrollAxis::Vertical => (&mut self.scroll_position.y, scroll_size_2d.y),
        };
        *component = match direction {
            ScrollDirection::Backward => component.saturating_sub(1),
            ScrollDirection::Forward => std::cmp::min(*component + 1, scroll_size),
        };

        self.get_taffy_node_data_mut().mark_cached_layout_dirty();

        Ok(HandleEventSuccess::handled().with_action(Action::Render))
    }
}

impl<T> Component for ScrollPane<T>
where
    T: DefaultDrawableComponent,
{
    fn is_focusable(&self) -> bool {
        true
    }

    fn handle_event(&mut self, event: &Event) -> Result<HandleEventSuccess> {
        match event {
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollUp,
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Up,
                kind: KeyEventKind::Press,
                ..
            }) => self.scroll(ScrollAxis::Vertical, ScrollDirection::Backward),
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollDown,
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Down,
                kind: KeyEventKind::Press,
                ..
            }) => self.scroll(ScrollAxis::Vertical, ScrollDirection::Forward),
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollLeft,
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Left,
                kind: KeyEventKind::Press,
                ..
            }) => self.scroll(ScrollAxis::Horizontal, ScrollDirection::Backward),
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollRight,
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Right,
                kind: KeyEventKind::Press,
                ..
            }) => self.scroll(ScrollAxis::Horizontal, ScrollDirection::Forward),
            _ => Ok(HandleEventSuccess::unhandled()),
        }
    }

    fn scroll_position(&self) -> Position {
        self.scroll_position
    }

    fn get_id(&self) -> ComponentId {
        self.id
    }

    fn get_taffy_node_data(&self) -> &TaffyNodeData {
        &self.taffy_node_data
    }

    fn get_taffy_node_data_mut(&mut self) -> &mut TaffyNodeData {
        &mut self.taffy_node_data
    }

    fn get_children(&self) -> Vec<&dyn Component> {
        vec![&self.child]
    }

    fn get_children_mut(&mut self) -> Vec<&mut dyn Component> {
        vec![&mut self.child]
    }
}

impl<T> Drawable for ScrollPane<T>
where
    T: DefaultDrawableComponent,
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
        let scrollbar_color = ColorU8Rgb::new_f32(0.0, 0.0, 1.0).into();
        let rail_color = ColorU8Rgb::new_f32(0.0, 0.0, 0.3).into();

        let absolute_layout = self.absolute_layout();
        let content_rect = absolute_layout.content_rect();
        let scrollbar_area_vertical = Rect {
            x: content_rect.x + content_rect.width - 1,
            width: 1,
            ..content_rect
        };

        context
            .with_scroll_position(self.scroll_position)
            .draw_component(&self.child)?;

        let overflow_size = absolute_layout.overflow_size();

        if overflow_size.height > content_rect.height {
            let rail_len_eights = 8 * content_rect.height as u32;
            // The bar must span at least one cell (8 eights of a cell),
            // otherwise it could not be rendered with the unicode block
            // symbols.
            let bar_len_eights = std::cmp::max(
                8,
                (rail_len_eights * content_rect.height as u32)
                    .div_ceil(overflow_size.height as u32),
            );
            let bar_offset_eights = ((rail_len_eights - bar_len_eights)
                * self.scroll_position.y as u32)
                .div_ceil(self.scroll_size().y as u32);
            let bar_end_eights = bar_offset_eights + bar_len_eights;

            let bar_offset_start_ceil = bar_offset_eights.div_ceil(8);
            let bar_offset_end_floor = bar_end_eights / 8;

            // Draw rail.
            context.set_style(
                scrollbar_area_vertical,
                TextColor {
                    fg: ColorU8Rgb::default().into(),
                    bg: rail_color,
                },
            );

            // Draw top cell of the bar.
            if bar_offset_eights % 8 != 0 {
                let bar_offset_start_floor = bar_offset_eights / 8;
                let position = Position {
                    x: scrollbar_area_vertical.x,
                    y: scrollbar_area_vertical.y + bar_offset_start_floor as u16,
                };
                if let Some(cell) = context.get_scrolled_cell_mut(position) {
                    let height = bar_offset_eights - bar_offset_start_floor * 8;
                    draw_block_symbol(cell, height, scrollbar_color, false);
                }
            }

            // Draw bottom cell of the bar.
            if bar_end_eights % 8 != 0 {
                let position = Position {
                    x: scrollbar_area_vertical.x,
                    y: scrollbar_area_vertical.y + bar_offset_end_floor as u16,
                };
                if let Some(cell) = context.get_scrolled_cell_mut(position) {
                    let height = bar_end_eights - bar_offset_end_floor * 8;
                    draw_block_symbol(cell, height, scrollbar_color, true);
                }
            }

            // Fill in between top and bottom cells.
            context.set_style(
                Rect {
                    y: scrollbar_area_vertical.y + bar_offset_start_ceil as u16,
                    height: (bar_offset_end_floor - bar_offset_start_ceil) as u16,
                    ..scrollbar_area_vertical
                },
                TextColor {
                    fg: ColorU8Rgb::default().into(),
                    bg: scrollbar_color,
                },
            );
        }

        Ok(())
    }
}

fn draw_block_symbol(cell: &mut Cell, height: u32, color: Color, invert: bool) {
    const SYMBOLS: [&str; 9] = ["█", "▇", "▆", "▅", "▄", "▃", "▂", "▁", " "];
    cell.set_symbol(SYMBOLS[std::cmp::min(height, 8) as usize]);
    let mut style = TextColor {
        fg: ColorU8Rgb::try_from(cell.fg).unwrap_or_default().into(),
        bg: ColorU8Rgb::try_from(cell.bg).unwrap_or_default().into(),
    };
    style.fg = color;
    if invert {
        style = style.invert();
    }
    cell.set_style(style);
}
