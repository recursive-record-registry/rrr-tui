use std::{
    num::NonZero,
    ops::Range,
    time::Instant,
};

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use nalgebra::{SVector, vector};
use ratatui::buffer::Cell;
use taffy::Overflow;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    animation::{BlendAnimation, BlendAnimationDescriptor},
    color::{Blended, ColorOklab, ColorU8Rgb, Over, TextColor},
    component::{
        Component, ComponentExt, ComponentId, DefaultDrawableComponent, Drawable,
        HandleEventSuccess,
    },
    layout::TaffyNodeData,
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
struct ScrollBarLayoutCap {
    height_eights: NonZero<u8>,
    absolute_position: i16,
}

#[derive(Debug)]
struct ScrollBarLayout {
    rail_x: i16,
    rail_range_top: Range<i16>,
    rail_range_bottom: Range<i16>,
    bar_start_ceil: i16,
    bar_end_floor: i16,
    bar_start_cap: Option<ScrollBarLayoutCap>,
    bar_end_cap: Option<ScrollBarLayoutCap>,
}

#[derive(Debug)]
pub struct ScrollPane<T: DefaultDrawableComponent> {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    pub child: T,
    scroll_position: SVector<u16, 2>,
    scroll_bar_layout: Option<ScrollBarLayout>,
    animation: Option<BlendAnimation>,
    color_rail: Blended<ColorOklab>,
    color_bar: Blended<ColorOklab>,
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
            scroll_bar_layout: None,
            animation: None,
            color_rail: Blended::new(ColorU8Rgb::new_f32(1.0, 1.0, 1.0).into(), 0.2),
            color_bar: Blended::new(ColorU8Rgb::new_f32(1.0, 1.0, 1.0).into(), 1.0),
        }
    }

    pub fn with_animation(mut self, animation_descriptor: BlendAnimationDescriptor) -> Self {
        self.animation = Some(BlendAnimation::new_stopped(animation_descriptor));
        self
    }

    pub fn with_rail_color<C>(mut self, color_rail: Blended<C>) -> Self
    where
        ColorOklab: From<C>,
    {
        self.color_rail = color_rail.cast::<ColorOklab>();
        self
    }

    pub fn with_bar_color<C>(mut self, color_bar: Blended<C>) -> Self
    where
        ColorOklab: From<C>,
    {
        self.color_bar = color_bar.cast::<ColorOklab>();
        self
    }

    /// The overflow size expanded by the view scrolled out of the overflow bounds.
    /// This typically happens when the scroll pane is enlarged after scrolling to the end.
    fn expanded_overflow_size(&self) -> SVector<u16, 2> {
        let absolute_layout = self.absolute_layout();
        let overflow_size = absolute_layout.overflow_size();
        let content_size = absolute_layout.content_rect().extent();

        overflow_size.sup(&(content_size.try_cast::<u16>().unwrap() + self.scroll_position))
    }

    fn scroll_size(&self) -> SVector<u16, 2> {
        let absolute_layout = self.absolute_layout();
        let content_size = absolute_layout.content_rect().extent();
        let expanded_overflow_size = self.expanded_overflow_size();

        vector![
            expanded_overflow_size
                .x
                .saturating_sub(content_size.x as u16),
            expanded_overflow_size
                .y
                .saturating_sub(content_size.y as u16),
        ]
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

        if let Some(animation) = self.animation.as_mut() {
            animation.restart(Instant::now()); // TODO: Should be next frame's instant.
        }

        self.get_taffy_node_data_mut()
            .mark_cached_absolute_layout_dirty();

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

    fn scroll_position(&self) -> SVector<u16, 2> {
        self.scroll_position
    }

    fn on_absolute_layout_updated(&mut self) {
        let absolute_layout = self.absolute_layout();
        let content_rect = absolute_layout.content_rect();
        let overflow_size = absolute_layout.overflow_size();
        let display_scroll_bar =
            self.scroll_position().y > 0 || overflow_size.y as i16 > content_rect.extent().y;

        self.scroll_bar_layout = display_scroll_bar.then(|| {
            let scroll_size = self.scroll_size();
            let expanded_overflow_size = self.expanded_overflow_size();
            let rail_len_eights = 8 * content_rect.extent().y as u32;
            // The bar must span at least one cell (8 eights of a cell),
            // otherwise it could not be rendered with the unicode block
            // symbols.
            let bar_len_eights = std::cmp::max(
                8,
                (rail_len_eights * content_rect.extent().y as u32)
                    .div_ceil(expanded_overflow_size.y as u32),
            );
            let bar_start_eights = content_rect.min().y as i32 * 8
                + ((rail_len_eights - bar_len_eights) * self.scroll_position.y as u32)
                    .div_ceil(scroll_size.y as u32) as i32;
            let bar_end_eights = bar_start_eights + bar_len_eights as i32;
            let bar_start_floor = bar_start_eights.div_floor(8) as i16;
            let bar_start_ceil = bar_start_eights.div_ceil(8) as i16;
            let bar_end_floor = bar_end_eights.div_floor(8) as i16;
            let bar_end_ceil = bar_end_eights.div_ceil(8) as i16;

            // Lay out the top cell of the bar.
            let bar_start_cap = (bar_start_eights % 8 != 0).then(|| ScrollBarLayoutCap {
                absolute_position: bar_start_floor,
                height_eights: NonZero::new((bar_start_eights - bar_start_floor as i32 * 8) as u8)
                    .expect("the remainder is assumed to be 0"),
            });

            // Lay out the bottom cell of the bar.
            let bar_end_cap = (bar_end_eights % 8 != 0).then(|| ScrollBarLayoutCap {
                absolute_position: bar_end_floor,
                height_eights: NonZero::new((bar_end_eights - bar_end_floor as i32 * 8) as u8)
                    .expect("the remainder is assumed to be 0"),
            });

            ScrollBarLayout {
                rail_x: content_rect.min().x + content_rect.extent().x - 1,
                rail_range_top: content_rect.min().y..bar_start_floor,
                rail_range_bottom: bar_end_ceil..content_rect.max().y,
                bar_start_ceil,
                bar_end_floor,
                bar_start_cap,
                bar_end_cap,
            }
        });
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
        context.draw_component(&self.child)?;

        let Some(animation) = self.animation.as_ref() else {
            return Ok(());
        };

        let Some(scrollbar_layout) = self.scroll_bar_layout.as_ref() else {
            return Ok(());
        };

        let now = context.now();
        let alpha_rail = animation.apply(now, &self.color_rail.alpha, &0.0);
        let alpha_bar = animation.apply(now, &self.color_bar.alpha, &0.0);

        if alpha_rail <= 0.0 && alpha_bar <= 0.0 {
            return Ok(());
        }

        let color_rail = Blended::new(self.color_rail.color, alpha_rail);
        let color_bar = Blended::new(self.color_bar.color, alpha_bar);

        // Draw rail.
        for y in scrollbar_layout
            .rail_range_top
            .clone()
            .chain(scrollbar_layout.rail_range_bottom.clone())
        {
            if let Some(cell) = context.get_cell_mut([scrollbar_layout.rail_x, y]) {
                cell.bg = color_rail
                    .over(&ColorU8Rgb::try_from(cell.bg).unwrap_or_default().into())
                    .into();
            }
        }

        // Draw top cell of the bar.
        if let Some(bar_start_cap) = scrollbar_layout.bar_start_cap.as_ref() {
            let position = vector![scrollbar_layout.rail_x, bar_start_cap.absolute_position];
            if let Some(cell) = context.get_cell_mut(position) {
                draw_block_symbol(
                    cell,
                    bar_start_cap.height_eights,
                    color_bar,
                    color_rail,
                    false,
                );
            }
        }

        // Draw bottom cell of the bar.
        if let Some(bar_end_cap) = scrollbar_layout.bar_end_cap.as_ref() {
            let position = vector![scrollbar_layout.rail_x, bar_end_cap.absolute_position];
            if let Some(cell) = context.get_cell_mut(position) {
                draw_block_symbol(cell, bar_end_cap.height_eights, color_bar, color_rail, true);
            }
        }

        // Fill in between top and bottom cells.
        for y in scrollbar_layout.bar_start_ceil..scrollbar_layout.bar_end_floor {
            if let Some(cell) = context.get_cell_mut([scrollbar_layout.rail_x, y]) {
                cell.set_char(' ');
                cell.bg = color_bar
                    .over(&ColorU8Rgb::try_from(cell.bg).unwrap_or_default().into())
                    .into();
            }
        }

        Ok(())
    }
}

fn draw_block_symbol(
    cell: &mut Cell,
    height: NonZero<u8>,
    color_fg: Blended<ColorOklab>,
    color_bg: Blended<ColorOklab>,
    invert: bool,
) {
    const SYMBOLS: [&str; 9] = ["█", "▇", "▆", "▅", "▄", "▃", "▂", "▁", " "];
    cell.set_symbol(SYMBOLS[std::cmp::min(height.get(), 8) as usize]);
    let mut style = TextColor {
        fg: color_fg
            .over(&ColorU8Rgb::try_from(cell.bg).unwrap_or_default().into())
            .into(),
        bg: color_bg
            .over(&ColorU8Rgb::try_from(cell.bg).unwrap_or_default().into())
            .into(),
    };
    if invert {
        style = style.invert();
    }
    cell.set_style(style);
}
