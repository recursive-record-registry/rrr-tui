use core::option::Option::Some;
use std::cell::RefCell;
use std::rc::Rc;

use color_eyre::eyre::Result;
use ratatui::prelude::*;
use taffy::BoxSizing;
use taffy::prelude::length;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::{Action, ComponentMessage};
use crate::component::{Component, ComponentId, DrawContext, Drawable};
use crate::components::main_view::{MainState, MainView};
use crate::layout::TaffyNodeData;

#[derive(Debug)]
pub struct PaneOverview {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    // main_state: Rc<RefCell<MainState>>,
    // content: ScrollPane<TextBlock>,
}

impl PaneOverview {
    pub fn new(
        id: ComponentId,
        _action_tx: &UnboundedSender<Action>,
        _main_state: &Rc<RefCell<MainState>>,
    ) -> Result<Self> {
        Ok(Self {
            id,
            taffy_node_data: TaffyNodeData::new(taffy::Style {
                box_sizing: BoxSizing::BorderBox,
                ..Default::default()
            }),
            // main_state: main_state.clone(),
        })
    }
}

impl Component for PaneOverview {
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

impl Drawable for PaneOverview {
    type Args<'a>
        = ()
    where
        Self: 'a;

    fn draw<'a>(&self, context: &mut DrawContext, (): Self::Args<'a>) -> Result<()>
    where
        Self: 'a,
    {
        // TODO
        //context.draw_component_with(&self.content, ())?;

        Ok(())
    }
}
