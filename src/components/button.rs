use std::borrow::Cow;

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{layout::Size, text::Span};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::{Action, ComponentMessage},
    color::TextColor,
    component::{Component, ComponentExt, ComponentId, DrawContext, Drawable, HandleEventSuccess},
    layout::TaffyNodeData,
    rect::{LineAlignment, PlaneAlignment, RectExt},
    tui::Event,
};

#[derive(Debug, Clone)]
pub struct Button {
    pub id: ComponentId,
    pub taffy_node_data: TaffyNodeData,
    pub label: Cow<'static, str>,
    pub action_tx: UnboundedSender<Action>,
    pub text_color_unfocused: TextColor,
    pub text_color_focused: TextColor,
    pub text_color_pressed: TextColor,
    pub held_down: bool,
    /// Whether the button is to be used for form confirmation.
    pub form_confirmation: bool,
}

impl Button {
    pub fn new(id: ComponentId, tx: &UnboundedSender<Action>, label: Cow<'static, str>) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            taffy_node_data: Default::default(),
            label,
            text_color_unfocused: Default::default(),
            text_color_focused: Default::default(),
            text_color_pressed: Default::default(),
            held_down: false,
            form_confirmation: false,
            action_tx: tx.clone(),
        }
    }

    pub fn with_text_color_unfocused(self, text_color: TextColor) -> Self {
        Self {
            text_color_unfocused: text_color,
            ..self
        }
    }

    pub fn with_text_color_focused(self, text_color: TextColor) -> Self {
        Self {
            text_color_focused: text_color,
            ..self
        }
    }

    pub fn with_text_color_pressed(self, text_color: TextColor) -> Self {
        Self {
            text_color_pressed: text_color,
            ..self
        }
    }

    pub fn with_form_confirmation(self, form_confirmation: bool) -> Self {
        Self {
            form_confirmation,
            ..self
        }
    }
}

impl Component for Button {
    fn is_focusable(&self) -> bool {
        true
    }

    fn handle_event(&mut self, event: &Event) -> Result<HandleEventSuccess> {
        if self.form_confirmation {
            return Ok(HandleEventSuccess::unhandled());
        }

        Ok(match event {
            Event::Key(KeyEvent {
                code: KeyCode::Enter | KeyCode::Char(' '),
                kind: KeyEventKind::Press,
                ..
            }) => {
                self.held_down = true;
                self.action_tx
                    .send(Action::BroadcastMessage(ComponentMessage::OnButtonPress {
                        id: self.id,
                    }))?;

                HandleEventSuccess::handled().with_action(Action::Render)
            }
            Event::Key(KeyEvent {
                code: KeyCode::Enter | KeyCode::Char(' '),
                kind: KeyEventKind::Release,
                ..
            })
            | Event::FocusLost => {
                self.held_down = false;
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
        taffy::Size {
            width: Span::raw(self.label.as_ref()).width() as f32,
            height: 1.0,
        }
    }
}

impl Drawable for Button {
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

        area.set_height(1);
        let focused = context.focused_id() == self.id;
        let text_color = if self.held_down {
            &self.text_color_pressed
        } else if focused {
            &self.text_color_focused
        } else {
            &self.text_color_unfocused
        };
        let span = Span::raw(self.label.as_ref());
        let span_size = Size::new(span.width() as u16, 1);
        let span_area = area.align(span_size, PlaneAlignment::horizontal(LineAlignment::Center));

        context.draw_widget(&span, span_area);
        context.set_style(area, text_color);

        Ok(())
    }
}
