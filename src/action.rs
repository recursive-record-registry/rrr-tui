use rrr::record::{HashedRecordKey, RecordReadVersionSuccess};
use strum::Display;

use crate::component::ComponentId;

/// These are applied to all components unconditionally using the `Component::update` method.
#[derive(Debug, Clone, PartialEq, Display)]
pub enum ComponentMessage {
    /// Sent when `Action::Tick` action is processed.
    OnTick,
    OnCheckboxToggle {
        id: ComponentId,
        new_value: bool,
    },
    ShowError {
        error: String,
    },
    RecordOpen {
        hashed_record_key: HashedRecordKey,
        read_result: Option<RecordReadVersionSuccess>,
    },
}

/// Messages generated by components, handled by the app.
#[derive(Debug, Clone, PartialEq, Display)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    FocusChange(FocusChange),
    /// Send a message to all other components.
    BroadcastMessage(ComponentMessage),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum FocusChangeDirection {
    Forward,
    Backward,
}

/// Describes the set of components considered for focus.
/// The horizontal scope is all the siblings of the currently focused component.
/// The vertical scope is all the parent or the first child
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusChange {
    pub direction: FocusChangeDirection,
    pub scope: FocusChangeScope,
}
