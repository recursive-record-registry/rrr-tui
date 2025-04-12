use core::option::Option::Some;
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use color_eyre::eyre::{Result, eyre};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::Table;
use rrr::record::{
    HashedRecordKey, RECORD_NAME_ROOT, RecordKey, RecordName, RecordReadVersionSuccess,
    SuccessionNonce,
};
use rrr::registry::Registry;
use rrr::utils::fd_lock::ReadLock;
use rrr::utils::serde::BytesOrAscii;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{Instrument, debug, info_span};

use crate::action::{Action, ComponentMessage};
use crate::args::Args;
use crate::color::{ColorOklch, TextColor};
use crate::component::{Component, ComponentId, DrawContext, Drawable, HandleEventSuccess};
use crate::env::PROJECT_VERSION;
use crate::error;
use crate::tui::Event;

use super::button::Button;
use super::input_field::InputField;
use super::open_status::{Animation, OpenStatus, SpinnerContent};
use super::radio_array::RadioArray;

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

    fn draw_header(&self, context: &mut DrawContext, area_header: Rect) -> Result<()> {
        context.frame().render_widget(
            Span::raw(format!("RRR TUI v{}", *PROJECT_VERSION)),
            area_header,
        );
        Ok(())
    }

    fn draw_pane_tree(&self, context: &mut DrawContext, area: Rect) -> Result<()> {
        let (area_title, _area_content) = Self::pane_areas(area, 0);
        context
            .frame()
            .render_widget(Span::raw("[T]ree"), area_title);
        Ok(())
    }

    fn draw_pane_metadata(&self, context: &mut DrawContext, area: Rect) -> Result<()> {
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

            context.frame().render_widget(metadata_table, area_content);
        }

        context
            .frame()
            .render_widget(Span::raw("Record [M]etadata"), area_title);

        Ok(())
    }

    fn draw_pane_overview(&self, context: &mut DrawContext, area: Rect) -> Result<()> {
        let (area_title, _area_content) = Self::pane_areas(area, 0);
        context
            .frame()
            .render_widget(Span::raw("[O]verview"), area_title);
        Ok(())
    }

    fn draw_pane_content(
        &self,
        context: &mut DrawContext,
        area: Rect,
        title_offset_x: u16,
    ) -> Result<()> {
        let (area_title, area_content) = Self::pane_areas(area, title_offset_x);

        context
            .frame()
            .render_widget(Span::raw("Record [C]ontent"), area_title);

        if let Some(opened_record) = self.state.borrow().opened_record.as_ref() {
            let data_string = String::from_utf8_lossy(&opened_record.record.data);
            let lines = textwrap::wrap(
                data_string.as_ref(),
                textwrap::Options::new(area.width as usize),
            );
            context.frame().render_widget(
                Text::from_iter(lines).style(Style::default()), // TODO: Other formats
                area_content,
            );
        }

        Ok(())
    }
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

    fn get_id(&self) -> ComponentId {
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

    fn draw<'a>(&self, context: &mut DrawContext, mut area: Rect, (): Self::Args<'a>) -> Result<()>
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

        context
            .frame()
            .buffer_mut()
            .set_style(area, TextColor::default());

        let [
            area_header,
            area_top,
            area_content,
            area_bottom,
            area_footer,
        ] = Layout::default()
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

        context
            .frame()
            .render_widget(SPACER_HORIZONTAL.clone(), area_top);
        context
            .frame()
            .render_widget(SPACER_HORIZONTAL.clone(), area_content);
        context
            .frame()
            .render_widget(SPACER_HORIZONTAL.clone(), area_bottom);
        context
            .frame()
            .render_widget(SPACER_HORIZONTAL.clone(), area_footer);
        context.frame().render_widget(
            SPACER_VERTICAL_FORKED.clone(),
            Rect {
                height: area_top_spacer_0.height + 1,
                ..area_top_spacer_0
            },
        );
        context.frame().render_widget(
            SPACER_VERTICAL_FORKED.clone(),
            Rect {
                height: area_top_spacer_1.height + 1,
                ..area_top_spacer_1
            },
        );

        self.draw_pane_tree(context, area_tree)?;
        self.draw_pane_metadata(context, area_metadata)?;
        self.draw_pane_overview(context, area_overview)?;
        self.draw_pane_content(context, area_content, area_metadata.x)?;
        self.pane_open.draw(
            context,
            area_bottom,
            PaneOpenArgs {
                title_offset_x: area_metadata.x,
            },
        )?;
        self.draw_header(context, area_header)?;

        /* Debug Oklch color space
        for y in area.y..(area.y + area.height) {
            for x in area.x..(area.x + area.width) {
                let u = (x - area.x) as f32 / area.width as f32;
                let v = (y - area.y) as f32 / area.height as f32;
                let Some(cell) = context.frame().buffer_mut().cell_mut((x, y)) else {
                    continue;
                };

                cell.set_char(' ');
                cell.bg = ColorOklch::new(0.75, 0.15 * v, u).into();
            }
        }
        */

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
    status_spinner: OpenStatus<'static>,
    button: Button,
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
            status_spinner: OpenStatus::new(
                ComponentId::new(),
                action_tx,
                SpinnerContent::default(),
            ),
            button: Button::new(ComponentId::new(), action_tx, "Search".into())
                .with_form_confirmation(true)
                .with_text_color_unfocused(TextColor::default().bg(ColorOklch::new(0.2, 0.0, 0.0)))
                .with_text_color_focused(TextColor::default().bg(ColorOklch::new(0.4, 0.0, 0.0)))
                .with_text_color_pressed(TextColor::default().bg(ColorOklch::new(0.3, 0.0, 0.0))),
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

        self.status_spinner.content = SpinnerContent::default()
            .with_text(" Searching… ".into())
            .with_animation(Some(Animation::ProgressIndeterminate {
                period: Duration::from_secs_f32(0.5),
                highlight: TextColor::default().bg(ColorOklch::new(0.4, 0.0, 0.0)),
            }));

        tokio::spawn(
            async move {
                let registry = &*main_state_clone.registry;
                let current_succession_nonce =
                    main_state_clone.get_current_succession_nonce().await;
                let record_key = RecordKey {
                    predecessor_nonce: current_succession_nonce,
                    record_name,
                };

                error::report(&action_tx.clone(), async move || {
                    let (hashed_record_key, read_result) =
                        Self::open_record(record_key, registry).await?;

                    debug!(?read_result, "Sending read result.");

                    action_tx.send(Action::BroadcastMessage(ComponentMessage::RecordOpen {
                        hashed_record_key,
                        read_result,
                    }))?;

                    Ok(())
                })
                .await;
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
            ComponentMessage::RecordOpen { read_result, .. } => {
                let now = Instant::now();
                if read_result.is_some() {
                    self.record_name_field.reset_content();
                    self.status_spinner.content = SpinnerContent::default()
                        .with_text("Record found".into())
                        .with_animation(Some(Animation::Ease {
                            easing_function: easing_function::easings::EaseInOutCubic.into(),
                            color_start: TextColor::default().fg(ColorOklch::new(
                                0.79,
                                0.1603,
                                153.29 / 360.0,
                            )),
                            color_end: TextColor::default().fg(ColorOklch::new(
                                0.5,
                                0.0,
                                153.29 / 360.0,
                            )),
                            instant_start: now + Duration::from_secs_f32(0.25),
                            instant_end: now + Duration::from_secs_f32(1.0),
                        }));
                } else {
                    self.status_spinner.content = SpinnerContent::default()
                        .with_text("Record not found".into())
                        .with_animation(Some(Animation::Ease {
                            easing_function: easing_function::easings::EaseInOutCubic.into(),
                            color_start: TextColor::default().fg(ColorOklch::new(
                                0.79,
                                0.1603,
                                67.76 / 360.0,
                            )),
                            color_end: TextColor::default().fg(ColorOklch::new(
                                0.5,
                                0.0,
                                67.76 / 360.0,
                            )),
                            instant_start: now + Duration::from_secs_f32(0.25),
                            instant_end: now + Duration::from_secs_f32(1.0),
                        }));
                }

                Ok(Some(Action::Render))
            }
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
                self.button.held_down = true;
                Ok(HandleEventSuccess::handled().with_action(Action::Render))
            }
            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                kind: KeyEventKind::Release,
                ..
            })
            | Event::FocusLost => {
                self.button.held_down = false;
                Ok(HandleEventSuccess::handled().with_action(Action::Render))
            }
            _ => Ok(HandleEventSuccess::unhandled()),
        }
    }

    fn get_id(&self) -> ComponentId {
        self.id
    }

    fn get_children(&self) -> Vec<&dyn Component> {
        vec![
            &self.record_name_field,
            &self.encoding_radio_array,
            &self.status_spinner,
            &self.button,
        ]
    }

    fn get_children_mut(&mut self) -> Vec<&mut dyn Component> {
        vec![
            &mut self.record_name_field,
            &mut self.encoding_radio_array,
            &mut self.status_spinner,
            &mut self.button,
        ]
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
        context: &mut DrawContext,
        area: Rect,
        extra_args: Self::Args<'a>,
    ) -> Result<()>
    where
        Self: 'a,
    {
        let (area_title, area_content) = MainView::pane_areas(area, extra_args.title_offset_x);

        context
            .frame()
            .render_widget(Span::raw("Open Sub-Record [Enter]"), area_title);

        let layout_bottom_lines = Layout::default()
            .direction(Direction::Horizontal)
            .spacing(1)
            .constraints([
                Constraint::Length(11),
                Constraint::Fill(1),
                Constraint::Length(18),
            ]);
        let [area_record_name, area_encoding] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .areas(area_content);
        let [area_record_name_label, area_record_name_field, area_status] =
            layout_bottom_lines.areas(area_record_name);
        let [area_encoding_label, area_encoding_field, area_button] =
            layout_bottom_lines.areas(area_encoding);

        context
            .frame()
            .render_widget(Span::raw("Record Name"), area_record_name_label);
        self.record_name_field
            .draw(context, area_record_name_field, ())?;
        self.status_spinner.draw(context, area_status, ())?;
        context
            .frame()
            .render_widget(Span::raw("Encoding"), area_encoding_label);
        self.encoding_radio_array
            .draw(context, area_encoding_field, ())?;
        self.button.draw(context, area_button, ())?;

        Ok(())
    }
}
