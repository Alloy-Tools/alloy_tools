use crate::{
    ComponentState, ComponentStateTrait, ComponentTrait, ComponentType, CoordType, InputEvent,
    Layout, LayoutType, Rect, Renderer, RendererError, SizeConstraints, Style,
};
use al_core::Command;
use std::pin::Pin;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(bound = "Coord: for<'des> serde::Deserialize<'des>")]
pub struct Pane<Coord: CoordType> {
    layout: Layout<Coord>,
    layout_dirty: bool,
    components: Vec<Box<dyn ComponentType<Coord>>>,
}

impl<Coord: CoordType> Pane<Coord> {
    pub fn new(layout: Layout<Coord>, components: Vec<Box<dyn ComponentType<Coord>>>) -> Self {
        Self {
            layout,
            layout_dirty: true,
            components,
        }
    }

    //TODO: add ability to add/remove children though StableVec (with generations once added)
}

impl<Coord: CoordType> ComponentTrait<Coord> for Pane<Coord> {
    //TODO: rather than simple bool returns, add more context eg: (handled, layout_dirty, render_dirty)
    fn handle_event_with_state(
        &mut self,
        state: &mut ComponentState<Coord>,
        event: InputEvent,
    ) -> bool {
        for child in &mut self.components {
            if child.handle_event(event) {
                state.mark_render_dirty();
                self.layout_dirty = true;
                return true;
            }
        }
        false
    }

    fn handle_command_with_state(
        &mut self,
        _: &mut ComponentState<Coord>,
        cmd: Command,
    ) -> Pin<Box<dyn std::future::Future<Output = ()>>> {
        // For simplicity, delegate to first child that might handle it.
        // TODO: later dispatch to the focused child or specific recipients.
        if let Some(child) = self.components.first_mut() {
            child.handle_command(cmd)
        } else {
            Box::pin(async {})
        }
    }

    fn render_with_state(
        &mut self,
        state: &mut ComponentState<Coord>,
        renderer: &mut dyn Renderer<Coord = Coord>,
        style: Style,
        clip: Rect<Coord>,
    ) -> Result<(), RendererError> {
        // Recompute layout if dirty
        if self.layout_dirty {
            if let Some(rect) = state.rect() {
                self.layout.layout(*rect, self.components.as_mut_slice());
                state.mark_render_dirty();
            }
            self.layout_dirty = false;
        }

        // Render children if dirty
        if state.is_render_dirty() {
            for child in &mut self.components {
                let child_clip = clip.intersection(child.rect().unwrap_or_default());
                child.render(renderer, style, child_clip)?;
            }
        }

        Ok(())
    }

    fn size_with_state(
        &self,
        _: &ComponentState<Coord>,
        constraints: SizeConstraints<Coord>,
    ) -> (Coord, Coord) {
        let (width, height) = self.layout.size(&self.components, constraints);
        constraints.clip(width, height)
    }

    fn clone_component(&self) -> Box<dyn ComponentTrait<Coord>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_component(self) -> Box<dyn ComponentType<Coord>> {
        Box::new(crate::Component::new(self))
    }

    fn components_equal(&self, other: &dyn ComponentTrait<Coord>) -> bool {
        if let Some(pane) = other.as_any().downcast_ref::<Pane<Coord>>() {
            self == pane
        } else {
            false
        }
    }
}
