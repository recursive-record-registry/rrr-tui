use core::option::Option::Some;
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use color_eyre::eyre::{Result, eyre};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use pane_content::{PaneContent, PaneContentArgs};
use pane_open::{PaneOpen, PaneOpenArgs};
use ratatui::prelude::*;
use ratatui::widgets::Table;
use rrr::record::{
    HashedRecordKey, RECORD_NAME_ROOT, RecordKey, RecordName, RecordReadVersionSuccess,
    SuccessionNonce,
};
use rrr::registry::Registry;
use rrr::utils::fd_lock::ReadLock;
use rrr::utils::serde::BytesOrAscii;
use taffy::Dimension;
use taffy::prelude::{auto, length, line, min_content};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{Instrument, debug, info_span};

use crate::action::{Action, ComponentMessage};
use crate::args::Args;
use crate::color::{ColorOklch, TextColor};
use crate::component::{
    Component, ComponentExt, ComponentId, DrawContext, Drawable, HandleEventSuccess,
};
use crate::env::PROJECT_VERSION;
use crate::error;
use crate::layout::{LayoutExt, TaffyNodeData};
use crate::tui::Event;

use super::button::Button;
use super::input_field::InputField;
use super::layout_placeholder::LayoutPlaceholder;
use super::open_status::{Animation, OpenStatus, SpinnerContent};
use super::radio_array::RadioArray;

pub mod pane_content;
pub mod pane_open;

#[derive(Clone)]
pub struct LineSpacer {
    direction: Direction,
    begin: &'static str,
    inner: &'static str,
    end: &'static str,
    merged: &'static str,
}

impl Widget for LineSpacer {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        debug_assert!(
            (self.direction == Direction::Horizontal || area.width == 1)
                && (self.direction == Direction::Vertical || area.height == 1)
        );

        let start_position = area.as_position();

        if area.width == 0 || area.height == 0 {
            return;
        }

        if area.width <= 1 && area.height <= 1 {
            buf[start_position].set_symbol(self.merged);
            return;
        }

        buf[start_position].set_symbol(self.begin);

        match self.direction {
            Direction::Horizontal => {
                let end_position: Position =
                    (start_position.x + area.width - 1, start_position.y).into();
                buf[end_position].set_symbol(self.end);
                for x in (start_position.x + 1)..end_position.x {
                    let position = (x, start_position.y);
                    buf[position].set_symbol(self.inner);
                }
            }
            Direction::Vertical => {
                let end_position: Position =
                    (start_position.x, start_position.y + area.height - 1).into();
                buf[end_position].set_symbol(self.end);
                for y in (start_position.y + 1)..end_position.y {
                    let position = (start_position.x, y);
                    buf[position].set_symbol(self.inner);
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Encoding {
    Utf8,
    Hex,
}

impl Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Utf8 => write!(f, "UTF-8"),
            Self::Hex => write!(f, "Hexadecimal Byte String"),
        }
    }
}

#[derive(Debug, Clone)]
struct OpenedRecord {
    hashed_record_key: HashedRecordKey,
    record: Arc<RecordReadVersionSuccess>, // Rc'd for cheaper cloning
}

#[derive(Debug, Clone)]
struct MainState {
    registry: Arc<Registry<ReadLock>>,
    opened_record: Option<OpenedRecord>,
}

impl MainState {
    async fn get_current_succession_nonce(&self) -> SuccessionNonce {
        if let Some(opened_record) = self.opened_record.as_ref() {
            // This should be a pretty brief operation.
            opened_record
                .hashed_record_key
                .derive_succession_nonce(&self.registry.config.kdf)
                .await
                .unwrap()
        } else {
            self.registry
                .config
                .kdf
                .get_root_record_predecessor_nonce()
                .clone()
        }
    }
}

#[derive(Debug)]
pub struct MainView {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    args: Arc<Args>,
    placeholder_top: LayoutPlaceholder,
    placeholder_footer: LayoutPlaceholder,
    pane_content: PaneContent,
    pane_open: PaneOpen,
    state: Rc<RefCell<MainState>>,
}

impl MainView {
    pub async fn new(
        id: ComponentId,
        action_tx: &UnboundedSender<Action>,
        args: &Arc<Args>,
    ) -> Result<Self>
    where
        Self: Sized,
    {
        tracing::trace!(dir=?args.registry_directory);
        let registry = Arc::new(
            Registry::open(args.registry_directory.clone())
                .await
                .unwrap(),
        );
        let state = Rc::new(RefCell::new(MainState {
            registry,
            opened_record: None,
        }));
        let mut pane_open = PaneOpen::new(ComponentId::new(), action_tx, &state)?;

        pane_open.spawn_open_record_task_with_record_name(RECORD_NAME_ROOT); // Attempt to open the default root record.

        Ok(Self {
            id,
            taffy_node_data: TaffyNodeData::new(taffy::Style {
                display: taffy::Display::Grid,
                grid_template_columns: vec![
                    length(16.0),
                    length(1.0), // Divider
                    auto(),
                    length(1.0), // Divider
                    length(16.0),
                ],
                grid_template_rows: vec![
                    length(1.0),   // Header
                    length(7.0),   // Top
                    auto(),        // Content
                    min_content(), // Bottom
                    length(1.0),   // Footer
                ],
                size: taffy::Size {
                    width: Dimension::percent(1.0),
                    height: Dimension::percent(1.0),
                },
                ..Default::default()
            }),
            args: args.clone(),
            placeholder_top: LayoutPlaceholder::new(ComponentId::new(), action_tx).with_style(
                |style| taffy::Style {
                    grid_column: taffy::Line {
                        start: line(1),
                        end: line(5),
                    },
                    grid_row: taffy::Line {
                        start: line(1),
                        end: line(2),
                    },
                    ..style
                },
            ),
            pane_content: PaneContent::new(ComponentId::new(), action_tx, &state)?.with_style(
                |style| taffy::Style {
                    grid_column: taffy::Line {
                        start: line(1),
                        end: line(5),
                    },
                    grid_row: line(3),
                    ..style
                },
            ),
            placeholder_footer: LayoutPlaceholder::new(ComponentId::new(), action_tx).with_style(
                |style| taffy::Style {
                    grid_column: taffy::Line {
                        start: line(1),
                        end: line(5),
                    },
                    grid_row: line(5),
                    ..style
                },
            ),
            pane_open: pane_open.with_style(|style| taffy::Style {
                grid_column: taffy::Line {
                    start: line(1),
                    end: line(5),
                },
                grid_row: line(4),
                ..style
            }),
            state,
        })
    }

