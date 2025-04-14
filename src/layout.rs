use std::{fmt::Write, ops::ControlFlow};

use ratatui::layout::{Offset, Position, Rect};
use taffy::{
    CacheTree, LayoutBlockContainer, LayoutFlexboxContainer, LayoutGridContainer,
    LayoutPartialTree, PrintTree, RoundTree, TraversePartialTree, TraverseTree,
};
use tracing::Level;

use crate::component::{self, ComponentId, DefaultDrawableComponent};

impl From<ComponentId> for taffy::NodeId {
    fn from(value: ComponentId) -> Self {
        taffy::NodeId::new(value.0)
    }
}

impl From<taffy::NodeId> for ComponentId {
    fn from(value: taffy::NodeId) -> Self {
        ComponentId(value.into())
    }
}

pub trait SizeExt {
    fn into_taffy<T: From<u16>>(self) -> taffy::Size<T>;
}

impl SizeExt for ratatui::layout::Size {
    fn into_taffy<T: From<u16>>(self) -> taffy::Size<T> {
        taffy::Size {
            width: self.width.into(),
            height: self.height.into(),
        }
    }
}

pub trait PositionExt {
    fn as_offset(self) -> Offset;
}

impl PositionExt for Position {
    fn as_offset(self) -> Offset {
        Offset {
            x: self.x as i32,
            y: self.y as i32,
        }
    }
}

pub trait LayoutExt {
    fn content_rect(self) -> Rect;
    fn padding_rect(self) -> Rect;
    fn border_rect(self) -> Rect;
}

impl LayoutExt for taffy::Layout {
    fn content_rect(self) -> Rect {
        Rect {
            x: self.content_box_x() as u16,
            y: self.content_box_y() as u16,
            width: self.content_box_width() as u16,
            height: self.content_box_height() as u16,
        }
    }

    fn padding_rect(self) -> Rect {
        Rect {
            x: (self.location.x + self.border.left) as u16,
            y: (self.location.y + self.border.top) as u16,
            width: (self.size.width - self.border.left - self.border.right) as u16,
            height: (self.size.height - self.border.top - self.border.bottom) as u16,
        }
    }

    fn border_rect(self) -> Rect {
        Rect {
            x: self.location.x as u16,
            y: self.location.y as u16,
            width: self.size.width as u16,
            height: self.size.height as u16,
        }
    }
}

/// An absolute-positioned layout.
#[derive(Default, Debug, Clone)]
pub struct AbsoluteLayout {
    /// The rectangle containing the content.
    pub(self) content_rect: Rect,
    /// The rectangle containing the padding, and the content.
    pub(self) padding_rect: Rect,
    /// The outermost rectangle containing the border, the padding, and the content.
    pub(self) border_rect: Rect,
}

impl AbsoluteLayout {
    pub fn content_rect(&self) -> Rect {
        self.content_rect
    }

    pub fn padding_rect(&self) -> Rect {
        self.padding_rect
    }

    pub fn border_rect(&self) -> Rect {
        self.border_rect
    }
}

#[derive(Default, Debug, Clone)]
pub struct TaffyNodeData {
    pub style: taffy::Style,
    unrounded_layout: taffy::Layout,
    rounded_layout: taffy::Layout,
    cache: taffy::Cache,
    detailed_grid_info: Option<taffy::DetailedGridInfo>,
    absolute_layout: AbsoluteLayout,
    cache_dirty: bool,
}

impl TaffyNodeData {
    pub fn new(style: taffy::Style) -> Self {
        Self {
            style,
            ..Default::default()
        }
    }

    pub fn absolute_layout(&self) -> &AbsoluteLayout {
        &self.absolute_layout
    }

    pub fn mark_cached_layout_dirty(&mut self) {
        self.cache_dirty = true;
    }

    fn clear_cache(&mut self) {
        self.cache.clear();
        self.cache_dirty = false;
    }
}

