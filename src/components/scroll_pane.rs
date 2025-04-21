use color_eyre::eyre::Result;
use taffy::{Dimension, Overflow};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    component::{Component, ComponentId, DefaultDrawableComponent, Drawable},
    layout::TaffyNodeData,
};

#[derive(Debug)]
pub struct ScrollPane<T: DefaultDrawableComponent> {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    pub child: T,
}

impl<T> ScrollPane<T>
where
    T: DefaultDrawableComponent,
{
    pub fn new(id: ComponentId, _action_tx: &UnboundedSender<Action>, child: T) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            taffy_node_data: TaffyNodeData::new(taffy::Style {
                overflow: taffy::Point {
                    x: Overflow::Hidden,
                    y: Overflow::Hidden,
                },
                ..Default::default()
            }),
            child,
        }
    }
}

impl<T> Component for ScrollPane<T>
where
    T: DefaultDrawableComponent,
{
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
        vec![&self.child]
    }

    fn get_children_mut(&mut self) -> Vec<&mut dyn Component> {
        vec![&mut self.child]
    }
}

impl<T> Drawable for ScrollPane<T>
where
    T: DefaultDrawableComponent,
{
    type Args<'a>
        = ()
    where
        Self: 'a;

    fn draw<'a>(
        &self,
        context: &mut crate::component::DrawContext,
        extra_args: Self::Args<'a>,
    ) -> Result<()>
    where
        Self: 'a,
    {
        self.child.default_draw(context)
    }
}