    fn pane_areas(area: Rect, title_offset_x: u16) -> (Rect, Rect) {
        let [mut title, content] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)])
            .areas(area);

        title.x += title_offset_x;
        title.width = title.width.saturating_sub(title_offset_x);

        (title, content)
    }

    fn draw_header(&self, context: &mut DrawContext, area_header: Rect) -> Result<()> {
        context.frame().render_widget(
            Span::raw(format!("RRR TUI v{}", *PROJECT_VERSION)),
            area_header,
        );
        Ok(())
    }

    fn draw_pane_tree(&self, context: &mut DrawContext, area: Rect) -> Result<()> {
        let (area_title, _area_content) = Self::pane_areas(area, 0);
        context
            .frame()
            .render_widget(Span::raw("[T]ree"), area_title);
        Ok(())
    }

    fn draw_pane_metadata(&self, context: &mut DrawContext, area: Rect) -> Result<()> {
        let (area_title, area_content) = Self::pane_areas(area, 0);

        if let Some(opened_record) = self.state.borrow().opened_record.as_ref() {
            let metadata_table = Table::new(
                opened_record
                    .record
                    .metadata
                    .iter_with_semantic_keys()
                    .map(|(key, value)| crate::cbor::record_metadata_to_row(key, value)),
                [Constraint::Length(16), Constraint::Fill(1)],
            );

            context.frame().render_widget(metadata_table, area_content);
        }

        context
            .frame()
            .render_widget(Span::raw("Record [M]etadata"), area_title);

        Ok(())
    }

    fn draw_pane_overview(&self, context: &mut DrawContext, area: Rect) -> Result<()> {
        let (area_title, _area_content) = Self::pane_areas(area, 0);
        context
            .frame()
            .render_widget(Span::raw("[O]verview"), area_title);
        Ok(())
    }
}

impl Component for MainView {
    fn update(&mut self, message: ComponentMessage) -> Result<Option<crate::action::Action>> {
        match message {
            ComponentMessage::RecordOpen {
                hashed_record_key,
                read_result: Some(read_result),
            } => {
                self.state.borrow_mut().opened_record = Some(OpenedRecord {
                    hashed_record_key,
                    record: Arc::new(read_result),
                });
                Ok(Some(Action::Render))
            }
            _ => Ok(None),
        }
    }

