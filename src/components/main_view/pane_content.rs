use core::option::Option::Some;
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use color_eyre::eyre::{Result, eyre};
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
use crate::components::styled_widget::StyledWidget;
use crate::env::PROJECT_VERSION;
use crate::error;
use crate::layout::{LayoutExt, SizeExt, TaffyNodeData};
use crate::tui::Event;

use super::{Encoding, MainState, MainView};

#[derive(Debug)]
pub struct PaneContent {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    action_tx: UnboundedSender<Action>,
    main_state: Rc<RefCell<MainState>>,
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
            main_state: main_state.clone(),
        })
    }
}

impl Component for PaneContent {
    fn get_id(&self) -> ComponentId {
        self.id
    }

    fn get_children(&self) -> Vec<&dyn Component> {
        vec![]
    }

    fn get_children_mut(&mut self) -> Vec<&mut dyn Component> {
        vec![]
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

        if let Some(opened_record) = self.main_state.borrow().opened_record.as_ref() {
            let data_string = String::from_utf8_lossy(&opened_record.record.data);
            let lines = textwrap::wrap(
                data_string.as_ref(),
                textwrap::Options::new(area.width as usize),
            );
            context.frame().render_widget(
                Text::from_iter(lines).style(Style::default()), // TODO: Other formats
                area_content,
            );
        }

        Ok(())
    }
}
