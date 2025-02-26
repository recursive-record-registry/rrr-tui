use color_eyre::eyre::Result;
use ratatui::prelude::*;
use ratatui::widgets::canvas::Label;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;
use tracing::{debug, info};

use crate::config::PROJECT_VERSION;

use super::input_field::InputField;
use super::Component;

#[derive(Clone)]
pub struct LineSpacer {
    direction: Direction,
    begin: &'static str,
    inner: &'static str,
    end: &'static str,
    merged: &'static str,
}

impl Widget for LineSpacer {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        debug_assert!(
            (self.direction == Direction::Horizontal || area.width == 1)
                && (self.direction == Direction::Vertical || area.height == 1)
        );

        let start_position = area.as_position();

        if area.width == 0 || area.height == 0 {
            return;
        }

        if area.width <= 1 && area.height <= 1 {
            buf[start_position].set_symbol(self.merged);
            return;
        }

        buf[start_position].set_symbol(self.begin);

        match self.direction {
            Direction::Horizontal => {
                let end_position: Position =
                    (start_position.x + area.width - 1, start_position.y).into();
                buf[end_position].set_symbol(self.end);
                for x in (start_position.x + 1)..end_position.x {
                    let position = (x, start_position.y);
                    buf[position].set_symbol(self.inner);
                }
            }
            Direction::Vertical => {
                let end_position: Position =
                    (start_position.x, start_position.y + area.height - 1).into();
                buf[end_position].set_symbol(self.end);
                for y in (start_position.y + 1)..end_position.y {
                    let position = (start_position.x, y);
                    buf[position].set_symbol(self.inner);
                }
            }
        }
    }
}

pub struct MainView {
    record_name_field: InputField,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            record_name_field: Default::default(),
        }
    }
}

impl Component for MainView {
    fn handle_events(
        &mut self,
        event: Option<crate::tui::Event>,
    ) -> Result<Option<crate::action::Action>> {
        self.record_name_field.handle_events(event.clone())
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let spacer_horizontal = LineSpacer {
            direction: Direction::Horizontal,
            begin: symbols::line::HORIZONTAL,
            inner: symbols::line::HORIZONTAL,
            end: symbols::line::HORIZONTAL,
            merged: symbols::line::HORIZONTAL,
        };
        let spacer_horizontal_forked = LineSpacer {
            direction: Direction::Horizontal,
            begin: symbols::line::VERTICAL_RIGHT,
            inner: symbols::line::HORIZONTAL,
            end: symbols::line::VERTICAL_LEFT,
            merged: symbols::line::VERTICAL,
        };
        let spacer_vertical = LineSpacer {
            direction: Direction::Vertical,
            begin: symbols::line::VERTICAL,
            inner: symbols::line::VERTICAL,
            end: symbols::line::VERTICAL,
            merged: symbols::line::VERTICAL,
        };
        let spacer_vertical_forked = LineSpacer {
            direction: Direction::Vertical,
            begin: symbols::line::HORIZONTAL_DOWN,
            inner: symbols::line::VERTICAL,
            end: symbols::line::HORIZONTAL_UP,
            merged: symbols::line::HORIZONTAL,
        };
        let block_horizontal = Block::new().borders(Borders::TOP | Borders::BOTTOM);
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(7),
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
            ]);
        let [area_header, area_top, area_content, area_bottom_spacer, area_bottom, area_footer] =
            layout.areas(area);
        let layout_top = Layout::default()
            .direction(Direction::Horizontal)
            .spacing(1)
            .constraints([
                Constraint::Length(8),
                Constraint::Fill(1),
                Constraint::Length(16),
            ]);
        let [area_tree, area_metadata, area_overview] = layout_top.areas(area_top);
        let [_, area_top_spacer_0, area_top_spacer_1, _] = layout_top.spacers(area_top);
        let area_content_title = Rect {
            x: area_metadata.x,
            y: (area_top.y + area_top.height).saturating_sub(1),
            width: area_top.width.saturating_sub(area_metadata.x),
            height: 1,
        };
        let area_bottom_title = Rect {
            y: area_bottom_spacer.y,
            ..area_content_title
        };

        frame.render_widget(spacer_vertical_forked.clone(), area_top_spacer_0);
        frame.render_widget(spacer_vertical_forked.clone(), area_top_spacer_1);
        frame.render_widget(spacer_horizontal.clone(), area_footer);
        frame.render_widget(spacer_horizontal.clone(), area_bottom_spacer);
        frame.render_widget(block_horizontal.clone().title("[T]ree"), area_tree);
        frame.render_widget(
            block_horizontal.clone().title("Record [M]etadata"),
            area_metadata,
        );
        frame.render_widget(block_horizontal.clone().title("[O]verview"), area_overview);
        frame.render_widget(Span::raw("Record [C]ontent"), area_content_title);
        frame.render_widget(Span::raw("Open Sub-Record [Enter]"), area_bottom_title);
        frame.render_widget(
            Span::raw(format!("RRR TUI v{}", *PROJECT_VERSION)),
            area_header,
        );
        frame.render_widget(Text::raw("Lorem ipsum dolor sit amet…"), area_content);
        let layout_bottom_lines = Layout::default()
            .direction(Direction::Horizontal)
            .spacing(1)
            .constraints([Constraint::Length(11), Constraint::Fill(1)]);
        let [area_record_name, area_encoding] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .areas(area_bottom);
        let [area_record_name_label, area_record_name_field] =
            layout_bottom_lines.areas(area_record_name);
        let [area_encoding_label, area_encoding_field] = layout_bottom_lines.areas(area_encoding);

        frame.render_widget(Span::raw("Record Name"), area_record_name_label);
        self.record_name_field
            .draw(frame, area_record_name_field)
            .unwrap();
        frame.render_widget(Span::raw("Encoding"), area_encoding_label);
        frame.render_widget(Span::raw("Field"), area_encoding_field);

        Ok(())
    }
}
