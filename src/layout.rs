use std::{fmt::Debug, ops::ControlFlow};

use nalgebra::{SVector, point, vector};
use ratatui::layout::Rect;
use taffy::{
    CacheTree, LayoutBlockContainer, LayoutFlexboxContainer, LayoutGridContainer,
    LayoutPartialTree, PrintTree, RoundTree, TraversePartialTree, TraverseTree,
};

use crate::{
    component::{self, ComponentId, DefaultDrawableComponent, TreeControlFlow},
    geometry::{
        Rectangle,
        ext::{IntoNalgebra, IntoNalgebraExt},
    },
};

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

pub trait LayoutExt {
    fn content_rect(self) -> Rectangle<i16>;
    fn padding_rect(self) -> Rectangle<i16>;
    fn border_rect(self) -> Rectangle<i16>;
}

impl LayoutExt for taffy::Layout {
    fn content_rect(self) -> Rectangle<i16> {
        Rectangle::from_extent(
            point![self.content_box_x() as i16, self.content_box_y() as i16],
            vector![
                self.content_box_width() as i16,
                self.content_box_height() as i16
            ],
        )
    }

    fn padding_rect(self) -> Rectangle<i16> {
        Rectangle::from_extent(
            point![
                (self.location.x + self.border.left) as i16,
                (self.location.y + self.border.top) as i16
            ],
            vector![
                (self.size.width - self.border.left - self.border.right) as i16,
                (self.size.height - self.border.top - self.border.bottom) as i16
            ],
        )
    }

    fn border_rect(self) -> Rectangle<i16> {
        Rectangle::from_extent(
            point![self.location.x as i16, self.location.y as i16],
            vector![self.size.width as i16, self.size.height as i16],
        )
    }
}

/// An absolute-positioned layout.
#[derive(Default, Debug, Clone)]
pub struct AbsoluteLayout {
    /// The rectangle containing the overflowing content.
    pub(self) overflow_size: SVector<u16, 2>,
    /// An area which the overflow content is clipped to.
    pub(self) overflow_rect_clip: Rectangle<i16>,
    /// The rectangle containing the unclipped content.
    pub(self) content_rect: Rectangle<i16>,
    /// The rectangle containing the padding, and the content.
    pub(self) padding_rect: Rectangle<i16>,
    /// The outermost rectangle containing the border, the padding, and the content.
    pub(self) border_rect: Rectangle<i16>,
    /// The amount of cells scrolled in each axis.
    pub(self) scroll_position: SVector<u16, 2>,
    pub(self) absolute_position_offset: SVector<i16, 2>,
}

impl AbsoluteLayout {
    pub fn overflow_rect_clip(&self) -> Rectangle<i16> {
        self.overflow_rect_clip
    }

    pub fn content_rect(&self) -> Rectangle<i16> {
        self.content_rect
    }

    pub fn padding_rect(&self) -> Rectangle<i16> {
        self.padding_rect
    }

    pub fn border_rect(&self) -> Rectangle<i16> {
        self.border_rect
    }

    pub fn scroll_position(&self) -> SVector<u16, 2> {
        self.scroll_position
    }

    pub fn overflow_size(&self) -> SVector<u16, 2> {
        self.overflow_size
    }

    // pub fn max_content_overflow_rect(&self) -> Rectangle<i16> {
    //     Rectangle::from_minmax(
    //         self.content_rect.min(),
    //         self.content_rect
    //             .max()
    //             .sup(&(self.content_rect.min() + self.overflow_size.cast::<i16>())),
    //     )
    // }
}

#[derive(Default, Debug, Clone)]
pub struct TaffyNodeData {
    pub style: taffy::Style,
    unrounded_layout: taffy::Layout,
    rounded_layout: taffy::Layout,
    cache: taffy::Cache,
    detailed_grid_info: Option<taffy::DetailedGridInfo>,
    absolute_layout: Option<AbsoluteLayout>,
    relative_layout_cache_dirty: bool,
    absolute_layout_of_successors_dirty: bool,
}

impl TaffyNodeData {
    pub fn new(style: taffy::Style) -> Self {
        Self {
            style,
            ..Default::default()
        }
    }

    pub fn absolute_layout(&self) -> &AbsoluteLayout {
        self.absolute_layout_opt()
            .expect("The absolute layout is not computed for this node.")
    }

    pub fn absolute_layout_opt(&self) -> Option<&AbsoluteLayout> {
        self.absolute_layout.as_ref()
    }

    pub fn mark_cached_relative_layout_dirty(&mut self) {
        self.relative_layout_cache_dirty = true;
    }

    pub fn mark_cached_absolute_layout_dirty(&mut self) {
        self.absolute_layout = None;
    }