impl TraversePartialTree for Box<dyn DefaultDrawableComponent> {
    type ChildIter<'a>
        = <Vec<taffy::NodeId> as IntoIterator>::IntoIter
    where
        Self: 'a;

    fn child_ids(&self, parent_node_id: taffy::NodeId) -> Self::ChildIter<'_> {
        let Some((parent_node, _id_path)) =
            component::find_component_by_id(self.as_ref(), parent_node_id.into())
        else {
            return Default::default();
        };
        let mut child_ids = Vec::<taffy::NodeId>::new();
        let _ = parent_node.for_each_child(&mut |child| {
            child_ids.push(child.get_id().into());
            ControlFlow::Continue(())
        });
        child_ids.into_iter()
    }

    fn child_count(&self, parent_node_id: taffy::NodeId) -> usize {
        let Some((parent_node, _id_path)) =
            component::find_component_by_id(self.as_ref(), parent_node_id.into())
        else {
            return Default::default();
        };
        let mut child_count = 0;
        let _ = parent_node.for_each_child(&mut |_| {
            child_count += 1;
            ControlFlow::Continue(())
        });
        child_count
    }

    fn get_child_id(&self, parent_node_id: taffy::NodeId, child_index: usize) -> taffy::NodeId {
        let Some((parent_node, _id_path)) =
            component::find_component_by_id(self.as_ref(), parent_node_id.into())
        else {
            panic!("parent node not found");
        };
        let mut child_count = 0;
        let mut child_id = None;
        let _ = parent_node.for_each_child(&mut |child| {
            if child_count == child_index {
                child_id = Some(child.get_id());
                ControlFlow::Break(())
            } else {
                child_count += 1;
                ControlFlow::Continue(())
            }
        });
        child_id.expect("child index out of bounds").into()
    }
}

impl TraverseTree for Box<dyn DefaultDrawableComponent> {}

impl PrintTree for Box<dyn DefaultDrawableComponent> {
    fn get_debug_label(&self, node_id: taffy::NodeId) -> &'static str {
        let (node, _) = component::find_component_by_id(self.as_ref(), node_id.into()).unwrap();
        node.get_debug_label()
    }

    fn get_final_layout(&self, node_id: taffy::NodeId) -> &taffy::Layout {
        let (node, _) = component::find_component_by_id(self.as_ref(), node_id.into()).unwrap();
        &node.get_taffy_node_data().rounded_layout
    }
}

impl CacheTree for Box<dyn DefaultDrawableComponent> {
    fn cache_get(
        &self,
        node_id: taffy::NodeId,
        known_dimensions: taffy::Size<Option<f32>>,
        available_space: taffy::Size<taffy::AvailableSpace>,
        run_mode: taffy::RunMode,
    ) -> Option<taffy::LayoutOutput> {
        let (node, _) = component::find_component_by_id(self.as_ref(), node_id.into())?;
        node.get_taffy_node_data()
            .cache
            .get(known_dimensions, available_space, run_mode)
    }

    fn cache_store(
        &mut self,
        node_id: taffy::NodeId,
        known_dimensions: taffy::Size<Option<f32>>,
        available_space: taffy::Size<taffy::AvailableSpace>,
        run_mode: taffy::RunMode,
        layout_output: taffy::LayoutOutput,
    ) {
        let Some((node, _)) = component::find_component_by_id_mut(self.as_mut(), node_id.into())
        else {
            return;
        };
        node.get_taffy_node_data_mut().cache.store(
            known_dimensions,
            available_space,
            run_mode,
            layout_output,
        );
    }

    fn cache_clear(&mut self, node_id: taffy::NodeId) {
        let Some((node, _)) = component::find_component_by_id_mut(self.as_mut(), node_id.into())
        else {
            return;
        };
        node.get_taffy_node_data_mut().cache.clear();
    }
}

