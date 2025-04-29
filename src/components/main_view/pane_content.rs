use core::option::Option::Some;
use std::cell::RefCell;
use std::rc::Rc;

use color_eyre::eyre::Result;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use taffy::prelude::{auto, length, percent, zero};
use taffy::{Display, FlexDirection};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::{Action, ComponentMessage};
use crate::component::{Component, ComponentExt, ComponentId, DrawContext, Drawable};
use crate::components::pane::Pane;
use crate::components::scroll_pane::ScrollPane;
use crate::components::styled_widget::StyledWidget;
use crate::components::text_block::TextBlock;
use crate::layout::TaffyNodeData;

use super::{MainState, MainView};

#[derive(Debug)]
pub struct PaneContent {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    action_tx: UnboundedSender<Action>,
    main_state: Rc<RefCell<MainState>>,
    // content: ScrollPane<StyledWidget<Text<'static>>>,
    // content: ScrollPane<TextBlock>,
    content: ScrollPane<Pane>,
}

impl PaneContent {
    pub fn new(
        id: ComponentId,
        action_tx: &UnboundedSender<Action>,
        main_state: &Rc<RefCell<MainState>>,
    ) -> Result<Self> {
        Ok(Self {
            id,
            taffy_node_data: TaffyNodeData::new(taffy::Style {
                // This padding is for the pane's title.
                padding: taffy::Rect {
                    top: taffy::prelude::length(1.0),
                    ..taffy::Rect::zero()
                },
                ..Default::default()
            }),
            action_tx: action_tx.clone(),
            content: ScrollPane::new(
                ComponentId::new(),
                action_tx,
                Pane::new(ComponentId::new(), action_tx)
                    .with_style(|style| taffy::Style {
                        display: Display::Block,
                        // display: Display::Flex,
                        // flex_direction: FlexDirection::Column,
                        size: taffy::Size {
                            width: auto(),
                            height: length(21.0), // TODO: compute
                        },
                        ..style
                    })
                    .with_child(StyledWidget::new(
                        ComponentId::new(),
                        action_tx,
                        ratatui::text::Text::from_iter(
                            "AA1\nAA2\nAA3\nAA4\nAA5\nAA6\nAA7\nAA8\nAA9".lines(),
                        ),
                    ))
                    .with_child(
                        ScrollPane::new(
                            ComponentId::new(),
                            action_tx,
                            TextBlock::new(ComponentId::new(), action_tx)
                                .with_text("BBB1\nBBB2\nBBB3\nBBB4\nBBB5\nBBB6\nBBBL")
                                .with_style(|style| taffy::Style {
                                    size: taffy::Size {
                                        width: length(8.0),
                                        height: length(7.0), // TODO: compute
                                    },
                                    ..style
                                }),
                        )
                        .with_style(|style| taffy::Style {
                            size: taffy::Size {
                                width: auto(),
                                height: length(3.0),
                            },
                            // size: taffy::Size {
                            //     width: percent(1.0),
                            //     height: length(17.0),
                            // },
                            // max_size: taffy::Size {
                            //     width: percent(1.0),
                            //     height: length(17.0),
                            // },
                            // min_size: taffy::Size {
                            //     width: auto(),
                            //     height: length(3.0),
                            // },
                            ..style
                        }),
                    )
                    .with_child(StyledWidget::new(
                        ComponentId::new(),
                        action_tx,
                        ratatui::text::Text::from_iter(
                            "CCC1\nCCC2\nCCC3\nCCC4\nCCC5\nCCC6\nCCC7\nCCC8\nCCC9".lines(),
                        ),
                    )),
            )
            .with_style(|style| taffy::Style {
                size: taffy::Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                max_size: percent(1.0),
                min_size: percent(1.0),
                ..style
            }),
            main_state: main_state.clone(),
        })
    }
}

impl Component for PaneContent {
    fn update(&mut self, message: ComponentMessage) -> Result<Option<Action>> {
        match message {
            ComponentMessage::RecordOpen {
                hashed_record_key: _,
                read_result: Some(read_result),
            } => {
                let data_string = String::from_utf8_lossy(&read_result.data);
                // self.content.child.set_text(data_string.into_owned().into());
                Ok(Some(Action::Render))
            }
            _ => Ok(None),
        }
    }

    fn get_id(&self) -> ComponentId {
        self.id
    }

    fn get_children(&self) -> Vec<&dyn Component> {
        vec![&self.content]
    }

    fn get_children_mut(&mut self) -> Vec<&mut dyn Component> {
        vec![&mut self.content]
    }

    fn get_taffy_node_data(&self) -> &TaffyNodeData {
        &self.taffy_node_data
    }

    fn get_taffy_node_data_mut(&mut self) -> &mut TaffyNodeData {
        &mut self.taffy_node_data
    }
}

pub struct PaneContentArgs {
    pub title_offset_x: u16,
}

impl Drawable for PaneContent {
    type Args<'a>
        = PaneContentArgs
    where
        Self: 'a;

    fn draw<'a>(&self, context: &mut DrawContext, extra_args: Self::Args<'a>) -> Result<()>
    where
        Self: 'a,
    {
        let area = self.taffy_node_data.absolute_layout().padding_rect();
        let (area_title, area_content) = MainView::pane_areas(area, extra_args.title_offset_x);

        context.draw_widget(&Span::raw("Record [C]ontent"), area_title);
        context.draw_component_with(&self.content, ())?;

        Ok(())
    }
}
