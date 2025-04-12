use std::borrow::Cow;

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::{Rect, Size},
    style::{Style, Stylize},
    text::{Line, Span},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::{Action, ComponentMessage},
    color::TextColor,
    component::{Component, ComponentId, DrawContext, Drawable, HandleEventSuccess},
    rect::{LineAlignment, PlaneAlignment, RectExt},
    tui::Event,
};

#[derive(Debug, Clone)]
pub struct Button {
    pub id: ComponentId,
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
                code: KeyCode::Enter,
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
                code: KeyCode::Enter,
                kind: KeyEventKind::Release,
                ..
            })
            | Event::FocusLost => {
                self.held_down = false;
                tracing::debug!("RELEASE");
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
}

impl Drawable for Button {
    type Args<'a>
        = ()
    where
        Self: 'a;

    fn draw<'a>(&self, context: &mut DrawContext, mut area: Rect, (): Self::Args<'a>) -> Result<()>
    where
        Self: 'a,
    {
        if area.area() == 0 {
            return Ok(());
        }

        area.height = 1;
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

        context.frame().render_widget(span, span_area);
        context.frame().buffer_mut().set_style(area, text_color);

        Ok(())
    }
}
