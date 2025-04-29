use std::{fmt::Debug, ops::ControlFlow};

use color_eyre::Result;
use ratatui::layout::Direction;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::{Action, ComponentMessage},
    component::{Component, ComponentExt, ComponentId, DrawContext, Drawable},
    components::checkbox::Checkbox,
    layout::TaffyNodeData,
};

#[derive(Debug, Clone)]
pub struct RadioArray<T>
where
    T: ToString + Clone + PartialEq + Debug,
{
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    items: Vec<(T, Checkbox)>,
    checked_index: usize,
    layout_direction: Direction,
}

impl<T> RadioArray<T>
where
    T: ToString + Clone + PartialEq + Debug,
{
    pub fn new(
        id: ComponentId,
        action_tx: &UnboundedSender<Action>,
        items: Vec<T>,
        checked_item: &T,
        layout_direction: Direction,
    ) -> Self
    where
        Self: Sized,
    {
        let checked_index = items
            .iter()
            .enumerate()
            .find(|(_, item)| checked_item == *item)
            .map(|(index, _)| index)
            .unwrap();
        Self {
            id,
            taffy_node_data: TaffyNodeData::new(taffy::Style {
                display: taffy::Display::Flex,
                flex_direction: match layout_direction {
                    Direction::Horizontal => taffy::FlexDirection::Row,
                    Direction::Vertical => taffy::FlexDirection::Column,
                },
                gap: taffy::Size {
                    width: taffy::prelude::length(2.0),
                    height: taffy::prelude::zero(),
                },
                ..Default::default()
            }),
            items: items
                .into_iter()
                .enumerate()
                .map(|(index, item)| {
                    let checkbox = Checkbox::new(
                        ComponentId::new(),
                        action_tx,
                        item.to_string().into(),
                        index == checked_index,
                    )
                    .with_checkbox("(x)".into(), "( )".into());
                    (item, checkbox)
                })
                .collect(),
            checked_index,
            layout_direction,
        }
    }

    pub fn get_checked(&self) -> &T {
        &self.items[self.checked_index].0
    }

    #[expect(unused)]
    pub fn get_checked_mut(&mut self) -> &mut T {
        &mut self.items[self.checked_index].0
    }

    #[expect(unused)]
    pub fn get_checked_entry(&self) -> &(T, Checkbox) {
        &self.items[self.checked_index]
    }

    #[expect(unused)]
    pub fn get_checked_entry_mut(&mut self) -> &mut (T, Checkbox) {
        &mut self.items[self.checked_index]
    }
}

impl<T> Component for RadioArray<T>
where
    T: ToString + Clone + PartialEq + Debug,
{
    fn update(&mut self, message: ComponentMessage) -> Result<Option<Action>> {
        Ok(match message {
            ComponentMessage::OnCheckboxToggle { id, new_value: _ } => {
                for (index, (_, checkbox)) in self.items.iter_mut().enumerate() {
                    if checkbox.get_id() == id {
                        checkbox.checked = true;
                        self.checked_index = index;
                    } else {
                        checkbox.checked = false;
                    }
                }

                None
            }
            _ => None,
        })
    }

    fn get_id(&self) -> ComponentId {
        self.id
    }

    fn get_children(&self) -> Vec<&dyn Component> {
        self.items
            .iter()
            .map(|(_, checkbox)| checkbox as &dyn Component)
            .collect()
    }

    fn get_children_mut(&mut self) -> Vec<&mut dyn Component> {
        self.items
            .iter_mut()
            .map(|(_, checkbox)| checkbox as &mut dyn Component)
            .collect()
    }

    fn for_each_child<'a>(
        &'a self,
        f: &mut dyn FnMut(&'a dyn Component) -> std::ops::ControlFlow<()>,
    ) -> std::ops::ControlFlow<()> {
        for (_, checkbox) in &self.items {
            (f)(checkbox)?;
        }

        ControlFlow::Continue(())
    }

    fn for_each_child_mut<'a>(
        &'a mut self,
        f: &mut dyn FnMut(&'a mut dyn Component) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        for (_, checkbox) in &mut self.items {
            (f)(checkbox)?;
        }

        ControlFlow::Continue(())
    }

    fn get_taffy_node_data(&self) -> &TaffyNodeData {
        &self.taffy_node_data
    }

    fn get_taffy_node_data_mut(&mut self) -> &mut TaffyNodeData {
        &mut self.taffy_node_data
    }
}

impl<T> Drawable for RadioArray<T>
where
    T: ToString + Clone + PartialEq + Debug,
{
    type Args<'a>
        = ()
    where
        Self: 'a;

    fn draw<'a>(&self, context: &mut DrawContext, (): Self::Args<'a>) -> Result<()>
    where
        Self: 'a,
    {
        let area = self.absolute_layout().content_rect();
        if area.area() == 0 {
            return Ok(());
        }

        for (_, checkbox) in self.items.iter() {
            context.draw_component_with(checkbox, ())?;
        }

        Ok(())
    }
}
