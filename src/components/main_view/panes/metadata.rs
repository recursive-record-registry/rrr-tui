use core::option::Option::Some;
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use color_eyre::eyre::Result;
use ratatui::prelude::*;
use ratatui::widgets::Row;
use rrr::crypto::encryption::EncryptionAlgorithm;
use taffy::prelude::{length, max_content, percent, zero};
use taffy::{BoxSizing, Display};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::{Action, ComponentMessage};
use crate::animation::BlendAnimationDescriptor;
use crate::color::{Blended, ColorU8Rgb};
use crate::component::{Component, ComponentExt, ComponentId, DrawContext, Drawable};
use crate::components::main_view::{MainState, MainView};
use crate::components::scroll_pane::ScrollPane;
use crate::components::styled_widget::{StyledWidget, TableProxy};
use crate::layout::TaffyNodeData;

#[derive(Debug)]
pub struct PaneMetadata {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    main_state: Rc<RefCell<MainState>>,
    content: ScrollPane<StyledWidget<TableProxy<'static>>>,
}

impl PaneMetadata {
    pub fn new(
        id: ComponentId,
        action_tx: &UnboundedSender<Action>,
        main_state: &Rc<RefCell<MainState>>,
    ) -> Result<Self> {
        Ok(Self {
            id,
            taffy_node_data: TaffyNodeData::new(taffy::Style {
                box_sizing: BoxSizing::BorderBox,
                ..Default::default()
            }),
            main_state: main_state.clone(),
            content: ScrollPane::new(
                ComponentId::new(),
                action_tx,
                StyledWidget::<TableProxy<'static>>::new(
                    ComponentId::new(),
                    action_tx,
                    TableProxy::default(),
                ),
            )
            .with_animation(BlendAnimationDescriptor {
                easing_function: easing_function::easings::EaseInOutCubic.into(),
                start_delay: Duration::from_secs_f32(0.25),
                duration: Duration::from_secs_f32(0.75),
            })
            .with_rail_color(Blended::new(ColorU8Rgb::new_f32(1.0, 1.0, 1.0), 0.25))
            .with_bar_color(Blended::new(ColorU8Rgb::new_f32(1.0, 1.0, 1.0), 1.0))
            .with_style(|style| taffy::Style {
                box_sizing: BoxSizing::BorderBox,
                size: taffy::Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                max_size: percent(1.0),
                min_size: percent(1.0),
                // Unconstrain the height of the child component.
                display: Display::Grid,
                grid_template_rows: vec![max_content()],
                grid_template_columns: vec![percent(1.0)],
                ..style
            }),
        })
    }
}

impl Component for PaneMetadata {
    fn update(&mut self, message: ComponentMessage) -> Result<Option<Action>> {
        match message {
            ComponentMessage::RecordOpen {
                hashed_record_key: _,
                read_result: Some(opened_record),
            } => {
                self.content.child.widget =
                    TableProxy {
                        rows: itertools::chain![
                            opened_record.record.metadata.iter_with_semantic_keys().map(
                                |(key, value)| crate::cbor::record_metadata_to_row(key, value)
                            ),
                            [
                                Row::new(vec![
                                    Cow::Borrowed("Record Nonce"),
                                    format!("{}", opened_record.record_nonce.0).into(),
                                ]),
                                Row::new(vec![
                                    Cow::Borrowed("Content Size"),
                                    format!("{} bytes", opened_record.record.data.len()).into(),
                                ]),
                                Row::new(vec![
                                    Cow::Borrowed("Segments"),
                                    format!("{}", opened_record.segments.len()).into(),
                                ]),
                            ],
                            opened_record.segments.iter().enumerate().flat_map(
                                |(mut index, segment)| {
                                    index += 1;
                                    [
                                        Row::new(vec![
                                            format!("Segment #{} File", index),
                                            format!("{}", segment.fragment_file_name),
                                        ]),
                                        Row::new(vec![
                                            format!("Segment #{} Encryption", index),
                                            format!(
                                                "{}",
                                                segment
                                                    .fragment_encryption_algorithm
                                                    .map(|encryption_algorithm| {
                                                        match encryption_algorithm {
                                                            EncryptionAlgorithm::Aes256Gcm => {
                                                                "AES-256-GCM"
                                                            }
                                                        }
                                                    })
                                                    .unwrap_or("None")
                                            ),
                                        ]),
                                    ]
                                }
                            ),
                        ]
                        .collect(),
                        constraints: [Constraint::Length(16), Constraint::Fill(1)].into(),
                    };
                self.content.child.mark_cached_layout_dirty();
                Ok(Some(Action::Render))
            }
            _ => Ok(None),
        }
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
        vec![&self.content]
    }

    fn get_children_mut(&mut self) -> Vec<&mut dyn Component> {
        vec![&mut self.content]
    }
}

impl Drawable for PaneMetadata {
    type Args<'a>
        = ()
    where
        Self: 'a;

    fn draw<'a>(&self, context: &mut DrawContext, (): Self::Args<'a>) -> Result<()>
    where
        Self: 'a,
    {
        context.draw_component(&self.content)?;

        Ok(())
    }
}
