use std::{fmt::Debug, ops::ControlFlow};

use color_eyre::Result;
use ratatui::{
    layout::{Direction, Layout, Rect},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use super::{checkbox::Checkbox, Component, ComponentId, Drawable, HandleEventSuccess};

use crate::{
    action::{Action, ComponentMessage},
    tui::Event,
};

#[derive(Debug, Clone)]
pub struct RadioArray<T>
where
    T: ToString + Clone + PartialEq + Debug,
{
    id: ComponentId,
    items: Vec<(T, Checkbox)>,
    checked_index: usize,
    action_tx: UnboundedSender<Action>,
    layout_direction: Direction,
}

impl<T> RadioArray<T>
where
    T: ToString + Clone + PartialEq + Debug,
{
    pub fn new(
        id: ComponentId,
        tx: &UnboundedSender<Action>,
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
            items: items
                .into_iter()
                .enumerate()
                .map(|(index, item)| {
                    let checkbox = Checkbox::new(
                        ComponentId::new(),
                        tx,
                        item.to_string().into(),
                        index == checked_index,
                    )
                    .with_checkbox("(x)".into(), "( )".into());
                    (item, checkbox)
                })
                .collect(),
            checked_index,
            action_tx: tx.clone(),
            layout_direction,
        }
    }
}

impl<T> Component for RadioArray<T>
where
    T: ToString + Clone + PartialEq + Debug,
{
    fn is_focusable(&self) -> bool {
        false
    }

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

    fn handle_event(&mut self, event: &Event) -> Result<HandleEventSuccess> {
        Ok(HandleEventSuccess::unhandled())
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

    fn get_accessibility_node(&self) -> Result<accesskit::Node> {
        todo!()
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

    fn draw<'a>(
        &self,
        frame: &mut Frame,
        area: Rect,
        focused_id: ComponentId,
        (): Self::Args<'a>,
    ) -> Result<()>
    where
        Self: 'a,
    {
        if area.area() == 0 {
            return Ok(());
        }

        let (areas, _) = Layout::new(
            self.layout_direction,
            self.items.iter().map(|(_, checkbox)| {
                let size = checkbox.size();
                match self.layout_direction {
                    Direction::Horizontal => size.width,
                    Direction::Vertical => size.height,
                }
            }),
        )
        .spacing(match self.layout_direction {
            Direction::Horizontal => 2,
            Direction::Vertical => 0,
        })
        .split_with_spacers(area);

        for ((_, checkbox), checkbox_area) in self.items.iter().zip(areas.iter()) {
            checkbox.draw(frame, *checkbox_area, focused_id, ())?;
        }

        Ok(())
    }
}
