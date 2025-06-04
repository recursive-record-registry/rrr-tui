
use crate::{
    component::{Component, ComponentId},
    layout::TaffyNodeData,
};

#[derive(Debug)]
pub struct LayoutPlaceholder {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
}

impl LayoutPlaceholder {
    pub fn new(id: ComponentId) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            taffy_node_data: Default::default(),
        }
    }
}

impl Component for LayoutPlaceholder {
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
