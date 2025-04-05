use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;
use std::sync::Arc;

use color_eyre::eyre::{eyre, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::Table;
use ratatui::Frame;
use rrr::record::{
    HashRecordPath, HashedRecordKey, RecordKey, RecordName, RecordPath, RecordReadVersionSuccess,
    SuccessionNonce, RECORD_NAME_ROOT,
};
use rrr::registry::Registry;
use rrr::utils::fd_lock::ReadLock;
use rrr::utils::serde::BytesOrAscii;
use tokio::runtime::Handle;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, info_span, Instrument};

use crate::action::{Action, ComponentMessage};
use crate::args::Args;
use crate::env::PROJECT_VERSION;
use crate::tui::Event;

use super::input_field::InputField;
use super::radio_array::RadioArray;
use super::{Component, ComponentId, Drawable, HandleEventSuccess};

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

#[derive(Debug, Clone)]
struct OpenedRecord {
    hashed_record_key: HashedRecordKey,
    record: Arc<RecordReadVersionSuccess>, // Rc'd for cheaper cloning
}

#[derive(Debug, Clone)]
struct MainState {
    registry: Arc<Registry<ReadLock>>,
    opened_record: Option<OpenedRecord>,
}

impl MainState {
    async fn get_current_succession_nonce(&self) -> SuccessionNonce {
        if let Some(opened_record) = self.opened_record.as_ref() {
            // This should be a pretty brief operation.
            opened_record
                .hashed_record_key
                .derive_succession_nonce(&self.registry.config.kdf)
                .await
                .unwrap()
        } else {
            self.registry
                .config
                .kdf
                .get_root_record_predecessor_nonce()
                .clone()
        }
    }
}

#[derive(Debug)]
pub struct MainView {
    id: ComponentId,
    args: Arc<Args>,
    pane_open: PaneOpen,
    state: Rc<RefCell<MainState>>,
}

impl MainView {
    pub async fn new(
        id: ComponentId,
        tx: &UnboundedSender<Action>,
        args: &Arc<Args>,
    ) -> Result<Self>
    where
        Self: Sized,
    {
        tracing::trace!(dir=?args.registry_directory);
        let registry = Arc::new(
            Registry::open(args.registry_directory.clone())
                .await
                .unwrap(),
        );
        let state = Rc::new(RefCell::new(MainState {
            registry,
            opened_record: None,
        }));
        let mut pane_open = PaneOpen::new(ComponentId::new(), tx, &state)?;

        pane_open.spawn_open_record_task_with_record_name(RECORD_NAME_ROOT); // Attempt to open the default root record.

        Ok(Self {
            id,
            args: args.clone(),
            state,
            pane_open,
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

        if let Some(opened_record) = self.state.borrow().opened_record.as_ref() {
            let metadata_table = Table::new(
                opened_record
                    .record
                    .metadata
                    .iter_with_semantic_keys()
                    .map(|(key, value)| crate::cbor::record_metadata_to_row(key, value)),
                [Constraint::Length(16), Constraint::Fill(1)],
            );

            frame.render_widget(metadata_table, area_content);
        }

        frame.render_widget(Span::raw("Record [M]etadata"), area_title);

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

        if let Some(opened_record) = self.state.borrow().opened_record.as_ref() {
            frame.render_widget(
                Text::raw(String::from_utf8_lossy(&opened_record.record.data)), // TODO: Other formats
                area_content,
            );
        }

        Ok(())
    }

    /*
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
            .draw(frame, area_record_name_field, focused_id, ())
            .unwrap();
        frame.render_widget(Span::raw("Encoding"), area_encoding_label);
        self.encoding_radio_array
            .draw(frame, area_encoding_field, focused_id, ())?;
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
    */
}

impl Component for MainView {
    fn update(&mut self, message: ComponentMessage) -> Result<Option<crate::action::Action>> {
        match message {
            ComponentMessage::RecordOpen {
                hashed_record_key,
                read_result: Some(read_result),
            } => {
                self.state.borrow_mut().opened_record = Some(OpenedRecord {
                    hashed_record_key,
                    record: Arc::new(read_result),
                });
                Ok(Some(Action::Render))
            }
            _ => Ok(None),
        }
    }

    fn get_id(&self) -> super::ComponentId {
        self.id
    }

    fn get_children(&self) -> Vec<&dyn Component> {
        vec![&self.pane_open]
    }

    fn get_children_mut(&mut self) -> Vec<&mut dyn Component> {
        vec![&mut self.pane_open]
    }

    fn get_accessibility_node(&self) -> Result<accesskit::Node> {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_children(vec![]);
        Ok(node)
    }
}

impl Drawable for MainView {
    type Args<'a>
        = ()
    where
        Self: 'a;

    fn draw<'a>(
        &self,
        frame: &mut Frame,
        mut area: Rect,
        focused_id: ComponentId,
        (): Self::Args<'a>,
    ) -> Result<()>
    where
        Self: 'a,
    {
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

        if let Some(force_max_width) = self.args.force_max_width.as_ref() {
            area.width = std::cmp::min(area.width, *force_max_width);
        }

        if let Some(force_max_height) = self.args.force_max_height.as_ref() {
            area.height = std::cmp::min(area.height, *force_max_height);
        }

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
        self.pane_open.draw(
            frame,
            area_bottom,
            focused_id,
            PaneOpenArgs {
                title_offset_x: area_metadata.x,
            },
        )?;
        self.draw_header(frame, area_header, focused_id)?;

        Ok(())
    }
}

#[derive(Debug)]
struct PaneOpen {
    id: ComponentId,
    action_tx: UnboundedSender<Action>,
    main_state: Rc<RefCell<MainState>>,
    record_name_field: InputField,
    encoding_radio_array: RadioArray<Encoding>,
}

impl PaneOpen {
    pub fn new(
        id: ComponentId,
        action_tx: &UnboundedSender<Action>,
        main_state: &Rc<RefCell<MainState>>,
    ) -> Result<Self> {
        Ok(Self {
            id,
            action_tx: action_tx.clone(),
            main_state: main_state.clone(),
            record_name_field: InputField::new(ComponentId::new(), action_tx),
            encoding_radio_array: RadioArray::new(
                ComponentId::new(),
                action_tx,
                vec![Encoding::Utf8, Encoding::Hex],
                &Encoding::Utf8,
                Direction::Horizontal,
            ),
        })
    }

