use std::{
    cell::RefCell,
    fmt::Debug,
    ops::ControlFlow,
    time::{Duration, Instant},
};

use color_eyre::Result;
use nalgebra::{Point, SVector, zero};
use ratatui::{
    Frame,
    buffer::{Buffer, Cell},
    layout::{Position, Rect, Size},
    style::Style,
    widgets::{StatefulWidgetRef, WidgetRef},
};
use static_assertions::assert_obj_safe;
use tracing::instrument;

use crate::{
    action::{Action, ComponentMessage},
    geometry::{
        Rectangle,
        ext::{IntoRatatuiExt, nalgebra::PointExt},
    },
    layout::{AbsoluteLayout, TaffyNodeData},
    tui::Event,
};

mod id {
    use std::sync::atomic::{AtomicU64, Ordering};

    use derive_deref::{Deref, DerefMut};
    use polonius_the_crab::{polonius_loop, polonius_return};

    use super::*;

    static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

    #[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
    pub struct ComponentId(pub(crate) u64);

    impl Default for ComponentId {
        fn default() -> Self {
            Self::new()
        }
    }

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
        #[expect(unused)]
        pub fn find_deepest_available_component<'a>(
            &self,
            root: &'a dyn super::Component,
        ) -> (&'a dyn super::Component, Self) {
            pub fn find_deepest_available_component_inner<'a>(
                path: &[ComponentId],
                root: &'a dyn super::Component,
                depth: usize,
            ) -> (&'a dyn super::Component, usize) {
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

        pub fn for_each_component_mut<B>(
            &self,
            root: &mut dyn super::Component,
            visit_preorder: &mut impl FnMut(&mut dyn Component) -> ControlFlow<B>,
            visit_postorder: &mut impl FnMut(&mut dyn Component) -> ControlFlow<B>,
        ) -> ControlFlow<B> {
            (visit_preorder)(root)?;

            if let Some((head, tail)) = self.0.split_first() {
                for child in root.get_children_mut() {
                    if child.get_id() == *head {
                        Self(tail.into()).for_each_component_mut(
                            child,
                            visit_preorder,
                            visit_postorder,
                        )?;
                    }
                }
            }

            (visit_postorder)(root)?;
            ControlFlow::Continue(())
        }
    }
}

pub use id::ComponentId;
pub use id::ComponentIdPath;

#[derive(Default)]
pub struct HandleEventSuccess {
    pub action: Option<Action>,
    /// `true` if the event should not be propagated upwards toward the root.
    pub absorb: bool,
}

impl HandleEventSuccess {
    pub fn unhandled() -> Self {
        Default::default()
    }

    pub fn handled() -> Self {
        Self {
            action: None,
            absorb: true,
        }
    }

    pub fn with_action(self, action: Action) -> Self {
        Self {
            action: Some(action),
            ..self
        }
    }
}

/// `Component` is a trait that represents a visual and interactive element of the user interface.
///
/// Implementors of this trait can be registered with the main application loop and will be able to
/// receive events, update state, and be rendered on the screen.
///
/// A component's layout is computed using the advanced layouting engine Taffy.
pub trait Component: Debug {
    /// Handle events when focused.
    fn handle_event(&mut self, _event: &Event) -> Result<HandleEventSuccess> {
        Ok(HandleEventSuccess::unhandled())
    }

    fn update(&mut self, _message: ComponentMessage) -> Result<Option<Action>> {
        Ok(None)
    }

    /// Returns the immutable unique ID of this component's instance.
    fn get_id(&self) -> ComponentId;

    // TODO: Accesskit support
    fn get_accessibility_node(&self) -> Result<accesskit::Node> {
        todo!()
    }

    /// Returns `true` iff this component can be focused such that it is able to handle events.
    fn is_focusable(&self) -> bool {
        false
    }

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

    fn get_taffy_node_data(&self) -> &TaffyNodeData;

    fn get_taffy_node_data_mut(&mut self) -> &mut TaffyNodeData;

    fn measure(
        &self,
        _known_dimensions: taffy::Size<Option<f32>>,
        _available_space: taffy::Size<taffy::AvailableSpace>,
    ) -> taffy::Size<f32> {
        Default::default()
    }

    fn scroll_position(&self) -> SVector<u16, 2> {
        Default::default()
    }

    fn on_absolute_layout_updated(&mut self) {}

