use std::ops::ControlFlow;

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::Rect;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::{
    action::{Action, FocusChange, FocusChangeDirection, FocusChangeScope},
    components::{self, main_view::MainView, Component, ComponentId, ComponentIdPath},
    tui::{Event, Tui},
};

pub struct App {
    tick_rate: f64,
    frame_rate: f64,
    should_quit: bool,
    should_suspend: bool,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
    root_component: Box<dyn Component>,
    focus_path: ComponentIdPath,
}

impl App {
    pub fn new(tick_rate: f64, frame_rate: f64) -> Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        Ok(Self {
            tick_rate,
            frame_rate,
            should_quit: false,
            should_suspend: false,
            last_tick_key_events: Vec::new(),
            root_component: Box::new(MainView::new(ComponentId::root(), action_tx.clone())),
            focus_path: Default::default(),
            action_tx,
            action_rx,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?
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

    async fn handle_events(&mut self, tui: &mut Tui) -> Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }
        components::depth_first_search_mut(
            &mut *self.root_component,
            &mut |component| {
                if let Some(action) = component.handle_event(event.clone()).unwrap() {
                    action_tx.send(action).unwrap();
                }

                ControlFlow::Continue(())
            },
            &mut |_| ControlFlow::Continue(()),
        );
        // for component in self.components.iter_mut() {
        //     if let Some(action) = component.handle_event(event.clone())? {
        //         action_tx.send(action)?;
        //     }
        // }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        let action = match key {
            KeyEvent {
                code: KeyCode::Char('c' | 'd'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => Some(Action::Quit),
            KeyEvent {
                code: KeyCode::Tab,
                modifiers: modifiers @ (KeyModifiers::NONE | KeyModifiers::SHIFT),
                ..
            } => Some(Action::FocusChange(FocusChange {
                direction: if modifiers == KeyModifiers::NONE {
                    FocusChangeDirection::Forward
                } else {
                    FocusChangeDirection::Backward
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

    fn handle_actions(&mut self, tui: &mut Tui) -> Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            if action != Action::Tick && action != Action::Render {
                debug!("{action:?}");
            }
            match action {
                Action::Tick => {
                    self.last_tick_key_events.drain(..);
                }
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.terminal.clear()?,
                Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                Action::Render => self.render(tui)?,
                _ => {}
            }
            components::depth_first_search_mut(
                &mut *self.root_component,
                &mut |component| {
                    if let Some(action) = component.update(action.clone()).unwrap() {
                        self.action_tx.send(action).unwrap()
                    }

                    ControlFlow::Continue(())
                },
                &mut |_| ControlFlow::Continue(()),
            );
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> Result<()> {
        let mut result = Ok(());
        tui.draw(|frame| {
            result = self.root_component.draw(frame, frame.area());
        })?;
        result
    }
}
