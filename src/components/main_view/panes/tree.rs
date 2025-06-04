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
pub struct PaneTree {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    // main_state: Rc<RefCell<MainState>>,
    // content: ScrollPane<TextBlock>,
}

impl PaneTree {
    pub fn new(
        id: ComponentId,
        _action_tx: &UnboundedSender<Action>,
        _main_state: &Rc<RefCell<MainState>>,
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
            // main_state: main_state.clone(),
        })
    }
}

impl Component for PaneTree {
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

impl Drawable for PaneTree {
    type Args<'a>
        = ()
    where
        Self: 'a;

    fn draw<'a>(&self, context: &mut DrawContext, (): Self::Args<'a>) -> Result<()>
    where
        Self: 'a,
    {
        let area = self.taffy_node_data.absolute_layout().padding_rect();
        let (area_title, _) = MainView::pane_areas(area, 0);

        context.draw_widget(&Span::raw("[T]ree"), area_title);

        // TODO
        //context.draw_component_with(&self.content, ())?;

        Ok(())
    }
}