    fn clear_relative_layout_cache(&mut self) {
        self.cache.clear();
        self.relative_layout_cache_dirty = false;
        self.mark_cached_absolute_layout_dirty();
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
    struct PostorderData {
        relative_dirty: bool,
        absolute_dirty: bool,
    }

    let _ = component::depth_first_search_with_data_mut::<(), (), PostorderData>(
        root_component,
        &(),
        &mut |_, _| TreeControlFlow::Continue(()),
        &mut |component, postorder_data| {
            let postorder_data = postorder_data.expect("children are never skipped");
            let children_relative_dirty = postorder_data
                .iter()
                .any(|postorder_data| postorder_data.relative_dirty);
            let children_absolute_dirty = postorder_data
                .iter()
                .any(|postorder_data| postorder_data.absolute_dirty);
            let relative_dirty = children_relative_dirty
                || component.get_taffy_node_data().relative_layout_cache_dirty;
            let absolute_dirty = children_absolute_dirty
                || component.get_taffy_node_data().absolute_layout.is_none();

            if relative_dirty {
                component
                    .get_taffy_node_data_mut()
                    .clear_relative_layout_cache();
            }

            component
                .get_taffy_node_data_mut()
                .absolute_layout_of_successors_dirty = absolute_dirty;

            ControlFlow::Continue(PostorderData {
                relative_dirty,
                absolute_dirty,
            })
        },
    );
}

pub fn compute_absolute_layout(
    root_component: &mut dyn DefaultDrawableComponent,
    frame_area: Rect,
    previous_frame_area: Option<Rect>,
) {
    struct PreorderData {
        overflow_clip_area: Rectangle<i16>,
        absolute_position_offset: SVector<i16, 2>,
        parent_recomputed: bool,
    }

    if Some(frame_area) != previous_frame_area {
        // Force layout update if the frame changed.
        root_component.get_taffy_node_data_mut().absolute_layout = None;
    }

    let _ = component::depth_first_search_with_data_mut::<(), PreorderData, ()>(
        root_component,
        &PreorderData {
            overflow_clip_area: Rectangle::from(frame_area).cast::<i16>(),
            absolute_position_offset: frame_area.as_position().into_nalgebra_cast::<i16>().coords,
            parent_recomputed: false,
        },
        &mut |component, preorder_data| {
            let absolute_layout_of_successors_dirty = std::mem::replace(
                &mut component
                    .get_taffy_node_data_mut()
                    .absolute_layout_of_successors_dirty,
                false,
            );

            if let Some(absolute_layout) = component.get_taffy_node_data().absolute_layout.as_ref()
                && !preorder_data.parent_recomputed
            {
                // The absolute layout is cached from a previous invocation, no need to recompute it.
                if absolute_layout_of_successors_dirty {
                    // A successor is dirty, keep traversing.
                    return TreeControlFlow::Continue(PreorderData {
                        overflow_clip_area: absolute_layout.overflow_rect_clip,
                        absolute_position_offset: absolute_layout.absolute_position_offset,
                        parent_recomputed: preorder_data.parent_recomputed,
                    });
                } else {
                    // None of the successors are dirty, skip visiting them.
                    return TreeControlFlow::SkipChildren;
                }
            }

            let scroll_position = component.scroll_position();
            let taffy_node_data = component.get_taffy_node_data_mut();
            let layout = &taffy_node_data.rounded_layout;
            let overflow_size = layout
                .content_size
                .into_nalgebra()
                .try_cast::<u16>()
                .unwrap_or_default();
            let content_rect = layout
                .content_rect()
                .translated(preorder_data.absolute_position_offset);
            let padding_rect = layout
                .padding_rect()
                .translated(preorder_data.absolute_position_offset);
            let border_rect = layout
                .border_rect()
                .translated(preorder_data.absolute_position_offset);
            let overflow_rect_clip = preorder_data
                .overflow_clip_area
                .cast::<i16>()
                .intersect(&padding_rect);
            let absolute_position_offset =
                padding_rect.min().cast::<i16>().coords - scroll_position.cast::<i16>();

            taffy_node_data.absolute_layout = Some(AbsoluteLayout {
                overflow_size,
                overflow_rect_clip,
                content_rect,
                padding_rect,
                border_rect,
                scroll_position,
                absolute_position_offset,
            });

            component.on_absolute_layout_updated();

            TreeControlFlow::Continue(PreorderData {
                overflow_clip_area: overflow_rect_clip,
                absolute_position_offset,
                parent_recomputed: true,
            })
        },
        &mut |_, _| ControlFlow::Continue(()),
    );
}

#[cfg(feature = "debug")]
pub fn trace_tree_custom(root: &dyn DefaultDrawableComponent) {
    use std::fmt::Write;

    use crate::component::ComponentId;

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
            if let Some(absolute_layout) = taffy_node_data.absolute_layout.as_ref() {
                writeln! {
                    &mut buffer_string,
                    "{lines}{fork}{label} [ xr: {xr}, yr: {yr}, xa: {xa}, ya: {ya}, w: {w}, h: {h}, wo: {wo}, ho: {ho}, xs: {xs}, ys: {ys} ]",
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
                    xa = absolute_layout.border_rect.min().x,
                    ya = absolute_layout.border_rect.min().y,
                    w = absolute_layout.border_rect.extent().x,
                    h = absolute_layout.border_rect.extent().y,
                    wo = absolute_layout.overflow_size.x,
                    ho = absolute_layout.overflow_size.y,
                    xs = absolute_layout.scroll_position.x,
                    ys = absolute_layout.scroll_position.y,
                }
                .unwrap();
            } else {
                writeln! {
                    &mut buffer_string,
                    "{lines}{fork}{label} [ xr: {xr}, yr: {yr} ]",
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
                }
                .unwrap();
            }
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
