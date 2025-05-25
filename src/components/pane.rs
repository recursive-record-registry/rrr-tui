use color_eyre::eyre::Result;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    component::{
        Component, ComponentId, DefaultDrawableComponent, Drawable,
    },
    layout::TaffyNodeData,
};

enum ScrollAxis {
    Horizontal,
    Vertical,
}

enum ScrollDirection {
    Backward,
    Forward,
}

#[derive(Debug)]
pub struct Pane {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    // TODO: Consider using `tuple_list`
    pub children: Vec<Box<dyn DefaultDrawableComponent>>,
}

impl Pane {
    pub fn new(id: ComponentId, _action_tx: &UnboundedSender<Action>) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            taffy_node_data: TaffyNodeData::default(),
            children: vec![],
        }
    }

    pub fn with_child(mut self, child: impl DefaultDrawableComponent + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl Component for Pane {
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
        self.children
            .iter()
            .map(|c| c.as_ref() as &dyn Component)
            .collect()
    }

    fn get_children_mut(&mut self) -> Vec<&mut dyn Component> {
        self.children
            .iter_mut()
            .map(|c| c.as_mut() as &mut dyn Component)
            .collect()
    }
}

impl Drawable for Pane {
    type Args<'a>
        = ()
    where
        Self: 'a;

    fn draw<'a>(
        &self,
        context: &mut crate::component::DrawContext,
        (): Self::Args<'a>,
    ) -> Result<()>
    where
        Self: 'a,
    {
        for child in &self.children {
            context.draw_component(child.as_ref())?;
        }

        Ok(())
    }
}
