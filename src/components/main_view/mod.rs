use core::option::Option::Some;
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;
use std::sync::Arc;

use color_eyre::eyre::Result;
use nalgebra::{SVector, vector};
use panes::content::PaneContent;
use panes::metadata::PaneMetadata;
use panes::open::PaneOpen;
use panes::overview::PaneOverview;
use panes::tree::PaneTree;
use ratatui::prelude::*;
use ratatui::widgets::WidgetRef;
use rrr::record::{HashedRecordKey, RECORD_NAME_ROOT, RecordReadVersionSuccess, SuccessionNonce};
use rrr::registry::Registry;
use rrr::utils::fd_lock::ReadLock;
use taffy::Dimension;
use taffy::prelude::{fr, length, line, min_content, minmax, percent, zero};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::{Action, ComponentMessage};
use crate::args::Args;
use crate::color::TextColor;
use crate::component::{
    Component, ComponentExt, ComponentId, DefaultDrawableComponent, DrawContext, Drawable,
};
use crate::env::PROJECT_VERSION;
use crate::geometry::Rectangle;
use crate::layout::TaffyNodeData;
use crate::widgets::line_spacer::{LineSpacerOld, LineType, RectSpacer};

use super::layout_placeholder::LayoutPlaceholder;

pub mod panes;

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
pub struct MainState {
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
    placeholder_header: LayoutPlaceholder,
    placeholder_footer: LayoutPlaceholder,
    pane_tree: PaneTree,
    pane_metadata: PaneMetadata,
    pane_overview: PaneOverview,
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
                gap: length(1.0),
                grid_template_columns: vec![length(12.0), minmax(zero(), fr(1.0)), length(12.0)],
                grid_template_rows: vec![
                    length(1.0),             // Header
                    length(10.0),            // Top
                    minmax(zero(), fr(1.0)), // Content
                    min_content(),           // Bottom
                    length(0.0),             // Footer
                ],
                size: percent(1.0),
                ..Default::default()
            }),
            args: args.clone(),
            placeholder_header: LayoutPlaceholder::new(ComponentId::new()).with_style(|style| {
                taffy::Style {
                    grid_column: taffy::Line {
                        start: line(1),
                        end: line(4),
                    },
                    grid_row: taffy::Line {
                        start: line(1),
                        end: line(2),
                    },
                    ..style
                }
            }),
            pane_tree: PaneTree::new(ComponentId::new(), action_tx, &state)?.with_style(|style| {
                taffy::Style {
                    grid_column: taffy::Line {
                        start: line(1),
                        end: line(2),
                    },
                    grid_row: taffy::Line {
                        start: line(2),
                        end: line(3),
                    },
                    ..style
                }
            }),
            pane_metadata: PaneMetadata::new(ComponentId::new(), action_tx, &state)?.with_style(
                |style| taffy::Style {
                    grid_column: taffy::Line {
                        start: line(2),
                        end: line(3),
                    },
                    grid_row: taffy::Line {
                        start: line(2),
                        end: line(3),
                    },
                    ..style
                },
            ),
            pane_overview: PaneOverview::new(ComponentId::new(), action_tx, &state)?.with_style(
                |style| taffy::Style {
                    grid_column: taffy::Line {
                        start: line(3),
                        end: line(4),
                    },
                    grid_row: taffy::Line {
                        start: line(2),
                        end: line(3),
                    },
                    ..style
                },
            ),
            pane_content: PaneContent::new(ComponentId::new(), action_tx, &state)?.with_style(
                |style| taffy::Style {
                    grid_column: taffy::Line {
                        start: line(1),
                        end: line(4),
                    },
                    grid_row: line(3),
                    ..style
                },
            ),
            pane_open: pane_open.with_style(|style| taffy::Style {
                grid_column: taffy::Line {
                    start: line(1),
                    end: line(4),
                },
                grid_row: line(4),
                ..style
            }),
            placeholder_footer: LayoutPlaceholder::new(ComponentId::new()).with_style(|style| {
                taffy::Style {
                    margin: taffy::Rect {
                        top: length(-1.0),
                        ..zero()
                    },
                    grid_column: taffy::Line {
                        start: line(1),
                        end: line(4),
                    },
                    grid_row: line(5),
                    ..style
                }
            }),
            state,
        })
    }

    fn draw_header(&self, context: &mut DrawContext, area_header: Rectangle<i16>) -> Result<()> {
        context.draw_widget(
            &Span::raw(format!("RRR TUI v{}", *PROJECT_VERSION)),
            area_header,
        );
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
            &self.placeholder_header,
            &self.pane_tree,
            &self.pane_metadata,
            &self.pane_overview,
            &self.pane_content,
            &self.pane_open,
            &self.placeholder_footer,
        ]
    }

    fn get_children_mut(&mut self) -> Vec<&mut dyn Component> {
        vec![
            &mut self.placeholder_header,
            &mut self.pane_tree,
            &mut self.pane_metadata,
            &mut self.pane_overview,
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

fn get_title_area_for(component: &impl Component, x_offset: i16) -> Rectangle<i16> {
    let area = component.absolute_layout().border_rect();
    Rectangle::from_minmax(
        [area.min().x + x_offset, area.min().y - 1],
        [area.max().x + x_offset, area.min().y],
    )
}

fn draw_pane(
    context: &mut DrawContext,
    component: &impl DefaultDrawableComponent,
    x_offset: i16,
    title: &str,
) -> Result<()> {
    let focused = context.is_child_focused(component.get_id());
    let border_area = component.absolute_layout().border_rect();
    let rect_area = Rectangle::from_minmax(
        border_area.min() - SVector::from([1, 1]),
        border_area.max() + SVector::from([1, 1]),
    );
    context.draw_widget(
        &RectSpacer {
            line_type: if focused {
                LineType::Bold
            } else {
                LineType::Standard
            },
        },
        rect_area,
    );
    context.draw_widget(&Span::raw(title), get_title_area_for(component, x_offset));
    context.draw_component(component)?;
    Ok(())
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
        let area = self.taffy_node_data.absolute_layout().content_rect();

        // Draw the background of the entire main window.
        context.set_style(area, TextColor::default());

        draw_pane(context, &self.pane_tree, 0, "[T]ree")?;
        draw_pane(context, &self.pane_metadata, 0, "Record [M]etadata")?;
        draw_pane(context, &self.pane_overview, 0, "[O]verview")?;
        draw_pane(
            context,
            &self.pane_content,
            self.pane_metadata.absolute_layout().padding_rect().min().x,
            "Record [C]ontent",
        )?;
        draw_pane(
            context,
            &self.pane_open,
            self.pane_metadata.absolute_layout().padding_rect().min().x,
            "Open Sub-Record [Enter]",
        )?;

        self.draw_header(
            context,
            self.placeholder_header.absolute_layout().padding_rect(),
        )?;

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
