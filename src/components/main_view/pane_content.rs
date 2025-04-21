use core::option::Option::Some;
use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use color_eyre::eyre::{Result, eyre};
use color_eyre::owo_colors::OwoColorize;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
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
use taffy::prelude::{auto, percent};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{Instrument, debug, info_span};

use crate::action::{Action, ComponentMessage};
use crate::args::Args;
use crate::color::{ColorOklch, TextColor};
use crate::component::{
    Component, ComponentExt, ComponentId, DefaultDrawable, DrawContext, Drawable,
    HandleEventSuccess,
};
use crate::components::button::Button;
use crate::components::input_field::InputField;
use crate::components::open_status::{Animation, OpenStatus, SpinnerContent};
use crate::components::radio_array::RadioArray;
use crate::components::scroll_pane::ScrollPane;
use crate::components::styled_widget::StyledWidget;
use crate::components::text_block::TextBlock;
use crate::env::PROJECT_VERSION;
use crate::error;
use crate::layout::{LayoutExt, TaffyNodeData, ext::ratatui::SizeExt};
use crate::tui::Event;

use super::{Encoding, MainState, MainView};

#[derive(Debug)]
pub struct PaneContent {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    action_tx: UnboundedSender<Action>,
    main_state: Rc<RefCell<MainState>>,
    // content: ScrollPane<StyledWidget<Text<'static>>>,
    content: ScrollPane<TextBlock>,
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
                TextBlock::new(ComponentId::new(), action_tx).with_style(|style| taffy::Style {
                    size: taffy::Size {
                        width: percent(1.0),
                        height: auto(),
                    },
                    ..style
                }),
            )
            .with_style(|style| taffy::Style {
                size: taffy::Size {
                    width: percent(1.0),
                    height: auto(),
                },
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
                self.content.child.text = data_string.into_owned().into();
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

        context
            .frame()
            .render_widget(Span::raw("Record [C]ontent"), area_title);

        self.content.default_draw(context)?;

        Ok(())
    }
}
