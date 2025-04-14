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
use taffy::Dimension;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{Instrument, debug, info_span};

use crate::action::{Action, ComponentMessage};
use crate::args::Args;
use crate::color::{ColorOklch, TextColor};
use crate::component::{
    Component, ComponentExt, ComponentId, DefaultDrawable, DrawContext, Drawable,
    HandleEventSuccess,
};
use crate::components::button::Button;
use crate::components::input_field::InputField;
use crate::components::open_status::{Animation, OpenStatus, SpinnerContent};
use crate::components::radio_array::RadioArray;
use crate::components::styled_widget::StyledWidget;
use crate::env::PROJECT_VERSION;
use crate::error;
use crate::layout::{LayoutExt, SizeExt, TaffyNodeData};
use crate::tui::Event;

use super::{Encoding, MainState, MainView};

#[derive(Debug)]
pub struct PaneOpen {
    id: ComponentId,
    taffy_node_data: TaffyNodeData,
    action_tx: UnboundedSender<Action>,
    main_state: Rc<RefCell<MainState>>,
    record_name_label: StyledWidget<Span<'static>>,
    record_name_field: InputField,
    encoding_label: StyledWidget<Span<'static>>,
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
            taffy_node_data: TaffyNodeData::new(taffy::Style {
                display: taffy::Display::Grid,
                size: taffy::Size {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                    // height: Dimension::length(3.0),
                },
                min_size: taffy::Size {
                    width: Dimension::length(0.0),
                    height: Dimension::length(1.0),
                },
                grid_template_columns: vec![
                    taffy::prelude::min_content(),
                    taffy::prelude::auto(),
                    taffy::prelude::min_content(),
                ],
                grid_template_rows: vec![taffy::prelude::auto(), taffy::prelude::auto()],
                gap: taffy::Size {
                    width: taffy::prelude::length(1.0),
                    ..taffy::Size::zero()
                },
                // This padding is for the pane's title.
                padding: taffy::Rect {
                    top: taffy::prelude::length(1.0),
                    ..taffy::Rect::zero()
                },
                ..Default::default()
            }),
            action_tx: action_tx.clone(),
            main_state: main_state.clone(),
            record_name_label: StyledWidget::new(
                ComponentId::new(),
                action_tx,
                "Record Name".into(),
            )
            .with_style(taffy::Style {
                grid_row: taffy::prelude::line(1),
                grid_column: taffy::prelude::line(1),
                ..Default::default()
            }),
            record_name_field: InputField::new(ComponentId::new(), action_tx).with_style(
                taffy::Style {
                    grid_row: taffy::prelude::line(1),
                    grid_column: taffy::prelude::line(2),
                    ..Default::default()
                },
            ),
            encoding_label: StyledWidget::new(ComponentId::new(), action_tx, "Encoding".into())
                .with_style(taffy::Style {
                    grid_row: taffy::prelude::line(2),
                    grid_column: taffy::prelude::line(1),
                    ..Default::default()
                }),
            encoding_radio_array: RadioArray::new(
                ComponentId::new(),
                action_tx,
                vec![Encoding::Utf8, Encoding::Hex],
                &Encoding::Utf8,
                Direction::Horizontal,
            )
            .with_style(taffy::Style {
                grid_row: taffy::prelude::line(2),
                grid_column: taffy::prelude::line(2),
                ..Default::default()
            }),
            status_spinner: OpenStatus::new(
                ComponentId::new(),
                action_tx,
                SpinnerContent::default(),
            )
            .with_style(taffy::Style {
                grid_row: taffy::prelude::line(1),
                grid_column: taffy::prelude::line(3),
                ..Default::default()
            }),
            button: Button::new(ComponentId::new(), action_tx, "Search".into())
                .with_form_confirmation(true)
                .with_text_color_unfocused(TextColor::default().bg(ColorOklch::new(0.2, 0.0, 0.0)))
                .with_text_color_focused(TextColor::default().bg(ColorOklch::new(0.4, 0.0, 0.0)))
                .with_text_color_pressed(TextColor::default().bg(ColorOklch::new(0.3, 0.0, 0.0)))
                .with_style(taffy::Style {
                    grid_row: taffy::prelude::line(2),
                    grid_column: taffy::prelude::line(3),
                    ..Default::default()
                }),
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