    fn get_debug_label(&self) -> &'static str {
        // std::any::type_name::<Self>()

        // Strip the first absolute path.
        let type_name = std::any::type_name::<Self>();
        let len = type_name.find(['<', '>']).unwrap_or(type_name.len());
        let start_index = type_name[0..len]
            .rfind("::")
            .map(|index| index + 2)
            .unwrap_or(0);

        &type_name[start_index..]
    }
}

pub trait ComponentExt {
    fn with_style(self, f: impl FnOnce(taffy::Style) -> taffy::Style) -> Self
    where
        Self: Sized;

    fn absolute_layout(&self) -> &AbsoluteLayout;
    fn mark_cached_layout_dirty(&mut self);
}

impl<T: Component> ComponentExt for T {
    fn with_style(mut self, f: impl FnOnce(taffy::Style) -> taffy::Style) -> Self {
        let style = &mut self.get_taffy_node_data_mut().style;
        *style = (f)(style.clone());
        self
    }

    fn absolute_layout(&self) -> &AbsoluteLayout {
        self.get_taffy_node_data().absolute_layout()
    }

    fn mark_cached_layout_dirty(&mut self) {
        self.get_taffy_node_data_mut()
            .mark_cached_relative_layout_dirty();
    }
}

pub trait BufferExt {
    fn blit(
        &mut self,
        other: &Self,
        position_src: impl Into<Position>,
        position_dst: impl Into<Position>,
        size: impl Into<Size>,
    );
    fn index_of_opt_alt(&self, position: Position) -> Option<usize>;
}

impl BufferExt for Buffer {
    fn index_of_opt_alt(&self, position: Position) -> Option<usize> {
        let area = self.area;
        if !area.contains(position) {
            return None;
        }
        // remove offset
        let y = (position.y - self.area.y) as usize;
        let x = (position.x - self.area.x) as usize;
        let width = self.area.width as usize;
        Some(y * width + x)
    }

    #[instrument(
        level = "trace",
        skip(self, position_src, position_dst, size),
        fields(position_src, position_dst, size)
    )]
    fn blit(
        &mut self,
        other: &Self,
        position_src: impl Into<Position>,
        position_dst: impl Into<Position>,
        size: impl Into<Size>,
    ) {
        let position_src = position_src.into();
        let position_dst = position_dst.into();
        let mut size = size.into();

        {
            let span = tracing::Span::current();
            span.record("position_src", position_src.to_string());
            span.record("position_dst", position_dst.to_string());
            span.record("size", size.to_string());
        }

        size.width = std::cmp::min(
            size.width,
            std::cmp::min(
                self.area.width.saturating_sub(position_dst.x),
                other.area.width.saturating_sub(position_src.x),
            ),
        );
        size.height = std::cmp::min(
            size.height,
            std::cmp::min(
                self.area.height.saturating_sub(position_dst.y),
                other.area.height.saturating_sub(position_src.y),
            ),
        );
        for y in 0..size.height {
            for x in 0..size.width {
                let src_x = position_src.x + x;
                let src_y = position_src.y + y;

                // let Some(src_index) = other.index_of_opt_alt(Position::new(src_x, src_y)) else {
                //     continue;
                // };

                let dst_x = x + position_dst.x;
                let dst_y = y + position_dst.y;

                // let Some(dst_index) = self.index_of_opt_alt(Position::new(dst_x, dst_y)) else {
                //     continue;
                // };

                self[(dst_x, dst_y)] = other[(src_x, src_y)].clone();
                // self.content[dst_index] = other.content[src_index].clone();
                // self[(dst_x, dst_y)].set_style(other[(src_x, src_y)].style());
            }
        }
    }
}

#[derive(Debug)]
pub struct DrawContext<'a, 'b: 'a> {
    frame: &'a mut Frame<'b>,
    /// The path of component ID's to the currently focused leaf component.
    focused_path: &'a ComponentIdPath,
    /// The instant at which the rendering of the corresponding frame started.
    now: Instant,
    /// Time elapsed since the app was launched until `now`.
    elapsed_time: Duration,
    /// The clipping area of the currently drawn component.
    view: Rectangle<u16>,
    /// The depth of the current component.
    current_depth: usize,
}

