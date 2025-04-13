use std::ops::ControlFlow;

use taffy::{
    CacheTree, LayoutBlockContainer, LayoutFlexboxContainer, LayoutGridContainer,
    LayoutPartialTree, TraversePartialTree, TraverseTree,
};

use crate::component::{self, Component, ComponentId, DefaultDrawableComponent, Drawable};

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

pub struct TaffyNodeData {
    pub core_style: taffy::Style,
    pub unrounded_layout: taffy::Layout,
    pub cache: taffy::Cache,
    pub detailed_grid_info: taffy::DetailedGridInfo,
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

impl LayoutPartialTree for Box<dyn DefaultDrawableComponent> {
    type CoreContainerStyle<'a>
        = &'a taffy::Style
    where
        Self: 'a;

    fn get_core_container_style(&self, node_id: taffy::NodeId) -> Self::CoreContainerStyle<'_> {
        let (node, _) = component::find_component_by_id(self.as_ref(), node_id.into()).unwrap();
        &node.get_taffy_node_data().core_style
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
            let display_mode = node.get_taffy_node_data().core_style.display;

            // Dispatch to a layout algorithm based on the node's display style and whether the node has children or not.
            match (display_mode, has_children) {
                (taffy::Display::None, _) => taffy::compute_hidden_layout(tree, node_id),
                (taffy::Display::Block, true) => taffy::compute_block_layout(tree, node_id, inputs),
                (taffy::Display::Flex, true) => {
                    taffy::compute_flexbox_layout(tree, node_id, inputs)
                }
                (taffy::Display::Grid, true) => taffy::compute_grid_layout(tree, node_id, inputs),
                (_, false) => {
                    let style = &node.get_taffy_node_data().core_style;
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
        &node.get_taffy_node_data().core_style
    }

    #[inline(always)]
    fn get_flexbox_child_style(&self, child_node_id: taffy::NodeId) -> Self::FlexboxItemStyle<'_> {
        let (node, _) =
            component::find_component_by_id(self.as_ref(), child_node_id.into()).unwrap();
        &node.get_taffy_node_data().core_style
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
        &node.get_taffy_node_data().core_style
    }

    #[inline(always)]
    fn get_grid_child_style(&self, child_node_id: taffy::NodeId) -> Self::GridItemStyle<'_> {
        let (node, _) =
            component::find_component_by_id(self.as_ref(), child_node_id.into()).unwrap();
        &node.get_taffy_node_data().core_style
    }

    #[inline(always)]
    fn set_detailed_grid_info(
        &mut self,
        node_id: taffy::NodeId,
        detailed_grid_info: taffy::DetailedGridInfo,
    ) {
        let (node, _) = component::find_component_by_id_mut(self.as_mut(), node_id.into()).unwrap();
        node.get_taffy_node_data_mut().detailed_grid_info = detailed_grid_info;
    }
}
