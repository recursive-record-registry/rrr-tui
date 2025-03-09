use serde::{Deserialize, Serialize};
use strum::Display;

/// Frontend-generated messages.
/// These are applied to all components unconditionally using the `Component::update` method.
/// TODO: Split into ComponentAction and AppAction
#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Help,
    FocusChange(FocusChange),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum FocusChangeDirection {
    Forward,
    Backward,
}

/// Describes the set of components considered for focus.
/// The horizontal scope is all the siblings of the currently focused component.
/// The vertical scope is all the parent or the first child
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum FocusChangeScope {
    Horizontal,
    Vertical,
    HorizontalAndVertical,
}

impl FocusChangeScope {
    pub fn is_horizontal_allowed(&self) -> bool {
        matches!(self, Self::Horizontal | Self::HorizontalAndVertical)
    }

    pub fn is_vertical_allowed(&self) -> bool {
        matches!(self, Self::Vertical | Self::HorizontalAndVertical)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusChange {
    pub direction: FocusChangeDirection,
    pub scope: FocusChangeScope,
}