impl<'a, 'b: 'a> DrawContext<'a, 'b> {
    pub fn new(
        frame: &'a mut Frame<'b>,
        focused_path: &'a ComponentIdPath,
        now: Instant,
        elapsed_time: Duration,
    ) -> Self {
        Self {
            view: frame.area().into(),
            frame,
            focused_path,
            now,
            elapsed_time,
            current_depth: 0,
        }
    }

    // pub fn frame(&mut self) -> &mut Frame<'b> {
    //     self.frame
    // }

    pub fn focused_id(&self) -> ComponentId {
        *self.focused_path.0.last().unwrap()
    }

    pub fn focused_path(&self) -> &ComponentIdPath {
        self.focused_path
    }

    pub fn is_child_focused(&self, id: ComponentId) -> bool {
        self.current_depth
            .checked_sub(1)
            .and_then(|depth| self.focused_path.get(depth))
            == Some(&id)
    }

    pub fn now(&self) -> Instant {
        self.now
    }

    pub fn elapsed_time(&self) -> Duration {
        self.elapsed_time
    }

    pub fn view(&self) -> Rectangle<u16> {
        self.view
    }

    pub fn get_cell_mut(&mut self, position: impl Into<Point<i16, 2>>) -> Option<&mut Cell> {
        let position = position
            .into()
            .sup(&Point::origin())
            .try_cast::<u16>()
            .unwrap();
        if self.view.contains(position) {
            self.frame.buffer_mut().cell_mut(position.into_ratatui())
        } else {
            None
        }
    }

    pub fn for_each_cell_in_mut(&mut self, area: Rectangle<i16>, mut f: impl FnMut(&mut Cell)) {
        let clipped_area = self.view.intersect(&area.clip());

        for y in clipped_area.min().y..clipped_area.max().y {
            for x in clipped_area.min().x..clipped_area.max().x {
                if let Some(cell) = self.frame.buffer_mut().cell_mut(Position::new(x, y)) {
                    (f)(cell)
                }
            }
        }
    }

    pub fn set_style(&mut self, area: impl Into<Rectangle<i16>>, style: impl Into<Style>) {
        let clipped_area = area.into().clip().intersect(&self.view);
        if !clipped_area.is_empty() {
            self.frame
                .buffer_mut()
                .set_style(clipped_area.into(), style);
        }
    }

    pub fn draw_widget_inner(
        &mut self,
        area: impl Into<Rectangle<i16>>,
        render_into: impl FnOnce(Rect, &mut Buffer),
    ) {
        let area: Rectangle<i16> = area.into();
        let clipped_area = area.intersect(&self.view.cast::<i16>());

        if clipped_area.is_empty() {
            // The entire widget is clipped. Nothing to render.
            return;
        }

        if area.min() == clipped_area.min() {
            // The top left corner of the widget is not clipped,
            // therefore the widget can be rendered the simple way.
            (render_into)(clipped_area.clip().into(), self.frame.buffer_mut());
            return;
        }

        // At this point, the top left corner of the widget is clipped.
        // The widget needs to be rendered into an intermediate buffer, to not overwrite existing
        // data of the buffer in the clipped region.

        let unused_margin = (clipped_area.min() - area.min())
            .sup(&zero())
            .try_cast::<u16>()
            .expect("`sup` ensures non-negative values");
        let draw_area = Rectangle::from_extent(
            Point::origin(),
            unused_margin
                + clipped_area
                    .extent()
                    .try_cast::<u16>()
                    .expect("the extent is always non-negative"),
        );
        let mut tmp_buffer = Buffer::empty(draw_area.into());
        tmp_buffer.blit(
            self.frame.buffer_mut(),
            clipped_area.min().try_cast::<u16>().unwrap().into_ratatui(),
            Point {
                coords: unused_margin,
            }
            .into_ratatui(),
            clipped_area
                .extent()
                .try_cast::<u16>()
                .unwrap()
                .into_ratatui(),
        );
        (render_into)(draw_area.into(), &mut tmp_buffer);
        self.frame.buffer_mut().blit(
            &tmp_buffer,
            Point {
                coords: unused_margin,
            }
            .into_ratatui(),
            clipped_area.min().try_cast::<u16>().unwrap().into_ratatui(),
            clipped_area
                .extent()
                .try_cast::<u16>()
                .unwrap()
                .into_ratatui(),
        );
    }

    pub fn draw_widget<W: WidgetRef + Debug>(
        &mut self,
        widget: &W,
        area: impl Into<Rectangle<i16>>,
    ) {
        self.draw_widget_inner(area, |clipped_area, buffer| {
            widget.render_ref(clipped_area, buffer)
        });
    }

    pub fn draw_stateful_widget<W: StatefulWidgetRef>(
        &mut self,
        widget: &W,
        area: impl Into<Rectangle<i16>>,
        state: &mut W::State,
    ) {
        self.draw_widget_inner(area, |clipped_area, buffer| {
            widget.render_ref(clipped_area, buffer, state)
        });
    }

    fn draw_component_impl<C: Component + ?Sized>(
        &mut self,
        component: &C,
        f: impl FnOnce(DrawContext<'_, '_>) -> Result<()>,
    ) -> Result<()> {
        let content_rect = component
            .get_taffy_node_data()
            .absolute_layout()
            .overflow_rect_clip();
        if content_rect.is_empty() {
            Ok(())
        } else {
            (f)(DrawContext {
                frame: self.frame,
                elapsed_time: self.elapsed_time,
                focused_path: self.focused_path,
                now: self.now,
                view: content_rect.clip(),
                current_depth: self.current_depth + 1,
            })
        }
    }

    pub fn draw_component_with<C: Drawable + Component>(
        &mut self,
        component: &C,
        extra_args: C::Args<'_>,
    ) -> Result<()> {
        self.draw_component_impl(component, move |mut context| -> Result<()> {
            #[expect(deprecated)]
            component.draw(&mut context, extra_args)
        })
    }

    pub fn draw_component<C: DefaultDrawableComponent + ?Sized>(
        &mut self,
        component: &C,
    ) -> Result<()> {
        self.draw_component_impl(component, move |mut context| -> Result<()> {
            #[expect(deprecated)]
            component.default_draw(&mut context)
        })
    }
}

