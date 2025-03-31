use std::fmt::Display;

use color_eyre::eyre::{eyre, Result};
use ratatui::prelude::*;
use ratatui::widgets::Table;
use ratatui::Frame;
use rrr::record::{HashRecordPath, RecordPath, RecordReadVersionSuccess};
use rrr::registry::Registry;
use rrr::utils::fd_lock::ReadLock;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::{Action, ComponentMessage};
use crate::args::Args;
use crate::env::PROJECT_VERSION;
use crate::tui::Event;

use super::input_field::InputField;
use super::radio_array::RadioArray;
use super::{Component, ComponentId, HandleEventSuccess};

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

#[derive(Debug, Clone, PartialEq, Eq)]
enum Encoding {
    Utf8,
    Hex,
}

impl Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Utf8 => write!(f, "UTF-8"),
            Self::Hex => write!(f, "Hexadecimal Byte String"),
        }
    }
}

#[derive(Debug)]
struct OpenedRecord {
    record: RecordReadVersionSuccess,
}

#[derive(Debug)]
pub struct MainView {
    id: ComponentId,
    record_name_field: InputField,
    encoding_radio_array: RadioArray<Encoding>,
    registry: Registry<ReadLock>,
    opened_record: OpenedRecord,
}

impl MainView {
    pub async fn new(id: ComponentId, tx: &UnboundedSender<Action>, args: &Args) -> Result<Self>
    where
        Self: Sized,
    {
        tracing::trace!(dir=?args.registry_directory);
        let registry = Registry::open(args.registry_directory.clone())
            .await
            .unwrap();
        // tokio::spawn(
        //     async move {
        // }
        //     .instrument(info_span!("load registry task")),
        // );
        let hashed_record_key = RecordPath::default()
            .hash_record_path(&registry)
            .await
            .unwrap();
        let versions = registry
            .list_record_versions(&hashed_record_key, 4, 4)
            .await
            .unwrap();
        let latest_version = versions
            .last()
            .ok_or_else(|| eyre!("No root record versions found."))?;
        let root_record = registry
            .load_record(&hashed_record_key, latest_version.record_version, 4)
            .await?
            .ok_or_else(|| eyre!("Failed to load the latest root record version."))?;
        Ok(Self {
            id,
            record_name_field: InputField::new(ComponentId::new(), tx),
            encoding_radio_array: RadioArray::new(
                ComponentId::new(),
                tx,
                vec![Encoding::Utf8, Encoding::Hex],
                &Encoding::Utf8,
                Direction::Horizontal,
            ),
            registry,
            opened_record: OpenedRecord {
                record: root_record,
            },
        })
    }

    fn pane_areas(area: Rect, title_offset_x: u16) -> (Rect, Rect) {
        let [mut title, content] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)])
            .areas(area);

        title.x += title_offset_x;
        title.width = title.width.saturating_sub(title_offset_x);

        (title, content)
    }

    fn draw_header(
        &self,
        frame: &mut Frame,
        area_header: Rect,
        _focused_id: ComponentId,
    ) -> Result<()> {
        frame.render_widget(
            Span::raw(format!("RRR TUI v{}", *PROJECT_VERSION)),
            area_header,
        );
        Ok(())
    }

    fn draw_pane_tree(
        &self,
        frame: &mut Frame,
        area: Rect,
        _focused_id: ComponentId,
    ) -> Result<()> {
        let (area_title, _area_content) = Self::pane_areas(area, 0);
        frame.render_widget(Span::raw("[T]ree"), area_title);
        Ok(())
    }

    fn draw_pane_metadata(
        &self,
        frame: &mut Frame,
        area: Rect,
        _focused_id: ComponentId,
    ) -> Result<()> {
        let (area_title, area_content) = Self::pane_areas(area, 0);
        let metadata_table = Table::new(
            self.opened_record
                .record
                .metadata
                .iter_with_semantic_keys()
                .map(|(key, value)| crate::cbor::record_metadata_to_row(key, value)),
            [Constraint::Length(16), Constraint::Fill(1)],
        );

        frame.render_widget(Span::raw("Record [M]etadata"), area_title);
        frame.render_widget(metadata_table, area_content);

        Ok(())
    }

    fn draw_pane_overview(
        &self,
        frame: &mut Frame,
        area: Rect,
        _focused_id: ComponentId,
    ) -> Result<()> {
        let (area_title, _area_content) = Self::pane_areas(area, 0);
        frame.render_widget(Span::raw("[O]verview"), area_title);
        Ok(())
    }

    fn draw_pane_content(
        &self,
        frame: &mut Frame,
        area: Rect,
        _focused_id: ComponentId,
        title_offset_x: u16,
    ) -> Result<()> {
        let (area_title, area_content) = Self::pane_areas(area, title_offset_x);

        frame.render_widget(Span::raw("Record [C]ontent"), area_title);
        frame.render_widget(
            Text::raw(String::from_utf8_lossy(&self.opened_record.record.data)),
            area_content,
        );

        Ok(())
    }

    fn draw_pane_open(
        &self,
        frame: &mut Frame,
        area: Rect,
        focused_id: ComponentId,
        title_offset_x: u16,
    ) -> Result<()> {
        let (area_title, area_content) = Self::pane_areas(area, title_offset_x);

        frame.render_widget(Span::raw("Open Sub-Record [Enter]"), area_title);

        let layout_bottom_lines = Layout::default()
            .direction(Direction::Horizontal)
            .spacing(1)
            .constraints([Constraint::Length(11), Constraint::Fill(1)]);
        let [area_record_name, area_encoding] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .areas(area_content);
        let [area_record_name_label, area_record_name_field] =
            layout_bottom_lines.areas(area_record_name);
        let [area_encoding_label, area_encoding_field] = layout_bottom_lines.areas(area_encoding);

        frame.render_widget(Span::raw("Record Name"), area_record_name_label);
        self.record_name_field
            .draw(frame, area_record_name_field, focused_id)
            .unwrap();
        frame.render_widget(Span::raw("Encoding"), area_encoding_label);
        self.encoding_radio_array
            .draw(frame, area_encoding_field, focused_id)?;
        // let [area_encoding_utf8, area_encoding_hex] = Layout::default()
        //     .direction(Direction::Horizontal)
        //     .spacing(2)
        //     .constraints([Constraint::Length(9), Constraint::Fill(1)])
        //     .areas(area_encoding_field);
        // self.encoding_utf8_checkbox
        //     .draw(frame, area_encoding_utf8, focused_id)
        //     .unwrap();
        // self.encoding_hex_checkbox
        //     .draw(frame, area_encoding_hex, focused_id)
        //     .unwrap();
        Ok(())
    }
}

