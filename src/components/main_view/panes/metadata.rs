use core::option::Option::Some;
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

use color_eyre::eyre::Result;
use ratatui::prelude::*;
use ratatui::widgets::{Row, Table};
use rrr::crypto::encryption::EncryptionAlgorithm;
use taffy::BoxSizing;
use taffy::prelude::length;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::{Action, ComponentMessage};
use crate::component::{Component, ComponentId, DrawContext, Drawable};
use crate::components::main_view::{MainState, MainView};
use crate::layout::TaffyNodeData;

#[derive(Debug)]
pub struct PaneMetadata {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    main_state: Rc<RefCell<MainState>>,
}

impl PaneMetadata {
    pub fn new(
        id: ComponentId,
        _action_tx: &UnboundedSender<Action>,
        main_state: &Rc<RefCell<MainState>>,
    ) -> Result<Self> {
        Ok(Self {
            id,
            taffy_node_data: TaffyNodeData::new(taffy::Style {
                box_sizing: BoxSizing::BorderBox,
                // This padding is for the pane's title.
                padding: taffy::Rect {
                    top: length(1.0),
                    ..taffy::Rect::zero()
                },
                ..Default::default()
            }),
            main_state: main_state.clone(),
        })
    }
}

impl Component for PaneMetadata {
    fn update(&mut self, message: ComponentMessage) -> Result<Option<Action>> {
        match message {
            ComponentMessage::RecordOpen {
                hashed_record_key: _,
                read_result: Some(_),
            } => {
                // TODO
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
        let area = self.taffy_node_data.absolute_layout().padding_rect();
        let (area_title, area_content) = MainView::pane_areas(area, 0);

        context.draw_widget(&Span::raw("Record [M]etadata"), area_title);

        if let Some(opened_record) = self.main_state.borrow().opened_record.as_ref() {
            let metadata_table = Table::new(
                itertools::chain![
                    opened_record
                        .record
                        .metadata
                        .iter_with_semantic_keys()
                        .map(|(key, value)| crate::cbor::record_metadata_to_row(key, value)),
                    [
                        Row::new(vec![
                            Cow::Borrowed("Record Nonce"),
                            format!("{}", opened_record.record.record_nonce.0).into(),
                        ]),
                        Row::new(vec![
                            Cow::Borrowed("Content Size"),
                            format!("{} bytes", opened_record.record.data.len()).into(),
                        ]),
                        Row::new(vec![
                            Cow::Borrowed("Segments"),
                            format!("{}", opened_record.record.segments.len()).into(),
                        ]),
                    ],
                    opened_record.record.segments.iter().enumerate().flat_map(
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
                                            .map(
                                                |encryption_algorithm| match encryption_algorithm {
                                                    EncryptionAlgorithm::Aes256Gcm => "AES-256-GCM",
                                                }
                                            )
                                            .unwrap_or("None")
                                    ),
                                ]),
                            ]
                        }
                    ),
                ],
                [Constraint::Length(16), Constraint::Fill(1)],
            );

            context.draw_widget(&metadata_table, area_content);
        }

        Ok(())
    }
}