/// Used for type equality constraints in `where` clauses.
trait Is {
    type Type;
    fn noop_from(value: Self::Type) -> Self;
    #[expect(unused)]
    fn noop_into(self) -> Self::Type;
}

impl<T> Is for T {
    type Type = T;

    fn noop_from(value: Self::Type) -> Self {
        value
    }

    fn noop_into(self) -> Self::Type {
        self
    }
}

/// A drawable element (usually a `Component`).
pub trait Drawable {
    type Args<'a>
    where
        Self: 'a;

    // TODO: Rename to `draw_with`
    #[deprecated = "Use DrawContext::draw_component_with instead"]
    fn draw<'a>(&self, context: &mut DrawContext, extra_args: Self::Args<'a>) -> Result<()>
    where
        Self: 'a;
}

/// A drawable element that takes no extra arguments for drawing.
/// As opposed to `Drawable`, this trait is dyn-compatible (object safe).
pub trait DefaultDrawable {
    // TODO: Rename to `draw`
    #[deprecated = "Use DrawContext::draw_component instead"]
    fn default_draw(&self, context: &mut DrawContext) -> Result<()>;
}

impl<T> DefaultDrawable for T
where
    T: Drawable,
    for<'a> <T as Drawable>::Args<'a>: Is<Type = ()>,
{
    fn default_draw(&self, context: &mut DrawContext) -> Result<()> {
        #[expect(deprecated)]
        self.draw(context, Is::noop_from(()))
    }
}

assert_obj_safe!(DefaultDrawable);

pub trait DefaultDrawableComponent: DefaultDrawable + Component {}
impl<T> DefaultDrawableComponent for T where T: DefaultDrawable + Component {}

assert_obj_safe!(DefaultDrawableComponent);

pub enum TreeControlFlow<B, C> {
    Continue(C),
    SkipChildren,
    Break(B),
}

impl<B, C> From<ControlFlow<B, C>> for TreeControlFlow<B, C> {
    fn from(value: ControlFlow<B, C>) -> Self {
        match value {
            ControlFlow::Continue(inner) => TreeControlFlow::Continue(inner),
            ControlFlow::Break(inner) => TreeControlFlow::Break(inner),
        }
    }
}

// Standalone generic functions folľow, because they cannot be on trait objects.

