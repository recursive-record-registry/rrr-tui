use std::borrow::Cow;

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::Size,
    style::{Style, Stylize},
    text::{Line, Span},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::{Action, ComponentMessage},
    component::{Component, ComponentExt, ComponentId, DrawContext, Drawable, HandleEventSuccess},
    geometry::ext::ratatui::SizeExt,
    layout::TaffyNodeData,
    tui::Event,
};

#[derive(Debug, Clone)]
pub struct Checkbox {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    label: Cow<'static, str>,
    pub checked: bool,
    string_checked: Cow<'static, str>,
    string_unchecked: Cow<'static, str>,
    action_tx: UnboundedSender<Action>,
}

impl Checkbox {
    pub fn new(
        id: ComponentId,
        tx: &UnboundedSender<Action>,
        label: Cow<'static, str>,
        checked: bool,
    ) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            taffy_node_data: Default::default(),
            label,
            checked,
            string_checked: "[x]".into(),
            string_unchecked: "[ ]".into(),
            action_tx: tx.clone(),
        }
    }

    pub fn with_checkbox(self, checked: Cow<'static, str>, unchecked: Cow<'static, str>) -> Self {
        Self {
            string_checked: checked,
            string_unchecked: unchecked,
            ..self
        }
    }

    pub fn size(&self) -> Size {
        Size::new(
            1 + Line::from_iter([
                Span::raw(if self.checked {
                    self.string_checked.as_ref()
                } else {
                    self.string_unchecked.as_ref()
                }),
                Span::raw(self.label.as_ref()),
            ])
            .width() as u16,
            1,
        )
    }
}

impl Component for Checkbox {
    fn is_focusable(&self) -> bool {
        true
    }

    fn handle_event(&mut self, event: &Event) -> Result<HandleEventSuccess> {
        Ok(match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char(' '),
                kind: KeyEventKind::Press | KeyEventKind::Repeat,
                ..
            }) => {
                self.checked = !self.checked;
                self.action_tx.send(Action::BroadcastMessage(
                    ComponentMessage::OnCheckboxToggle {
                        id: self.id,
                        new_value: self.checked,
                    },
                ))?;
                HandleEventSuccess::handled().with_action(Action::Render)
            }
            _ => HandleEventSuccess::unhandled(),
        })
    }

    fn get_id(&self) -> ComponentId {
        self.id
    }

    fn get_accessibility_node(&self) -> Result<accesskit::Node> {
        todo!()
    }

    fn get_taffy_node_data(&self) -> &TaffyNodeData {
        &self.taffy_node_data
    }

    fn get_taffy_node_data_mut(&mut self) -> &mut TaffyNodeData {
        &mut self.taffy_node_data
    }

    fn measure(
        &self,
        _known_dimensions: taffy::Size<Option<f32>>,
        _available_space: taffy::Size<taffy::AvailableSpace>,
    ) -> taffy::Size<f32> {
        self.size().into_taffy()
    }
}

impl Drawable for Checkbox {
    type Args<'a>
        = ()
    where
        Self: 'a;

    fn draw<'a>(&self, context: &mut DrawContext, (): Self::Args<'a>) -> Result<()>
    where
        Self: 'a,
    {
        let mut area = self.absolute_layout().content_rect();

        if area.area() == 0 {
            return Ok(());
        }

        area.height = 1;
        let focused = context.focused_id() == self.id;
        let checkmark_style = if focused {
            Style::new().reversed()
        } else {
            Style::new()
        };
        let spans = [
            Span::raw(if self.checked {
                self.string_checked.as_ref()
            } else {
                self.string_unchecked.as_ref()
            })
            .style(checkmark_style),
            Span::raw(" "),
            Span::raw(self.label.as_ref()),
        ];

        context.draw_widget(&Line::from_iter(spans), area);

        Ok(())
    }
}