    pub fn spawn_open_record_task_with_record_name(&mut self, record_name: RecordName) {
        // The main state is being cloned just because `MainState::get_current_succession_nonce`
        // is an async function that needs to be awaited from within an async block.
        // If this function ever becomes async, it should be moved up out of the async task.
        let main_state_clone = self.main_state.borrow().clone();
        let action_tx = self.action_tx.clone();

        self.status_spinner.set_content(
            SpinnerContent::default()
                .with_text(" Searching… ".into())
                .with_animation(Some(Animation::ProgressIndeterminate {
                    period: Duration::from_secs_f32(0.5),
                    highlight: TextColor::default().bg(ColorOklch::new(0.4, 0.0, 0.0)),
                })),
        );

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
                    self.status_spinner.set_content(
                        SpinnerContent::default()
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
                            })),
                    );
                } else {
                    self.status_spinner.set_content(
                        SpinnerContent::default()
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
                            })),
                    );
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
            &self.record_name_label,
            &self.record_name_field,
            &self.encoding_label,
            &self.encoding_radio_array,
            &self.status_spinner,
            &self.button,
        ]
    }

    fn get_children_mut(&mut self) -> Vec<&mut dyn Component> {
        vec![
            &mut self.record_name_label,
            &mut self.record_name_field,
            &mut self.encoding_label,
            &mut self.encoding_radio_array,
            &mut self.status_spinner,
            &mut self.button,
        ]
    }

    fn get_taffy_node_data(&self) -> &TaffyNodeData {
        &self.taffy_node_data
    }

    fn get_taffy_node_data_mut(&mut self) -> &mut TaffyNodeData {
        &mut self.taffy_node_data
    }
}

pub struct PaneOpenArgs {
    pub title_offset_x: u16,
}

impl Drawable for PaneOpen {
    type Args<'a>
        = PaneOpenArgs
    where
        Self: 'a;

    fn draw<'a>(&self, context: &mut DrawContext, extra_args: Self::Args<'a>) -> Result<()>
    where
        Self: 'a,
    {
        let area = self.taffy_node_data.absolute_layout().padding_rect();
        let (area_title, _) = MainView::pane_areas(area, extra_args.title_offset_x);
        // tracing::trace!(
        //     ?area,
        //     ?area_title,
        //     c = ?self.absolute_layout().content_rect()
        // );

        context
            .frame()
            .render_widget(Span::raw("Open Sub-Record [Enter]"), area_title);

        // let layout_bottom_lines = Layout::default()
        //     .direction(Direction::Horizontal)
        //     .spacing(1)
        //     .constraints([
        //         Constraint::Length(11),
        //         Constraint::Fill(1),
        //         Constraint::Length(18),
        //     ]);
        // let [area_record_name, area_encoding] = Layout::default()
        //     .direction(Direction::Vertical)
        //     .constraints([Constraint::Length(1), Constraint::Length(1)])
        //     .areas(area_content);
        // let [area_record_name_label, area_record_name_field, area_status] =
        //     layout_bottom_lines.areas(area_record_name);
        // let [area_encoding_label, area_encoding_field, area_button] =
        //     layout_bottom_lines.areas(area_encoding);

        self.record_name_label.default_draw(context)?;
        // context
        //     .frame()
        //     .render_widget(Span::raw("Record Name"), area_record_name_label);
        self.record_name_field.default_draw(context)?;
        self.status_spinner.default_draw(context)?;
        self.encoding_label.default_draw(context)?;
        // context
        //     .frame()
        //     .render_widget(Span::raw("Encoding"), area_encoding_label);
        self.encoding_radio_array.default_draw(context)?;
        self.button.default_draw(context)?;

        Ok(())
    }
}
