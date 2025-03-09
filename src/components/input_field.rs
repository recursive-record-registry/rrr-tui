use std::ops::Range;

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    Frame,
};

use super::{Component, ComponentId};

use crate::{action::Action, tui::Event};

#[derive(Debug, Clone, PartialEq, Default)]
struct Cursor {
    /// The position where a selection started, in bytes. Must be at a boundary of grapheme clusters.
    start: usize,
    /// The current position of the cursor, in bytes. Must be at a boundary of grapheme clusters.
    end: usize,
}

impl Cursor {
    pub fn at(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }

    pub fn minmax(&self) -> Range<usize> {
        if self.start <= self.end {
            self.start..self.end
        } else {
            self.end..self.start
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct DeleteSelectionResult {
    selection_deleted: bool,
    cursor_position: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RemoveKeyCode {
    Backspace,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CursorMoveDirection {
    Left,
    Right,
}

impl TryFrom<KeyCode> for CursorMoveDirection {
    type Error = ();

    fn try_from(value: KeyCode) -> Result<Self, Self::Error> {
        match value {
            KeyCode::Left => Ok(Self::Left),
            KeyCode::Right => Ok(Self::Right),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputField {
    id: ComponentId,
    cursor: Cursor,
    content: String,
}

impl InputField {
    pub fn new(
        id: ComponentId,
        tx: tokio::sync::mpsc::UnboundedSender<crate::action::Action>,
    ) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            cursor: Cursor::default(),
            content: String::new(),
        }
    }

    /// Deletes the current selection, returning the new cursor position, without updating the position.
    fn delete_selection(&mut self) -> DeleteSelectionResult {
        let minmax = self.cursor.minmax();

        if !minmax.is_empty() {
            self.content = format!(
                "{}{}",
                &self.content[..minmax.start],
                &self.content[minmax.end..]
            );

            DeleteSelectionResult {
                selection_deleted: true,
                cursor_position: minmax.start,
            }
        } else {
            DeleteSelectionResult {
                selection_deleted: false,
                cursor_position: minmax.end,
            }
        }
    }

    fn insert(&mut self, string: &str) {
        let result = self.delete_selection();
        self.content.insert_str(result.cursor_position, string);
        self.cursor = Cursor::at(result.cursor_position + string.len());
    }

    fn remove(&mut self, key: RemoveKeyCode) {
        let result = self.delete_selection();

        if result.selection_deleted {
            self.cursor = Cursor::at(result.cursor_position);
        } else {
            let delete_position = match key {
                RemoveKeyCode::Backspace => {
                    self.get_move_cursor_position(result.cursor_position, CursorMoveDirection::Left)
                }
                RemoveKeyCode::Delete => {
                    if result.cursor_position < self.content.len() {
                        Some(result.cursor_position)
                    } else {
                        None
                    }
                }
            };

            if let Some(delete_position) = delete_position {
                self.content.remove(delete_position);
                self.cursor = Cursor::at(delete_position);
            }
        }
    }

    fn get_move_cursor_delta(
        &self,
        position: usize,
        direction: CursorMoveDirection,
    ) -> Option<isize> {
        let (prefix, suffix) = self.content.split_at(position);
        match direction {
            CursorMoveDirection::Left => {
                prefix.chars().next_back().map(|c| -(c.len_utf8() as isize))
            }
            CursorMoveDirection::Right => suffix.chars().next().map(|c| c.len_utf8() as isize),
        }
    }

    fn get_move_cursor_position(
        &self,
        position: usize,
        direction: CursorMoveDirection,
    ) -> Option<usize> {
        self.get_move_cursor_delta(position, direction)
            .map(|delta| (position as isize + delta) as usize)
    }
}

impl Component for InputField {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        Ok(None)
    }

    fn handle_event(&mut self, event: Event) -> Result<Option<Action>> {
        Ok(match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char(character),
                kind: KeyEventKind::Press | KeyEventKind::Repeat,
                ..
            }) => {
                let string = character.to_string();
                self.insert(&string);
                Some(Action::Render)
            }
            Event::Key(KeyEvent {
                code: KeyCode::Backspace,
                kind: KeyEventKind::Press | KeyEventKind::Repeat,
                ..
            }) => {
                self.remove(RemoveKeyCode::Backspace);
                Some(Action::Render)
            }
            Event::Key(KeyEvent {
                code: KeyCode::Delete,
                kind: KeyEventKind::Press | KeyEventKind::Repeat,
                ..
            }) => {
                self.remove(RemoveKeyCode::Delete);
                Some(Action::Render)
            }
            Event::Key(KeyEvent {
                code: code @ (KeyCode::Left | KeyCode::Right),
                kind: KeyEventKind::Press | KeyEventKind::Repeat,
                modifiers,
                ..
            }) => {
                if modifiers.contains(KeyModifiers::SHIFT) {
                    let direction =
                        CursorMoveDirection::try_from(code).unwrap_or_else(|()| unreachable!());

                    if let Some(new_position) =
                        self.get_move_cursor_position(self.cursor.end, direction)
                    {
                        self.cursor.end = new_position;
                    }
                } else {
                    let minmax = self.cursor.minmax();

                    if minmax.is_empty() {
                        let direction =
                            CursorMoveDirection::try_from(code).unwrap_or_else(|()| unreachable!());

                        if let Some(new_position) =
                            self.get_move_cursor_position(minmax.start, direction)
                        {
                            self.cursor = Cursor::at(new_position);
                        }
                    } else {
                        self.cursor = Cursor::at(match code {
                            KeyCode::Left => minmax.start,
                            KeyCode::Right => minmax.end,
                            _ => unreachable!(),
                        })
                    }
                }

                Some(Action::Render)
            }
            Event::Paste(paste_string) => {
                self.insert(&paste_string);
                Some(Action::Render)
            }
            _ => None,
        })
    }

    fn draw(&self, frame: &mut Frame, mut area: Rect) -> Result<()> {
        if area.area() == 0 {
            return Ok(());
        }

        area.height = 1;

        let minmax = self.cursor.minmax();

        if minmax.is_empty() {
            let mut spans = vec![Span::styled(&self.content[..minmax.start], Style::new())];
            if minmax.start < self.content.len() {
                let mut chars = self.content[minmax.start..].chars();
                let cursor_char = chars.next().into_iter().collect::<String>();
                let remaining = chars.collect::<String>();
                spans.extend([
                    Span::styled(cursor_char, Style::new().reversed()),
                    Span::styled(remaining, Style::new()),
                ]);
            } else {
                spans.push(Span::styled(" ", Style::new().reversed()));
            }
            frame.render_widget(Line::from(spans), area);
        } else {
            let spans = vec![
                Span::styled(&self.content[..minmax.start], Style::new()),
                Span::styled(
                    &self.content[minmax.start..minmax.end],
                    Style::new().white().bg(Color::Rgb(0x5F, 0x5F, 0x5F)),
                ),
                Span::styled(&self.content[minmax.end..], Style::new()),
            ];
            frame.render_widget(Line::from(spans), area);
        }

        Ok(())
    }

    fn get_id(&self) -> ComponentId {
        self.id
    }

    fn get_accessibility_node(&self) -> Result<accesskit::Node> {
        todo!()
    }
}