    fn get_id(&self) -> ComponentId {
        self.id
    }

    fn get_children(&self) -> Vec<&dyn Component> {
        vec![
            &self.placeholder_top,
            &self.pane_content,
            &self.pane_open,
            &self.placeholder_footer,
        ]
    }

    fn get_children_mut(&mut self) -> Vec<&mut dyn Component> {
        vec![
            &mut self.placeholder_top,
            &mut self.pane_content,
            &mut self.pane_open,
            &mut self.placeholder_footer,
        ]
    }

    fn get_accessibility_node(&self) -> Result<accesskit::Node> {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_children(vec![]);
        Ok(node)
    }

    fn get_taffy_node_data(&self) -> &TaffyNodeData {
        &self.taffy_node_data
    }

    fn get_taffy_node_data_mut(&mut self) -> &mut TaffyNodeData {
        &mut self.taffy_node_data
    }
}

impl Drawable for MainView {
    type Args<'a>
        = ()
    where
        Self: 'a;

    fn draw<'a>(&self, context: &mut DrawContext, (): Self::Args<'a>) -> Result<()>
    where
        Self: 'a,
    {
        const SPACER_HORIZONTAL: LineSpacer = LineSpacer {
            direction: Direction::Horizontal,
            begin: symbols::line::HORIZONTAL,
            inner: symbols::line::HORIZONTAL,
            end: symbols::line::HORIZONTAL,
            merged: symbols::line::HORIZONTAL,
        };
        const SPACER_VERTICAL_FORKED: LineSpacer = LineSpacer {
            direction: Direction::Vertical,
            begin: symbols::line::HORIZONTAL_DOWN,
            inner: symbols::line::VERTICAL,
            end: symbols::line::HORIZONTAL_UP,
            merged: symbols::line::HORIZONTAL,
        };

        let area = self.taffy_node_data.absolute_layout().content_rect();

        context
            .frame()
            .buffer_mut()
            .set_style(area, TextColor::default());

        let [
            area_header,
            area_top,
            area_content,
            area_bottom,
            area_footer,
        ] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(7),
                Constraint::Fill(1),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .areas(area);
        let layout_top = Layout::default()
            .direction(Direction::Horizontal)
            .spacing(1)
            .constraints([
                Constraint::Length(8),
                Constraint::Fill(1),
                Constraint::Length(16),
            ]);
        let [area_tree, area_metadata, area_overview] = layout_top.areas(area_top);
        let [_, area_top_spacer_0, area_top_spacer_1, _] = layout_top.spacers(area_top);

        context
            .frame()
            .render_widget(SPACER_HORIZONTAL.clone(), area_top);
        context
            .frame()
            .render_widget(SPACER_HORIZONTAL.clone(), area_content);
        context
            .frame()
            .render_widget(SPACER_HORIZONTAL.clone(), area_bottom);
        context
            .frame()
            .render_widget(SPACER_HORIZONTAL.clone(), area_footer);
        context.frame().render_widget(
            SPACER_VERTICAL_FORKED.clone(),
            Rect {
                height: area_top_spacer_0.height + 1,
                ..area_top_spacer_0
            },
        );
        context.frame().render_widget(
            SPACER_VERTICAL_FORKED.clone(),
            Rect {
                height: area_top_spacer_1.height + 1,
                ..area_top_spacer_1
            },
        );

        self.draw_pane_tree(context, area_tree)?;
        self.draw_pane_metadata(context, area_metadata)?;
        self.draw_pane_overview(context, area_overview)?;
        self.pane_content.draw(
            context,
            PaneContentArgs {
                title_offset_x: area_metadata.x,
            },
        );
        self.pane_open.draw(
            context,
            // area_bottom,
            PaneOpenArgs {
                title_offset_x: area_metadata.x,
            },
        )?;
        self.draw_header(context, area_header)?;

        /* Debug Oklch color space
        for y in area.y..(area.y + area.height) {
            for x in area.x..(area.x + area.width) {
                let u = (x - area.x) as f32 / area.width as f32;
                let v = (y - area.y) as f32 / area.height as f32;
                let Some(cell) = context.frame().buffer_mut().cell_mut((x, y)) else {
                    continue;
                };

                cell.set_char(' ');
                cell.bg = ColorOklch::new(0.75, 0.15 * v, u).into();
            }
        }
        */

        Ok(())
    }
}