impl Component for MainView {
    fn update(&mut self, _message: ComponentMessage) -> Result<Option<crate::action::Action>> {
        Ok(None)
    }

    fn handle_event(&mut self, _event: &Event) -> Result<HandleEventSuccess> {
        Ok(HandleEventSuccess::unhandled())
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn draw(&self, frame: &mut Frame, area: Rect, focused_id: ComponentId) -> Result<()> {
        const SPACER_HORIZONTAL: LineSpacer = LineSpacer {
            direction: Direction::Horizontal,
            begin: symbols::line::HORIZONTAL,
            inner: symbols::line::HORIZONTAL,
            end: symbols::line::HORIZONTAL,
            merged: symbols::line::HORIZONTAL,
        };
        const SPACER_VERTICAL_FORKED: LineSpacer = LineSpacer {
            direction: Direction::Vertical,
            begin: symbols::line::HORIZONTAL_DOWN,
            inner: symbols::line::VERTICAL,
            end: symbols::line::HORIZONTAL_UP,
            merged: symbols::line::HORIZONTAL,
        };
        let [area_header, area_top, area_content, area_bottom, area_footer] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(7),
                Constraint::Fill(1),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .areas(area);
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

        frame.render_widget(SPACER_HORIZONTAL.clone(), area_top);
        frame.render_widget(SPACER_HORIZONTAL.clone(), area_content);
        frame.render_widget(SPACER_HORIZONTAL.clone(), area_bottom);
        frame.render_widget(SPACER_HORIZONTAL.clone(), area_footer);
        frame.render_widget(
            SPACER_VERTICAL_FORKED.clone(),
            Rect {
                height: area_top_spacer_0.height + 1,
                ..area_top_spacer_0
            },
        );
        frame.render_widget(
            SPACER_VERTICAL_FORKED.clone(),
            Rect {
                height: area_top_spacer_1.height + 1,
                ..area_top_spacer_1
            },
        );

        self.draw_pane_tree(frame, area_tree, focused_id)?;
        self.draw_pane_metadata(frame, area_metadata, focused_id)?;
        self.draw_pane_overview(frame, area_overview, focused_id)?;
        self.draw_pane_content(frame, area_content, focused_id, area_metadata.x)?;
        self.draw_pane_open(frame, area_bottom, focused_id, area_metadata.x)?;
        self.draw_header(frame, area_header, focused_id)?;

        Ok(())
    }

    fn get_id(&self) -> super::ComponentId {
        self.id
    }

    fn get_children(&self) -> Vec<&dyn Component> {
        vec![
            &self.record_name_field,
            &self.encoding_radio_array,
            // &self.encoding_utf8_checkbox,
            // &self.encoding_hex_checkbox,
        ]
    }

    fn get_children_mut(&mut self) -> Vec<&mut dyn Component> {
        vec![
            &mut self.record_name_field,
            &mut self.encoding_radio_array,
            // &mut self.encoding_utf8_checkbox,
            // &mut self.encoding_hex_checkbox,
        ]
    }

    fn get_accessibility_node(&self) -> Result<accesskit::Node> {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_children(vec![]);
        Ok(node)
    }
}
