use core::option::Option::Some;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use color_eyre::eyre::Result;
use taffy::prelude::{max_content, percent};
use taffy::{BoxSizing, Display};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::{Action, ComponentMessage};
use crate::animation::BlendAnimationDescriptor;
use crate::color::{Blended, ColorU8Rgb};
use crate::component::{Component, ComponentExt, ComponentId, DrawContext, Drawable};
use crate::components::main_view::MainState;
use crate::components::scroll_pane::ScrollPane;
use crate::components::text_block::TextBlock;
use crate::layout::TaffyNodeData;

#[derive(Debug)]
pub struct PaneContent {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    // main_state: Rc<RefCell<MainState>>,
    content: ScrollPane<TextBlock>,
}

impl PaneContent {
    pub fn new(
        id: ComponentId,
        action_tx: &UnboundedSender<Action>,
        _main_state: &Rc<RefCell<MainState>>,
    ) -> Result<Self> {
        Ok(Self {
            id,
            taffy_node_data: TaffyNodeData::new(taffy::Style {
                box_sizing: BoxSizing::BorderBox,
                ..Default::default()
            }),
            content: ScrollPane::new(
                ComponentId::new(),
                action_tx,
                TextBlock::new(ComponentId::new(), action_tx),
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
            // main_state: main_state.clone(),
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
                self.content.child.set_text(data_string.into_owned().into());
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

impl Drawable for PaneContent {
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
