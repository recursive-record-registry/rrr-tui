use std::{ops::ControlFlow, sync::atomic::AtomicU64};

use color_eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    layout::{Rect, Size},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, tui::Event};

// pub mod fps;
// pub mod home;
pub mod input_field;
pub mod main_view;

mod id {
    use std::{
        ops::ControlFlow,
        sync::atomic::{AtomicU64, Ordering},
    };

    use derive_deref::{Deref, DerefMut};

    static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

    #[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
    pub struct ComponentId(u64);

    impl ComponentId {
        pub fn root() -> Self {
            Self(0)
        }

        pub fn new() -> Self {
            Self(ID_COUNTER.fetch_add(1, Ordering::SeqCst))
        }
    }

    impl From<ComponentId> for accesskit::NodeId {
        fn from(value: ComponentId) -> Self {
            accesskit::NodeId(value.0)
        }
    }

    /// Contains the path to the focused node, excluding the root node's ID.
    #[derive(Default, Deref, DerefMut)]
    pub struct ComponentIdPath(pub Vec<ComponentId>);

    impl ComponentIdPath {
        pub fn find_deepest_available_component(&self, root: &dyn super::Component) -> ComponentId {
            let mut deepest_available_component_id = ComponentId::root();
            let mut component = root;

            for id in &self.0 {
                let found = component
                    .for_each_child(&mut |child| {
                        if child.get_id() == *id {
                            component = child;
                            ControlFlow::Break(())
                        } else {
                            ControlFlow::Continue(())
                        }
                    })
                    .is_break();

                if found {
                    deepest_available_component_id = *id;
                } else {
                    break;
                }
            }

            deepest_available_component_id
        }
    }
}

pub use id::ComponentId;
pub use id::ComponentIdPath;

/// `Component` is a trait that represents a visual and interactive element of the user interface.
///
/// Implementors of this trait can be registered with the main application loop and will be able to
/// receive events, update state, and be rendered on the screen.
pub trait Component {
    fn handle_event(&mut self, event: Event) -> Result<Option<Action>>;
    fn update(&mut self, action: Action) -> Result<Option<Action>>;
    fn draw(&self, frame: &mut Frame, area: Rect) -> Result<()>;
    fn get_id(&self) -> ComponentId;
    fn get_accessibility_node(&self) -> Result<accesskit::Node>;

    fn get_children(&self) -> Vec<&dyn Component> {
        Default::default()
    }

    fn get_children_mut(&mut self) -> Vec<&mut dyn Component> {
        Default::default()
    }

    fn for_each_child<'a>(
        &'a self,
        f: &mut dyn FnMut(&'a dyn Component) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        for child in self.get_children() {
            (f)(child)?;
        }

        ControlFlow::Continue(())
    }

    fn for_each_child_mut<'a>(
        &'a mut self,
        f: &mut dyn FnMut(&'a mut dyn Component) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        for child in self.get_children_mut() {
            (f)(child)?;
        }

        ControlFlow::Continue(())
    }
}

pub fn depth_first_search(
    subtree_root: &dyn Component,
    visit_preorder: &mut dyn FnMut(&dyn Component) -> ControlFlow<()>,
    visit_postorder: &mut dyn FnMut(&dyn Component) -> ControlFlow<()>,
) -> ControlFlow<()> {
    (visit_preorder)(subtree_root)?;
    subtree_root
        .for_each_child(&mut |child| depth_first_search(child, visit_preorder, visit_postorder))?;
    (visit_postorder)(subtree_root)?;
    ControlFlow::Continue(())
}

pub fn depth_first_search_mut(
    subtree_root: &mut dyn Component,
    visit_preorder: &mut dyn FnMut(&mut dyn Component) -> ControlFlow<()>,
    visit_postorder: &mut dyn FnMut(&mut dyn Component) -> ControlFlow<()>,
) -> ControlFlow<()> {
    (visit_preorder)(subtree_root)?;
    subtree_root.for_each_child_mut(&mut |child| {
        depth_first_search_mut(child, visit_preorder, visit_postorder)
    })?;
    (visit_postorder)(subtree_root)?;
    ControlFlow::Continue(())
}