    fn get_record_name(&self) -> RecordName {
        match self.encoding_radio_array.get_checked() {
            Encoding::Utf8 => BytesOrAscii(self.record_name_field.get_content().as_bytes().into()),
            Encoding::Hex => todo!(),
        }
    }

    fn spawn_open_record_task(&mut self) {
        let record_name = self.get_record_name();
        self.spawn_open_record_task_with_record_name(record_name);
    }

    fn spawn_open_record_task_with_record_name(&mut self, record_name: RecordName) {
        // The main state is being cloned just because `MainState::get_current_succession_nonce`
        // is an async function that needs to be awaited from within an async block.
        // If this function ever becomes async, it should be moved up out of the async task.
        let main_state_clone = self.main_state.borrow().clone();
        let action_tx = self.action_tx.clone();
        tokio::spawn(
            async move {
                let registry = &*main_state_clone.registry;
                let current_succession_nonce =
                    main_state_clone.get_current_succession_nonce().await;
                let record_key = RecordKey {
                    predecessor_nonce: current_succession_nonce,
                    record_name,
                };
                // TODO: Handle errors by displaying an error message
                let (hashed_record_key, read_result) =
                    Self::open_record(record_key, registry).await.unwrap();

                debug!(?read_result, "Sending read result.");

                action_tx
                    .send(Action::BroadcastMessage(ComponentMessage::RecordOpen {
                        hashed_record_key,
                        read_result,
                    }))
                    .unwrap();
            }
            .instrument(info_span!("open record task")),
        );
    }

    async fn open_record(
        record_key: RecordKey,
        registry: &Registry<ReadLock>,
    ) -> Result<(HashedRecordKey, Option<RecordReadVersionSuccess>)> {
        let hashed_record_key = record_key.hash(&registry.config.hash).await?;
        let versions = registry
            .list_record_versions(&hashed_record_key, 4, 4)
            .await?;
        let Some(latest_version) = versions.last() else {
            return Ok((hashed_record_key, None));
        };
        let record = registry
            .load_record(&hashed_record_key, latest_version.record_version, 4)
            .await?
            .ok_or_else(|| eyre!("Failed to load the latest root record version."))?;
        Ok((hashed_record_key, Some(record)))
    }
}

impl Component for PaneOpen {
    fn update(&mut self, message: ComponentMessage) -> Result<Option<Action>> {
        match message {
            // ComponentMessage::RecordOpen { read_result: None } => {
            //     todo!(); // Display message above button
            //     Ok(Some(Action::Render))
            // }
            // ComponentMessage::RecordOpen { read_result: Some(read_result) } => {

            // }
            _ => Ok(None),
        }
    }

    fn handle_event(&mut self, event: &Event) -> Result<HandleEventSuccess> {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                kind: KeyEventKind::Press,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.spawn_open_record_task();
                Ok(HandleEventSuccess::handled().with_action(Action::Render))
            }
            _ => Ok(HandleEventSuccess::unhandled()),
        }
    }

    fn get_id(&self) -> super::ComponentId {
        self.id
    }

    fn get_children(&self) -> Vec<&dyn Component> {
        vec![&self.record_name_field, &self.encoding_radio_array]
    }

    fn get_children_mut(&mut self) -> Vec<&mut dyn Component> {
        vec![&mut self.record_name_field, &mut self.encoding_radio_array]
    }
}

struct PaneOpenArgs {
    title_offset_x: u16,
}

impl Drawable for PaneOpen {
    type Args<'a>
        = PaneOpenArgs
    where
        Self: 'a;

    fn draw<'a>(
        &self,
        frame: &mut Frame,
        area: Rect,
        focused_id: ComponentId,
        extra_args: Self::Args<'a>,
    ) -> Result<()>
    where
        Self: 'a,
    {
        let (area_title, area_content) = MainView::pane_areas(area, extra_args.title_offset_x);

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
            .draw(frame, area_record_name_field, focused_id, ())
            .unwrap();
        frame.render_widget(Span::raw("Encoding"), area_encoding_label);
        self.encoding_radio_array
            .draw(frame, area_encoding_field, focused_id, ())?;

        Ok(())
    }
}
