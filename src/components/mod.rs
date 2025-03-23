use std::{cell::RefCell, fmt::Debug, ops::ControlFlow, sync::atomic::AtomicU64};

use color_eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use polonius_the_crab::{polonius, polonius_return};
use ratatui::{
    layout::{Rect, Size},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::{Action, ComponentMessage},
    tui::Event,
};

// pub mod fps;
// pub mod home;
pub mod checkbox;
pub mod input_field;
pub mod main_view;

mod id {
    use std::{
        ops::ControlFlow,
        sync::atomic::{AtomicU64, Ordering},
    };

    use derive_deref::{Deref, DerefMut};
    use polonius_the_crab::{
        exit_polonius, polonius, polonius_break_dependent, polonius_continue, polonius_loop,
        polonius_return,
        ඞ::cannot_use__polonius_break_dependentǃ__without_a_break_type_annotation_on__polonius_loopǃ,
    };

    use super::*;

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
    #[derive(Debug, Clone, Default, Deref, DerefMut)]
    pub struct ComponentIdPath(pub Vec<ComponentId>);

    impl ComponentIdPath {
        pub fn find_deepest_available_component<'a>(
            &self,
            root: &'a dyn super::Component,
        ) -> (&'a dyn super::Component, Self) {
            pub fn find_deepest_available_component_inner<'a>(
                path: &[ComponentId],
                root: &'a dyn super::Component,
                depth: usize,
            ) -> (&'a dyn super::Component, usize) {
                tracing::trace!(?path, ?depth, "d");

                if let Some((head, tail)) = path.split_first() {
                    for child in root.get_children() {
                        if child.get_id() == *head {
                            return find_deepest_available_component_inner(tail, child, depth + 1);
                        }
                    }
                }

                (root, depth)
            }

            let (node, depth) = find_deepest_available_component_inner(&self.0, root, 0);

            (node, Self(self.0[..depth].into()))
        }

        pub fn find_deepest_available_component_mut<'a>(
            &self,
            root: &'a mut dyn super::Component,
        ) -> (&'a mut dyn super::Component, Self) {
            pub fn find_deepest_available_component_mut_inner<'a>(
                path: &[ComponentId],
                mut root: &'a mut dyn super::Component,
                depth: usize,
            ) -> (&'a mut dyn super::Component, usize) {
                if let Some((head, tail)) = path.split_first() {
                    polonius_loop!(|root| -> (&'polonius mut dyn super::Component, usize) {
                        for child in root.get_children_mut() {
                            if child.get_id() == *head {
                                polonius_return!(find_deepest_available_component_mut_inner(
                                    tail,
                                    child,
                                    depth + 1,
                                ));
                            }
                        }
                    });
                }

                (root, depth)
            }

            let (node, depth) = find_deepest_available_component_mut_inner(&self.0, root, 0);

            (node, Self(self.0[..depth].into()))
        }
    }
}

pub use id::ComponentId;
pub use id::ComponentIdPath;

/// `Component` is a trait that represents a visual and interactive element of the user interface.
///
/// Implementors of this trait can be registered with the main application loop and will be able to
/// receive events, update state, and be rendered on the screen.
pub trait Component: Debug {
    /// Handle events when focused.
    fn handle_event(&mut self, event: Event) -> Result<Option<Action>>;

    fn update(&mut self, message: ComponentMessage) -> Result<Option<Action>>;

    fn draw(&self, frame: &mut Frame, area: Rect, focused_id: ComponentId) -> Result<()>;

    /// Returns the immutable unique ID of this component's instance.
    fn get_id(&self) -> ComponentId;

    fn get_accessibility_node(&self) -> Result<accesskit::Node>;