pub fn for_each_child<'a, B: 'a>(
    component: &'a dyn Component,
    mut f: impl FnMut(&'a dyn Component) -> ControlFlow<B>,
) -> Option<B> {
    let mut result = ControlFlow::Continue(());
    let _ = component.for_each_child(&mut |component| {
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
    let _ = component.for_each_child_mut(&mut |component| {
        result = (f)(component);
        match &result {
            ControlFlow::Break(_) => ControlFlow::Break(()),
            ControlFlow::Continue(_) => ControlFlow::Continue(()),
        }
    });
    result.break_value()
}

#[expect(unused)]
pub fn find_child_by_id(
    component: &dyn Component,
    child_id: ComponentId,
) -> Option<&dyn Component> {
    for_each_child(component, |child| {
        if child.is_focusable() && child.get_id() == child_id {
            ControlFlow::Break(child)
        } else {
            ControlFlow::Continue(())
        }
    })
}

#[expect(unused)]
pub fn find_child_by_id_mut(
    component: &mut dyn Component,
    child_id: ComponentId,
) -> Option<&mut dyn Component> {
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

pub fn depth_first_search_with_data<'a, B: 'a, C1, C2>(
    subtree_root: &'a dyn Component,
    init: &C1,
    visit_preorder: &mut dyn FnMut(&'a dyn Component, &C1) -> ControlFlow<B, C1>,
    visit_postorder: &mut dyn FnMut(&'a dyn Component, Vec<C2>) -> ControlFlow<B, C2>,
) -> ControlFlow<B, C2> {
    let preorder_data = (visit_preorder)(subtree_root, init)?;
    let mut postorder_data_vec = Vec::<C2>::new();
    if let Some(break_value) = for_each_child::<B>(subtree_root, |child| {
        let postorder_data =
            depth_first_search_with_data(child, &preorder_data, visit_preorder, visit_postorder)?;
        postorder_data_vec.push(postorder_data);
        ControlFlow::Continue(())
    }) {
        return ControlFlow::Break(break_value);
    }
    (visit_postorder)(subtree_root, postorder_data_vec)
}

pub fn depth_first_search_with_data_mut<'a, B: 'a, C1, C2>(
    subtree_root: &'a mut dyn Component,
    init: &C1,
    visit_preorder: &mut dyn FnMut(&'a mut dyn Component, &C1) -> TreeControlFlow<B, C1>,
    visit_postorder: &mut dyn FnMut(&'a mut dyn Component, Option<Vec<C2>>) -> ControlFlow<B, C2>,
) -> ControlFlow<B, C2> {
    // Safety:
    // No aliased mutable references actually occur, because the try-operator (`?`) is used to
    // return `ControlFlow::<B>::Break` early.
    // This seems like a case where the Polonius-based borrowck would be required to avoid the use
    // of `unsafe`.
    let subtree_root_ptr = subtree_root as *mut dyn Component;
    let preorder_cf = (visit_preorder)(unsafe { &mut *subtree_root_ptr }, init);
    match preorder_cf {
        TreeControlFlow::Continue(preorder_data) => {
            let mut postorder_data_vec = Vec::<C2>::new();
            if let Some(break_value) =
                for_each_child_mut::<B>(unsafe { &mut *subtree_root_ptr }, |child| {
                    let postorder_data = depth_first_search_with_data_mut(
                        child,
                        &preorder_data,
                        visit_preorder,
                        visit_postorder,
                    )?;
                    postorder_data_vec.push(postorder_data);
                    ControlFlow::Continue(())
                })
            {
                return ControlFlow::Break(break_value);
            }

            (visit_postorder)(unsafe { &mut *subtree_root_ptr }, Some(postorder_data_vec))
        }
        TreeControlFlow::SkipChildren => (visit_postorder)(unsafe { &mut *subtree_root_ptr }, None),
        TreeControlFlow::Break(break_value) => ControlFlow::Break(break_value),
    }
}

pub fn find_component_by_id(
    subtree_root: &dyn Component,
    id: ComponentId,
) -> Option<(&dyn Component, ComponentIdPath)> {
    let path = RefCell::new(ComponentIdPath::default());
    let component = depth_first_search(
        subtree_root,
        &mut |component| {
            path.borrow_mut().push(component.get_id());
            if component.get_id() == id {
                return ControlFlow::Break(component);
            }
            ControlFlow::Continue(())
        },
        &mut |_| {
            path.borrow_mut().pop();
            ControlFlow::Continue(())
        },
    )
    .break_value()?;

    let mut path = path.into_inner();

    path.0.remove(0);

    Some((component, path))
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
        &mut |_| {
            path.borrow_mut().pop();
            ControlFlow::Continue(())
        },
    )
    .break_value()?;

    let mut path = path.into_inner();

    path.0.remove(0);

    Some((component, path))
}
