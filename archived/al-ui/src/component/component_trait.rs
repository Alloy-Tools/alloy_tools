use crate::{
    ComponentState, ComponentTraitRequirements, ComponentType, CoordType, InputEvent, Rect,
    Renderer, RendererError, SizeConstraints,
};
use std::{future::Future, pin::Pin};

/// Trait for all UI components
/// Components render to a renderer, handle input events, process commands, and can layout children
pub trait ComponentTrait<Coord: CoordType>: ComponentTraitRequirements {
    /// Handle input event
    /// Returns true if event was consumed (stop propagation), false to bubble up
    fn handle_event_with_state(
        &mut self,
        state: &mut ComponentState<Coord>,
        event: InputEvent,
    ) -> bool;

    /// Handle async command (dispatched by window manager)
    fn handle_command_with_state(
        &mut self,
        state: &mut ComponentState<Coord>,
        cmd: al_core::Command,
    ) -> Pin<Box<dyn Future<Output = ()>>>;

    /// Render component to renderer
    fn render_with_state(
        &mut self,
        state: &mut ComponentState<Coord>,
        renderer: &mut dyn Renderer<Coord = Coord>,
        style: crate::Style,
        clip: Rect<Coord>,
    ) -> Result<(), RendererError>;

    /// Compute the preferred size of this component given constraints.
    fn size_with_state(
        &self,
        state: &ComponentState<Coord>,
        constraints: SizeConstraints<Coord>,
    ) -> (Coord, Coord);

    /// Clone the component
    fn clone_component(&self) -> Box<dyn ComponentTrait<Coord>>;

    /// Check if this component is equal to another (for PartialEq)
    fn components_equal(&self, other: &dyn ComponentTrait<Coord>) -> bool;

    /// Get a reference to this component as Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;

    fn as_component(self) -> Box<dyn ComponentType<Coord>>;

    /// Get accessibility label for screen readers
    fn aria_label(&self) -> String {
        String::new()
    }

    /// Convert component to boxed trait object for heterogeneous collections
    fn to_box(self) -> Box<dyn ComponentTrait<Coord>>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

impl<Coord: CoordType> Clone for Box<dyn ComponentTrait<Coord>> {
    fn clone(&self) -> Self {
        self.clone_component()
    }
}

impl<Coord: CoordType> PartialEq for dyn ComponentTrait<Coord> {
    fn eq(&self, other: &Self) -> bool {
        self.components_equal(other)
    }
}

impl<Coord: CoordType> serde::Serialize for dyn ComponentTrait<Coord> {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let _erased = self as &dyn erased_serde::Serialize;
        todo!() // Use a registry after it is split out into `al-structures`
    }
}
impl<'de, Coord: CoordType> serde::Deserialize<'de> for Box<dyn ComponentTrait<Coord>> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!() // Use a registry after it is split out into `al-structures`
    }
}
