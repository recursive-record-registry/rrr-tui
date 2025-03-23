use std::{borrow::Cow, ops::Range, sync::Arc};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use super::{Component, ComponentId};

use crate::{
    action::{Action, ComponentMessage},
    tui::Event,
};

#[derive(Debug, Clone)]
pub struct Checkbox {
    id: ComponentId,
    label: Cow<'static, str>,
    checked: bool,
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
            label,
            checked,
            action_tx: tx.clone(),
        }
    }
}

impl Component for Checkbox {
    fn is_focusable(&self) -> bool {
        true
    }

    fn update(&mut self, message: ComponentMessage) -> Result<Option<Action>> {
        Ok(None)
    }

    fn handle_event(&mut self, event: Event) -> Result<Option<Action>> {
        Ok(match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char(' '),
                kind: KeyEventKind::Press | KeyEventKind::Repeat,
                ..
            }) => {
                self.checked = !self.checked;
                Some(Action::Render)
            }
            _ => None,
        })
    }

    fn draw(&self, frame: &mut Frame, mut area: Rect, focused_id: ComponentId) -> Result<()> {
        if area.area() == 0 {
            return Ok(());
        }

        area.height = 1;
        let focused = focused_id == self.id;
        let checkmark_style = if focused {
            Style::new().reversed()
        } else {
            Style::new()
        };
        let spans = [
            Span::raw("["),
            Span::raw(if self.checked { "x" } else { " " }).style(checkmark_style),
            Span::raw("] "),
            Span::raw(self.label.as_ref()),
        ];

        frame.render_widget(Line::from_iter(spans), area);

        // if focused {
        //     let minmax = self.cursor.minmax();

        //     if minmax.is_empty() {
        //         let mut spans = vec![Span::styled(&self.content[..minmax.start], Style::new())];
        //         if minmax.start < self.content.len() {
        //             let mut chars = self.content[minmax.start..].chars();
        //             let cursor_char = chars.next().into_iter().collect::<String>();
        //             let remaining = chars.collect::<String>();
        //             spans.extend([
        //                 Span::styled(cursor_char, Style::new().reversed()),
        //                 Span::styled(remaining, Style::new()),
        //             ]);
        //         } else {
        //             spans.push(Span::styled(" ", Style::new().reversed()));
        //         }
        //         frame.render_widget(Line::from(spans), area);
        //     } else {
        //         let spans = vec![
        //             Span::styled(&self.content[..minmax.start], Style::new()).width(),
        //             Span::styled(
        //                 &self.content[minmax.start..minmax.end],
        //                 Style::new().white().bg(Color::Rgb(0x5F, 0x5F, 0x5F)),
        //             ),
        //             Span::styled(&self.content[minmax.end..], Style::new()),
        //         ];
        //         frame.render_widget(Line::from(spans), area);
        //     }
        // } else {
        //     frame.render_widget(Span::styled(&self.content, Style::new()), area);
        // }

        Ok(())
    }

    fn get_id(&self) -> ComponentId {
        self.id
    }

    fn get_accessibility_node(&self) -> Result<accesskit::Node> {
        todo!()
    }
}
