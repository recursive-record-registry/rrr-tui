use std::{ops::ControlFlow, sync::Arc};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::Rect;
use tokio::sync::mpsc;
use tracing::instrument;

use crate::{
    action::{Action, ComponentMessage, FocusChange, FocusChangeDirection, FocusChangeScope},
    args::Args,
    components::{
        self, find_component_by_id_mut, main_view::MainView, Component, ComponentId,
        ComponentIdPath, DefaultDrawableComponent, HandleEventSuccess,
    },
    tui::{Event, Tui},
};

#[derive(Debug)]
pub struct App {
    tick_rate: f64,
    frame_rate: f64,
    should_quit: bool,
    should_suspend: bool,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
    root_component: Box<dyn DefaultDrawableComponent>,
    focus_path: ComponentIdPath,
}

impl App {
    #[instrument]
    pub async fn new(args: &Arc<Args>) -> Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let mut app = Self {
            tick_rate: args.tick_rate,
            frame_rate: args.frame_rate,
            should_quit: false,
            should_suspend: false,
            last_tick_key_events: Vec::new(),
            root_component: Box::new(MainView::new(ComponentId::root(), &action_tx, args).await?),
            focus_path: Default::default(),
            action_tx,
            action_rx,
        };

        // Ensure a valid initial focus.
        if !app.root_component.is_focusable() {
            app.change_focus(FocusChange {
                direction: FocusChangeDirection::Forward,
                scope: FocusChangeScope::HorizontalAndVertical,
            })?;
        }