    /// Returns `true` iff this component can be focused such that it is able to handle events.
    fn is_focusable(&self) -> bool;

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

// Standalone generic functions folľow, because they cannot be on trait objects.

pub fn for_each_child<'a, B: 'a>(
    component: &'a dyn Component,
    mut f: impl FnMut(&'a dyn Component) -> ControlFlow<B>,
) -> Option<B> {
    let mut result = ControlFlow::Continue(());
    component.for_each_child(&mut |component| {
        result = (f)(component);
        match &result {
            ControlFlow::Break(_) => ControlFlow::Break(()),
            ControlFlow::Continue(_) => ControlFlow::Continue(()),
        }
    });
    result.break_value()
}

pub fn for_each_child_mut<'a, B: 'a>(
    component: &'a mut dyn Component,
    mut f: impl FnMut(&'a mut dyn Component) -> ControlFlow<B>,
) -> Option<B> {
    let mut result = ControlFlow::Continue(());
    component.for_each_child_mut(&mut |component| {
        result = (f)(component);
        match &result {
            ControlFlow::Break(_) => ControlFlow::Break(()),
            ControlFlow::Continue(_) => ControlFlow::Continue(()),
        }
    });
    result.break_value()
}

pub fn find_child_by_id<'a>(
    component: &'a dyn Component,
    child_id: ComponentId,
) -> Option<&'a dyn Component> {
    for_each_child(component, |child| {
        if child.is_focusable() && child.get_id() == child_id {
            ControlFlow::Break(child)
        } else {
            ControlFlow::Continue(())
        }
    })
}

pub fn find_child_by_id_mut<'a>(
    component: &'a mut dyn Component,
    child_id: ComponentId,
) -> Option<&'a mut dyn Component> {
    for_each_child_mut(component, |child| {
        if child.is_focusable() && child.get_id() == child_id {
            ControlFlow::Break(child)
        } else {
            ControlFlow::Continue(())
        }
    })
}

pub fn depth_first_search<'a, B: 'a>(
    subtree_root: &'a dyn Component,
    visit_preorder: &mut dyn FnMut(&'a dyn Component) -> ControlFlow<B>,
    visit_postorder: &mut dyn FnMut(&'a dyn Component) -> ControlFlow<B>,
) -> ControlFlow<B> {
    (visit_preorder)(subtree_root)?;
    if let Some(break_value) = for_each_child::<B>(subtree_root, |child| {
        depth_first_search(child, visit_preorder, visit_postorder)
    }) {
        return ControlFlow::Break(break_value);
    }
    (visit_postorder)(subtree_root)?;
    ControlFlow::Continue(())
}

pub fn depth_first_search_mut<'a, B: 'a>(
    subtree_root: &'a mut dyn Component,
    visit_preorder: &mut dyn FnMut(&'a mut dyn Component) -> ControlFlow<B>,
    visit_postorder: &mut dyn FnMut(&'a mut dyn Component) -> ControlFlow<B>,
) -> ControlFlow<B> {
    // Safety:
    // No aliased mutable references actually occur, because the try-operator (`?`) is used to
    // return `ControlFlow::<B>::Break` early.
    // This seems like a case where the Polonius-based borrowck would be required to avoid the use
    // of `unsafe`.
    let subtree_root_ptr = subtree_root as *mut dyn Component;
    (visit_preorder)(unsafe { &mut *subtree_root_ptr })?;
    if let Some(break_value) = for_each_child_mut::<B>(unsafe { &mut *subtree_root_ptr }, |child| {
        depth_first_search_mut(child, visit_preorder, visit_postorder)
    }) {
        return ControlFlow::Break(break_value);
    }
    (visit_postorder)(unsafe { &mut *subtree_root_ptr })?;
    ControlFlow::Continue(())
}

pub fn find_component_by_id_mut(
    subtree_root: &mut dyn Component,
    id: ComponentId,
) -> Option<(&mut dyn Component, ComponentIdPath)> {
    let path = RefCell::new(ComponentIdPath::default());
    let component = depth_first_search_mut(
        subtree_root,
        &mut |component| {
            path.borrow_mut().push(component.get_id());
            if component.get_id() == id {
                return ControlFlow::Break(component);
            }
            ControlFlow::Continue(())
        },
        &mut |component| {
            path.borrow_mut().pop();
            ControlFlow::Continue(())
        },
    )
    .break_value()?;

    let mut path = path.into_inner();

    path.0.remove(0);

    Some((component, path))
}