impl RoundTree for Box<dyn DefaultDrawableComponent> {
    fn get_unrounded_layout(&self, node_id: taffy::NodeId) -> &taffy::Layout {
        let (node, _) = component::find_component_by_id(self.as_ref(), node_id.into()).unwrap();
        &node.get_taffy_node_data().unrounded_layout
    }

    fn set_final_layout(&mut self, node_id: taffy::NodeId, layout: &taffy::Layout) {
        let (node, _) = component::find_component_by_id_mut(self.as_mut(), node_id.into()).unwrap();
        node.get_taffy_node_data_mut().rounded_layout = *layout;
    }
}

impl LayoutPartialTree for Box<dyn DefaultDrawableComponent> {
    type CoreContainerStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;

    fn get_core_container_style(&self, node_id: taffy::NodeId) -> Self::CoreContainerStyle<'_> {
        let (node, _) = component::find_component_by_id(self.as_ref(), node_id.into()).unwrap();
        &node.get_taffy_node_data().style
    }

    fn set_unrounded_layout(&mut self, node_id: taffy::NodeId, layout: &taffy::Layout) {
        let (node, _) = component::find_component_by_id_mut(self.as_mut(), node_id.into()).unwrap();
        node.get_taffy_node_data_mut().unrounded_layout = *layout;
    }

    fn compute_child_layout(
        &mut self,
        node_id: taffy::NodeId,
        inputs: taffy::LayoutInput,
    ) -> taffy::LayoutOutput {
        // If RunMode is PerformHiddenLayout then this indicates that an ancestor node is `Display::None`
        // and thus that we should lay out this node using hidden layout regardless of it's own display style.
        if inputs.run_mode == taffy::RunMode::PerformHiddenLayout {
            return taffy::compute_hidden_layout(self, node_id);
        }

        // We run the following wrapped in "compute_cached_layout", which will check the cache for an entry matching the node and inputs and:
        //   - Return that entry if exists
        //   - Else call the passed closure (below) to compute the result
        //
        // If there was no cache match and a new result needs to be computed then that result will be added to the cache
        taffy::compute_cached_layout(self, node_id, inputs, |tree, node_id, inputs| {
            let (node, _) =
                component::find_component_by_id_mut(tree.as_mut(), node_id.into()).unwrap();
            let has_children = !node.get_children().is_empty();
            let display_mode = node.get_taffy_node_data().style.display;

            // Dispatch to a layout algorithm based on the node's display style and whether the node has children or not.
            match (display_mode, has_children) {
                (taffy::Display::None, _) => taffy::compute_hidden_layout(tree, node_id),
                (taffy::Display::Block, true) => taffy::compute_block_layout(tree, node_id, inputs),
                (taffy::Display::Flex, true) => {
                    taffy::compute_flexbox_layout(tree, node_id, inputs)
                }
                (taffy::Display::Grid, true) => taffy::compute_grid_layout(tree, node_id, inputs),
                (_, false) => {
                    let style = &node.get_taffy_node_data().style;
                    let measure_function = |known_dimensions, available_space| {
                        node.measure(known_dimensions, available_space)
                    };

                    // TODO: implement calc() in high-level API
                    taffy::compute_leaf_layout(inputs, style, |_, _| 0.0, measure_function)
                }
            }
        })
    }
}

impl LayoutBlockContainer for Box<dyn DefaultDrawableComponent> {
    type BlockContainerStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;
    type BlockItemStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;

    #[inline(always)]
    fn get_block_container_style(&self, node_id: taffy::NodeId) -> Self::BlockContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    #[inline(always)]
    fn get_block_child_style(&self, child_node_id: taffy::NodeId) -> Self::BlockItemStyle<'_> {
        self.get_core_container_style(child_node_id)
    }
}

impl LayoutFlexboxContainer for Box<dyn DefaultDrawableComponent> {
    type FlexboxContainerStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;
    type FlexboxItemStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;