        Ok(app)
    }

    #[instrument(skip(self))]
    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new(tracing::Span::current())?
            // .mouse(true) // uncomment this line to enable mouse support
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        tui.enter()?;

        let action_tx = self.action_tx.clone();
        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui)?;
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                // tui.mouse(true);
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    #[instrument(skip(self, tui))]
    async fn handle_events(&mut self, tui: &mut Tui) -> Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            // TODO: App could get overwhelmed by tick/render events/actions.
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }

        self.focus_path
            .for_each_component_mut::<Result<()>>(
                &mut *self.root_component,
                &mut |_| ControlFlow::Continue(()),
                &mut |focused_component| -> ControlFlow<Result<()>, ()> {
                    match focused_component.handle_event(&event) {
                        Ok(HandleEventSuccess { action, absorb }) => {
                            if let Some(action) = action {
                                action_tx.send(action).unwrap();
                            }

                            if absorb {
                                ControlFlow::Break(Ok(()))
                            } else {
                                ControlFlow::Continue(())
                            }
                        }
                        Err(error) => ControlFlow::Break(Err(error)),
                    }
                },
            )
            .break_value()
            .transpose()?;

        // let (focused_component, _) = self
        //     .focus_path
        //     .find_deepest_available_component_mut(&mut *self.root_component);
        // focused_component.handle_event(event)?;
        // components::depth_first_search_mut(
        //     &mut *self.root_component,
        //     &mut |component| {
        //         if let Some(action) = component.handle_event(event.clone()).unwrap() {
        //             action_tx.send(action).unwrap();
        //         }

        //         ControlFlow::Continue(())
        //     },
        //     &mut |_| ControlFlow::Continue(()),
        // );
        // for component in self.components.iter_mut() {
        //     if let Some(action) = component.handle_event(event.clone())? {
        //         action_tx.send(action)?;
        //     }
        // }
        Ok(())
    }

    #[instrument(skip(self))]
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        tracing::trace!(?key);
        let action = match key {
            KeyEvent {
                code: KeyCode::Char('c' | 'd'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => Some(Action::Quit),
            KeyEvent {
                code: code @ (KeyCode::Tab | KeyCode::BackTab),
                modifiers: modifiers @ (KeyModifiers::NONE | KeyModifiers::SHIFT),
                ..
            } => Some(Action::FocusChange(FocusChange {
                direction: if (modifiers != KeyModifiers::NONE) || (code == KeyCode::BackTab) {
                    FocusChangeDirection::Backward
                } else {
                    FocusChangeDirection::Forward
                },
                scope: FocusChangeScope::HorizontalAndVertical,
            })),
            KeyEvent {
                code: code @ (KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right),
                modifiers: KeyModifiers::ALT,
                ..
            } => Some(Action::FocusChange(FocusChange {
                direction: if code == KeyCode::Down || code == KeyCode::Right {
                    FocusChangeDirection::Forward
                } else {
                    FocusChangeDirection::Backward
                },
                scope: if code == KeyCode::Up || code == KeyCode::Down {
                    FocusChangeScope::Vertical
                } else {
                    FocusChangeScope::Horizontal
                },
            })),
            _ => None,
        };
        if let Some(action) = action {
            self.action_tx.send(action)?;
        }
        Ok(())
    }

    #[instrument(skip(self))]
    fn change_focus(&mut self, focus_change: FocusChange) -> Result<()> {
        match focus_change.scope {
            FocusChangeScope::HorizontalAndVertical => {
                let mut focused_component_visited = false;
                let mut first_focusable_component = None;
                let mut last_focusable_component = None;
                let mut previous_focusable_component = None;
                let mut next_focusable_component = None;
                let (originally_selected_component, deepest_available_path) = self
                    .focus_path
                    .find_deepest_available_component_mut(&mut *self.root_component);

                originally_selected_component.handle_event(&Event::FocusLost)?;

                let deepest_available_id = deepest_available_path
                    .last()
                    .copied()
                    .unwrap_or(self.root_component.get_id());

                let _ = components::depth_first_search(
                    &*self.root_component,
                    &mut |component| -> ControlFlow<()> {
                        if component.is_focusable() {
                            if first_focusable_component.is_none() {
                                first_focusable_component = Some(component);
                            }

                            if focused_component_visited && next_focusable_component.is_none() {
                                next_focusable_component = Some(component);
                            }

                            if component.get_id() == deepest_available_id {
                                focused_component_visited = true;
                                previous_focusable_component = last_focusable_component;
                            }

                            last_focusable_component = Some(component);
                        }

                        ControlFlow::Continue(())
                    },
                    &mut |_component| -> ControlFlow<()> { ControlFlow::Continue(()) },
                );

                next_focusable_component = next_focusable_component.or(first_focusable_component);
                previous_focusable_component =
                    previous_focusable_component.or(last_focusable_component);

                if focus_change.direction == FocusChangeDirection::Backward {
                    std::mem::swap(
                        &mut next_focusable_component,
                        &mut previous_focusable_component,
                    );
                }

                if let Some(next_focusable_component) = next_focusable_component {
                    let next_focusable_component_id = next_focusable_component.get_id();
                    let (newly_selected_component, focus_path) = find_component_by_id_mut(
                        &mut *self.root_component,
                        next_focusable_component_id,
                    )
                    .unwrap();
                    self.focus_path = focus_path;
                    newly_selected_component.handle_event(&Event::FocusGained)?;
                    tracing::debug!(focus_path=?self.focus_path, "Focus changed.");
                }
            }
            FocusChangeScope::Horizontal => unimplemented!(),
            FocusChangeScope::Vertical => unimplemented!(),
        }

        Ok(())
    }

    #[instrument(skip(self, tui))]
    fn handle_actions(&mut self, tui: &mut Tui) -> Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            let mut component_message = None;

            match action {
                Action::Tick => {
                    self.last_tick_key_events.drain(..);
                    component_message = Some(ComponentMessage::OnTick);
                }
                Action::BroadcastMessage(message) => component_message = Some(message),
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.terminal.clear()?,
                Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                Action::Render => self.render(tui)?,
                Action::FocusChange(focus_change) => self.change_focus(focus_change)?,
            }

            if let Some(component_message) = component_message {
                let _ = components::depth_first_search_mut(
                    &mut *self.root_component,
                    &mut |component| -> ControlFlow<()> {
                        if let Some(action) = component.update(component_message.clone()).unwrap() {
                            self.action_tx.send(action).unwrap()
                        }

                        ControlFlow::Continue(())
                    },
                    &mut |_| ControlFlow::Continue(()),
                );
            }
        }
        Ok(())
    }

    #[instrument(skip(self, tui))]
    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    #[instrument(skip(self, tui))]
    fn render(&mut self, tui: &mut Tui) -> Result<()> {
        let mut result = Ok(());
        tui.draw(|frame| {
            result = self.root_component.default_draw(
                frame,
                frame.area(),
                self.get_focused_component_id(),
            );
        })?;
        result
    }

    fn get_focused_component_id(&self) -> ComponentId {
        self.focus_path
            .last()
            .copied()
            .unwrap_or_else(|| self.root_component.get_id())
    }
}
