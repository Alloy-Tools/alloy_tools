use crate::{
    Component, ComponentStateTrait, ComponentTrait, ComponentType, CoordType, Layout, Pane, Rect,
    RendererError, SizeConstraints,
};
use serde::{Deserialize, Serialize};

pub struct WindowBuilder<Coord: CoordType> {
    layout: Layout<Coord>,
    components: Vec<Box<dyn ComponentType<Coord>>>,
    rect: Rect<Coord>,
    z: Coord,
    title: Option<String>,
    color: Option<crate::Color>,
    border_color: Option<crate::Color>,
    component_color: Option<crate::Color>,
}

impl<Coord: CoordType> Default for WindowBuilder<Coord> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Coord: CoordType> WindowBuilder<Coord> {
    /// Create a new window builder
    pub fn new() -> Self {
        Self {
            layout: Layout::default(),
            components: Vec::new(),
            rect: Rect::default(),
            z: Coord::zero(),
            title: None,
            color: None,
            border_color: None,
            component_color: None,
        }
    }

    /// Set the layout for the window
    pub fn layout(mut self, layout: Layout<Coord>) -> Self {
        self.layout = layout;
        self
    }

    /// Add a component to the window
    pub fn with_component(mut self, component: Box<dyn ComponentType<Coord>>) -> Self {
        self.components.push(component);
        self
    }

    /// Set the rectangle for the window
    pub fn rect(mut self, rect: Rect<Coord>) -> Self {
        self.rect = rect;
        self
    }

    /// Set the z-order for the window
    pub fn z(mut self, z: Coord) -> Self {
        self.z = z;
        self
    }

    /// Set the title for the window
    pub fn title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the color for the window
    pub fn color(mut self, color: impl Into<crate::Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the border color for the window
    pub fn border_color(mut self, color: Option<impl Into<crate::Color>>) -> Self {
        self.border_color = color.map(|c| c.into());
        self
    }

    /// Set the component color for the window
    pub fn component_color(mut self, color: Option<impl Into<crate::Color>>) -> Self {
        self.component_color = color.map(|c| c.into());
        self
    }

    /// Build a window from this builder configuration
    pub fn build(self) -> Window<Coord> {
        Window {
            rect: self.rect,
            z: self.z,
            title: self.title.unwrap_or_default(),
            visible: true,
            color: self.color.unwrap_or(crate::Color::Default),
            border_color: self.border_color,
            component_color: self.component_color,
            pane: Pane::new(self.layout, self.components).as_component(),
        }
    }
}

/// Window wraps a pane with windowing features
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "Coord: for<'des> serde::Deserialize<'des>")]
pub struct Window<Coord: CoordType> {
    /// Window position and size
    pub rect: Rect<Coord>,
    /// Z-order (higher = on top)
    pub z: Coord,
    /// Window title
    pub title: String,
    /// Is window visible?
    pub visible: bool,
    /// The windows pane
    pub pane: Box<dyn ComponentType<Coord>>,
    /// Window background color
    pub color: crate::Color,
    /// Border color (None = no border)
    pub border_color: Option<crate::Color>,
    /// Optional color for component reference
    pub component_color: Option<crate::Color>,
}

impl<Coord: CoordType> PartialEq for Window<Coord> {
    fn eq(&self, other: &Self) -> bool {
        self.rect == other.rect
            && self.z == other.z
            && self.title == other.title
            && self.visible == other.visible
            && self.color == other.color
            && self.border_color == other.border_color
            && self.component_color == other.component_color
            && self.pane.partial_eq(&other.pane)
    }
}

impl<Coord: CoordType> Window<Coord> {
    /// Create new window
    pub fn new() -> WindowBuilder<Coord> {
        WindowBuilder::new()
    }

    /// Set z-order
    pub fn set_z(&mut self, z: Coord) {
        self.z = z;
    }

    /// Set window title
    pub fn set_title(&mut self, title: String) {
        self.title = title.clone();
    }

    /// Set window size
    pub fn set_size(&mut self, width: Coord, height: Coord) {
        self.rect.width = width;
        self.rect.height = height;
    }

    /// Set window position
    pub fn set_pos(&mut self, x: Coord, y: Coord) {
        self.rect.x = x;
        self.rect.y = y;
    }

    /// Set visibility
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Check if window is visible
    pub fn visible(&self) -> bool {
        self.visible
    }

    /// Set window color
    pub fn set_color(&mut self, color: impl Into<crate::Color>) {
        self.color = color.into();
    }

    /// Set border color
    pub fn set_border_color(&mut self, color: Option<impl Into<crate::Color>>) {
        self.border_color = color.map(|c| c.into());
    }

    /// Set component color
    pub fn set_component_color(&mut self, color: Option<impl Into<crate::Color>>) {
        self.component_color = color.map(|c| c.into());
    }
}

impl<Coord: CoordType> ComponentTrait<Coord> for Window<Coord> {
    fn handle_event_with_state(
        &mut self,
        state: &mut crate::ComponentState<Coord>,
        event: crate::InputEvent,
    ) -> bool {
        if self.pane.handle_event(event) {
            state.mark_render_dirty();
            return true;
        }
        false
    }