    #[inline(always)]
    fn get_flexbox_container_style(
        &self,
        node_id: taffy::NodeId,
    ) -> Self::FlexboxContainerStyle<'_> {
        let (node, _) = component::find_component_by_id(self.as_ref(), node_id.into()).unwrap();
        &node.get_taffy_node_data().style
    }

    #[inline(always)]
    fn get_flexbox_child_style(&self, child_node_id: taffy::NodeId) -> Self::FlexboxItemStyle<'_> {
        let (node, _) =
            component::find_component_by_id(self.as_ref(), child_node_id.into()).unwrap();
        &node.get_taffy_node_data().style
    }
}

impl LayoutGridContainer for Box<dyn DefaultDrawableComponent> {
    type GridContainerStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;
    type GridItemStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;

    #[inline(always)]
    fn get_grid_container_style(&self, node_id: taffy::NodeId) -> Self::GridContainerStyle<'_> {
        let (node, _) = component::find_component_by_id(self.as_ref(), node_id.into()).unwrap();
        &node.get_taffy_node_data().style
    }

    #[inline(always)]
    fn get_grid_child_style(&self, child_node_id: taffy::NodeId) -> Self::GridItemStyle<'_> {
        let (node, _) =
            component::find_component_by_id(self.as_ref(), child_node_id.into()).unwrap();
        &node.get_taffy_node_data().style
    }

    #[inline(always)]
    fn set_detailed_grid_info(
        &mut self,
        node_id: taffy::NodeId,
        detailed_grid_info: taffy::DetailedGridInfo,
    ) {
        let (node, _) = component::find_component_by_id_mut(self.as_mut(), node_id.into()).unwrap();
        node.get_taffy_node_data_mut().detailed_grid_info = Some(detailed_grid_info);
    }
}

pub fn clear_dirty_cache(root_component: &mut dyn DefaultDrawableComponent) {
    let _ = component::depth_first_search_with_data_mut::<(), (), bool>(
        root_component,
        &(),
        &mut |_, _| ControlFlow::Continue(()),
        &mut |component, children_dirty| {
            let dirty =
                children_dirty.contains(&true) || component.get_taffy_node_data().cache_dirty;

            if dirty {
                component.get_taffy_node_data_mut().clear_cache();
            }

            ControlFlow::Continue(dirty)
        },
    );
}

pub fn compute_absolute_layout(
    root_component: &mut dyn DefaultDrawableComponent,
    frame_area: Rect,
) {
    let _ = component::depth_first_search_with_data_mut::<(), Rect, ()>(
        root_component,
        &frame_area,
        &mut |component, parent_area| {
            let taffy_node_data = component.get_taffy_node_data_mut();
            let layout = &taffy_node_data.rounded_layout;
            let absolute_layout = &mut taffy_node_data.absolute_layout;
            absolute_layout.content_rect = layout
                .content_rect()
                .offset(parent_area.as_position().as_offset());
            absolute_layout.padding_rect = layout
                .padding_rect()
                .offset(parent_area.as_position().as_offset());
            absolute_layout.border_rect = layout
                .border_rect()
                .offset(parent_area.as_position().as_offset());
            ControlFlow::Continue(absolute_layout.padding_rect)
        },
        &mut |_, _| ControlFlow::Continue(()),
    );
}

