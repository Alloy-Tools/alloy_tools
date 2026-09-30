use crate::{
    ComponentRequirements, ComponentState, ComponentStateTrait, ComponentTrait, CoordType,
};

/// `ComponentType` is used to allow any `Component<C, Coord>` type to be passed as a `dyn ComponentType`
pub trait ComponentType<Coord: CoordType>:
    ComponentTrait<Coord> + ComponentStateTrait<Coord>
{
    fn handle_event(&mut self, event: crate::InputEvent) -> bool;

    fn handle_command(
        &mut self,
        cmd: al_core::Command,
    ) -> std::pin::Pin<Box<dyn std::prelude::rust_2024::Future<Output = ()>>>;

    fn render(
        &mut self,
        renderer: &mut dyn crate::Renderer<Coord = Coord>,
        style: crate::Style,
        clip: crate::Rect<Coord>,
    ) -> Result<(), crate::RendererError>;

    fn size(&self, constraints: crate::SizeConstraints<Coord>) -> (Coord, Coord);

    fn clone_component_type(&self) -> Box<dyn ComponentType<Coord>>;

    fn partial_eq(&self, other: &Box<dyn ComponentType<Coord>>) -> bool;
}
impl<Coord: CoordType, C: ComponentRequirements + ComponentTrait<Coord>> ComponentType<Coord>
    for Component<C, Coord>
{
    fn handle_event(&mut self, event: crate::InputEvent) -> bool {
        self.0.handle_event_with_state(&mut self.1, event)
    }

    fn handle_command(
        &mut self,
        cmd: al_core::Command,
    ) -> std::pin::Pin<Box<dyn std::prelude::rust_2024::Future<Output = ()>>> {
        self.0.handle_command_with_state(&mut self.1, cmd)
    }

    fn render(
        &mut self,
        renderer: &mut dyn crate::Renderer<Coord = Coord>,
        style: crate::Style,
        clip: crate::Rect<Coord>,
    ) -> Result<(), crate::RendererError> {
        self.0.render_with_state(&mut self.1, renderer, style, clip)
    }

    fn size(&self, constraints: crate::SizeConstraints<Coord>) -> (Coord, Coord) {
        self.0.size_with_state(&self.1, constraints)
    }

    fn clone_component_type(&self) -> Box<dyn ComponentType<Coord>> {
        Box::new(self.clone())
    }

    fn partial_eq(&self, other: &Box<dyn ComponentType<Coord>>) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Component<C, Coord>>() {
            self == other
        } else {
            false
        }
    }
}

impl<Coord: CoordType> Clone for Box<dyn ComponentType<Coord>> {
    fn clone(&self) -> Self {
        self.clone_component_type()
    }
}

impl<Coord: CoordType> PartialEq for Box<dyn ComponentType<Coord>> {
    fn eq(&self, other: &Self) -> bool {
        self.partial_eq(other)
    }
}

impl<Coord: CoordType> serde::Serialize for Box<dyn ComponentType<Coord>> {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!() // Use a registry after it is split out into `al-structures`
    }
}

impl<'de, Coord: CoordType> serde::Deserialize<'de> for Box<dyn ComponentType<Coord>> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!() // Use a registry after it is split out into `al-structures`
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(bound = "Coord: for<'des> serde::Deserialize<'des>")]
pub struct Component<C: ComponentRequirements + ComponentTrait<Coord>, Coord: CoordType>(
    C,
    ComponentState<Coord>,
);

impl<C: ComponentRequirements + ComponentTrait<Coord>, Coord: CoordType> Component<C, Coord> {
    pub fn new(inner: C) -> Self {
        Self(inner, ComponentState::default())
    }

    pub fn as_dyn_component(&self) -> &dyn ComponentType<Coord> {
        self
    }

    pub fn to_box(self) -> Box<dyn ComponentType<Coord>> {
        Box::new(self) as Box<dyn ComponentType<Coord>>
    }
}

impl<C: ComponentRequirements + ComponentTrait<Coord>, Coord: CoordType> ComponentTrait<Coord>
    for Component<C, Coord>
{
    fn handle_event_with_state(
        &mut self,
        state: &mut ComponentState<Coord>,
        event: crate::InputEvent,
    ) -> bool {
        self.0.handle_event_with_state(state, event)
    }

    fn handle_command_with_state(
        &mut self,
        state: &mut ComponentState<Coord>,
        cmd: al_core::Command,
    ) -> std::pin::Pin<Box<dyn std::prelude::rust_2024::Future<Output = ()>>> {
        self.0.handle_command_with_state(state, cmd)
    }

    fn render_with_state(
        &mut self,
        state: &mut ComponentState<Coord>,
        renderer: &mut dyn crate::Renderer<Coord = Coord>,
        style: crate::Style,
        clip: crate::Rect<Coord>,
    ) -> Result<(), crate::RendererError> {
        self.0.render_with_state(state, renderer, style, clip)
    }

    fn size_with_state(
        &self,
        state: &ComponentState<Coord>,
        constraints: crate::SizeConstraints<Coord>,
    ) -> (Coord, Coord) {
        self.0.size_with_state(state, constraints)
    }

    fn clone_component(&self) -> Box<dyn ComponentTrait<Coord>> {
        Box::new(self.clone())
    }

    fn components_equal(&self, other: &dyn ComponentTrait<Coord>) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Component<C, Coord>>() {
            self == other
        } else {
            false
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_component(self) -> Box<dyn ComponentType<Coord>> {
        Box::new(self)
    }
}

impl<Coord: CoordType, C: ComponentRequirements + ComponentTrait<Coord>> ComponentStateTrait<Coord>
    for Component<C, Coord>
{
    fn mark_render_dirty(&mut self) {
        self.1.mark_render_dirty();
    }

    fn mark_render_clean(&mut self) {
        self.1.mark_render_clean();
    }

    fn is_render_dirty(&self) -> bool {
        self.1.is_render_dirty()
    }

    fn rect(&self) -> &Option<crate::Rect<Coord>> {
        self.1.rect()
    }

    fn set_rect(&mut self, rect: Option<crate::Rect<Coord>>) {
        self.1.set_rect(rect);
    }

    fn contains(&self, pos: crate::Point<Coord, 2>) -> bool {
        self.1.contains(pos)
    }
}
