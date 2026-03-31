use crate::{
    Component, ComponentStateTrait, ComponentTrait, CoordType, InputEvent, Rect, Renderer,
    SizeConstraints, Style,
};

/// Label component - displays static text
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Label {
    text: String,
    style: Option<Style>,
    wrapping: bool,
    alignment: TextAlignment,
}

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            text,
            style: None,
            wrapping: false,
            alignment: TextAlignment::Left,
        }
    }

    pub fn with_style(mut self, style: Option<Style>) -> Self {
        self.style = style;
        self
    }

    pub fn with_alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn with_wrapping(mut self, wrapping: bool) -> Self {
        self.wrapping = wrapping;
        self
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

impl<Coord: CoordType> ComponentTrait<Coord> for Label {
    fn handle_event_with_state(
        &mut self,
        _: &mut crate::ComponentState<Coord>,
        _: InputEvent,
    ) -> bool {
        false
    }

    fn handle_command_with_state(
        &mut self,
        _: &mut crate::ComponentState<Coord>,
        _: al_core::Command,
    ) -> std::pin::Pin<Box<dyn std::prelude::rust_2024::Future<Output = ()>>> {
        Box::pin(async {})
    }

    fn render_with_state(
        &mut self,
        state: &mut crate::ComponentState<Coord>,
        renderer: &mut dyn Renderer<Coord = Coord>,
        style: crate::Style,
        clip: Rect<Coord>,
    ) -> Result<(), crate::RendererError> {
        if let Some(rect) = state.rect() {
            let style = self.style.unwrap_or(style);
            // Use optimized text rendering - backend handles spacing with RendererMetrics
            renderer.render_text((rect.x, rect.y).into(), &self.text, style, clip)
        } else {
            Ok(())
        }
    }

    fn size_with_state(
        &self,
        _: &crate::ComponentState<Coord>,
        constraints: SizeConstraints<Coord>,
    ) -> (Coord, Coord) {
        constraints.clip(
            Coord::from_usize(self.text.len()).unwrap_or_else(Coord::zero),
            Coord::one(),
        )
    }

    fn clone_component(&self) -> Box<dyn ComponentTrait<Coord>> {
        Box::new(self.clone())
    }

    fn components_equal(&self, other: &dyn ComponentTrait<Coord>) -> bool {
        if let Some(label) = other.as_any().downcast_ref::<Label>() {
            self == label
        } else {
            false
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_component(self) -> Box<dyn crate::ComponentType<Coord>> {
        Box::new(Component::new(self))
    }
}

#[cfg(test)]
mod tests {
    use crate::ComponentState;
    use super::*;

    #[test]
    fn set_text() {
        let mut label = Label::new("Hello 世界 🌍");
        assert_eq!(label.text(), "Hello 世界 🌍");

        label.set_text("Updated");
        assert_eq!(label.text(), "Updated");

        label.set_text("");
        assert_eq!(label.text(), "");
    }

    #[test]
    fn style() {
        let style = crate::Style::default();
        let label = Label::new("").with_style(Some(style));
        assert_eq!(label.style, Some(style));
        assert!(!label.wrapping);
        
        let label = Label::new("").with_wrapping(true);
        assert!(label.wrapping);
    }

    #[test]
    fn size_constraints() {
        let (w, h) = Label::new("Hello").size_with_state(&ComponentState::<u8>::default(), SizeConstraints::unbounded());
        assert_eq!(w, 5); // "Hello" is 5 characters
        assert_eq!(h, 1); // Labels are single-line

        let (w, h) = Label::new("Hello World").size_with_state(&ComponentState::default(), SizeConstraints::width(5u8));
        assert_eq!(w, 5); // Constrained
        assert_eq!(h, 1);

        let (w, h) = Label::new("").size_with_state(&ComponentState::<u8>::default(), SizeConstraints::unbounded());
        assert_eq!(w, 0);
        assert_eq!(h, 1); // Still one tall
    }
}