#[cfg(feature = "debug")]
pub fn trace_tree_custom(root: &dyn DefaultDrawableComponent) {
    struct PreorderData {
        lines: String,
        last_child_id: ComponentId,
        first: bool,
    }

    let init = PreorderData {
        lines: "".to_string(),
        last_child_id: root.get_id(),
        first: true,
    };
    let mut buffer_string = String::new();
    let _ = component::depth_first_search_with_data::<(), PreorderData, ()>(
        root,
        &init,
        &mut |component, preorder_data| {
            let taffy_node_data = component.get_taffy_node_data();
            let rounded_layout = &taffy_node_data.rounded_layout;
            let absolute_layout = &taffy_node_data.absolute_layout;
            writeln! {
                &mut buffer_string,
                "{lines}{fork}{label} [ xr: {xr}, yr: {yr}, xa: {xa}, ya: {ya}, w: {w}, h: {h} ]",
                lines = preorder_data.lines,
                label = component.get_debug_label(),
                fork = if preorder_data.first {
                    ""
                } else if component.get_id() == preorder_data.last_child_id {
                    "└──"
                } else {
                    "├──"
                },
                xr = rounded_layout.location.x,
                yr = rounded_layout.location.y,
                xa = absolute_layout.border_rect.x,
                ya = absolute_layout.border_rect.y,
                w = absolute_layout.border_rect.width,
                h = absolute_layout.border_rect.height,
            }
            .unwrap();
            ControlFlow::Continue(PreorderData {
                lines: format! {
                    "{lines}{bar}",
                    lines = preorder_data.lines,
                    bar = if preorder_data.first {
                        ""
                    } else if component.get_id() == preorder_data.last_child_id {
                        "    "
                    } else {
                        "│   "
                    }
                },
                last_child_id: component
                    .get_children()
                    .last()
                    .map(|component| component.get_id())
                    .unwrap_or_default(),
                first: false,
            })
        },
        &mut |_, _| ControlFlow::Continue(()),
    );

    tracing::trace!("\n{buffer_string}");
}

/// Based on `taffy::print_tree`.
#[cfg(feature = "debug")]
pub fn trace_tree(tree: &impl PrintTree, root: taffy::NodeId) {
    let mut buffer_string = "\nTREE\n".to_string();
    print_node(tree, root, false, String::new(), &mut buffer_string).unwrap();

    /// Recursive function that prints each node in the tree
    fn print_node(
        tree: &impl PrintTree,
        node_id: taffy::NodeId,
        has_sibling: bool,
        lines_string: String,
        buffer_string: &mut String,
    ) -> std::fmt::Result {
        let layout = &tree.get_final_layout(node_id);
        let display2: &'static str = tree.get_debug_label(node_id);
        let num_children = tree.child_count(node_id);

        let fork_string = if has_sibling {
            "├──"
        } else {
            "└──"
        };
        #[cfg(feature = "debug_layout_content_size")]
        writeln!(
            buffer_string,
            "{lines}{fork} {display} [x: {x} y: {y} w: {width} h: {height} content_w: {content_width} content_h: {content_height} border: l:{bl} r:{br} t:{bt} b:{bb}, padding: l:{pl} r:{pr} t:{pt} b:{pb}] ({key:?})",
            lines = lines_string,
            fork = fork_string,
            display = display2,
            x = layout.location.x,
            y = layout.location.y,
            width = layout.size.width,
            height = layout.size.height,
            content_width = layout.content_size.width,
            content_height = layout.content_size.height,
            bl = layout.border.left,
            br = layout.border.right,
            bt = layout.border.top,
            bb = layout.border.bottom,
            pl = layout.padding.left,
            pr = layout.padding.right,
            pt = layout.padding.top,
            pb = layout.padding.bottom,
            key = node_id,
        )?;
        #[cfg(not(feature = "debug_layout_content_size"))]
        writeln!(
            buffer_string,
            "{lines}{fork} {display} [x: {x} y: {y} width: {width} height: {height}] ({key:?})",
            lines = lines_string,
            fork = fork_string,
            display = display2,
            x = layout.location.x,
            y = layout.location.y,
            width = layout.size.width,
            height = layout.size.height,
            key = node_id,
        )?;
        let bar = if has_sibling { "│   " } else { "    " };
        let new_string = lines_string + bar;

        // Recurse into children
        for (index, child) in tree.child_ids(node_id).enumerate() {
            let has_sibling = index < num_children - 1;
            print_node(tree, child, has_sibling, new_string.clone(), buffer_string)?;
        }

        Ok(())
    }

    tracing::trace!("{}", buffer_string);
}