    fn handle_command_with_state(
        &mut self,
        _: &mut crate::ComponentState<Coord>,
        cmd: al_core::Command,
    ) -> std::pin::Pin<Box<dyn std::prelude::rust_2024::Future<Output = ()>>> {
        self.pane.handle_command(cmd)
    }

    fn render_with_state(
        &mut self,
        state: &mut crate::ComponentState<Coord>,
        renderer: &mut dyn crate::Renderer<Coord = Coord>,
        style: crate::Style,
        mut clip: Rect<Coord>,
    ) -> Result<(), RendererError> {
        if !self.visible || !state.is_render_dirty() {
            return Ok(());
        }
        let two = Coord::from_u8(2).unwrap_or_else(Coord::zero);
        //let style = crate::Style::new()
        //    .fg(self.component_color.unwrap_or(crate::Color::Default))
        //    .bg(self.color);
        // Render background and possibly borders, removing from clip if drawn
        let mut rect_clip = self.rect;
        renderer.draw_rect(self.rect, clip, self.color, self.border_color)?;
        if self.border_color.is_some() {
            rect_clip = Rect::new(
                rect_clip.x + Coord::one(),
                rect_clip.y + Coord::one(),
                rect_clip.width - two,
                rect_clip.height - two,
            );
            clip = clip.intersection(rect_clip);
        }
        if clip.width > Coord::zero() && clip.height > Coord::zero() {
            // Render title and title bar, removing them from clip
            if !self.title.is_empty() {
                // Render title
                renderer.draw_text((rect_clip.x, rect_clip.y).into(), &self.title, style, clip)?;
                // Render title bar
                renderer.draw_rect(
                    Rect {
                        x: rect_clip.x,
                        y: rect_clip.y + Coord::one(),
                        width: rect_clip.width,
                        height: Coord::one(),
                    },
                    clip,
                    self.color,
                    self.border_color.or(Some(crate::Color::Default)),
                )?;
                rect_clip = Rect::new(
                    rect_clip.x,
                    rect_clip.y + two,
                    rect_clip.width,
                    rect_clip.height - two,
                );
                clip = clip.intersection(rect_clip);
            }
            self.pane.render(renderer, style, clip)?;
        }
        Ok(())
    }

    fn size_with_state(
        &self,
        _: &crate::ComponentState<Coord>,
        constraints: SizeConstraints<Coord>,
    ) -> (Coord, Coord) {
        constraints.clip(self.rect.width, self.rect.height)
    }

    fn clone_component(&self) -> Box<dyn ComponentTrait<Coord>> {
        Box::new(self.clone())
    }

    fn components_equal(&self, other: &dyn ComponentTrait<Coord>) -> bool {
        if let Some(window) = other.as_any().downcast_ref::<Window<Coord>>() {
            self == window
        } else {
            false
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_component(self) -> Box<dyn ComponentType<Coord>> {
        Box::new(Component::new(self))
    }
    /*fn serialize_state(
        &self,
        serializer: &mut dyn erased_serde::Serializer,
    ) -> Result<(), SerdeError> {
        use erased_serde::Serialize as _;
        self.erased_serialize(serializer)
            .map_err(|e| SerdeError::from(e))?;
        self.components
            .len()
            .erased_serialize(serializer)
            .map_err(|e| SerdeError::from(e))?;
        for component in self.components {
            component.serialize_state(serializer)?;
        }
        self.layout.serialize_state(serializer)?;
        Ok(())
    }

    fn deserialize_state(
        &mut self,
        deserializer: &mut dyn erased_serde::Deserializer,
    ) -> Result<(), SerdeError> {
        self.from_state(erased_serde::deserialize(deserializer).map_err(|e| SerdeError::from(e))?);
        let len: usize =
            erased_serde::deserialize(deserializer).map_err(|e| SerdeError::from(e))?;
        let components = Vec::with_capacity(len);
        for _ in 0..len {
            //TODO: deserialize using a registry (like event), then remove the skip for this
            let component: Box<dyn Component> =
                erased_serde::deserialize(deserializer).map_err(|e| SerdeError::from(e))?;
            components.push(component);
        }
        self.components
        self.layout.deserialize_state(deserializer)?;
        Ok(())
    }*/
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let builder = WindowBuilder::<u8>::new();
        assert_eq!(builder.rect, Rect::default());
        assert_eq!(builder.z, 0);
        assert!(builder.title.is_none());
        assert!(builder.color.is_none());

        let rect = Rect::new(5, 5, 100, 50);
        let builder = WindowBuilder::new()
            .layout(Layout::Block)
            .rect(rect)
            .z(10)
            .title("Test Window".to_string())
            .color(crate::Color::Default)
            .border_color(Some(crate::Color::Default));

        assert_eq!(builder.layout, Layout::Block);
        assert_eq!(builder.rect, rect);
        assert_eq!(builder.z, 10);
        assert_eq!(builder.title, Some("Test Window".to_string()));
        assert!(builder.color.is_some());
        assert!(builder.border_color.is_some());

        let mut window = builder.build();
        assert_eq!(window.title, "Test Window");
        assert_eq!(window.rect, Rect::new(5, 5, 100, 50));
        assert_eq!(window.z, 10);
        assert!(window.visible);
        window.set_visible(false);
        assert!(!window.visible);
    }
}
