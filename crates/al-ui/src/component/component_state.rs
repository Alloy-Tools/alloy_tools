use crate::{CoordType, Rect};

pub trait ComponentStateTrait<Coord: CoordType> {
    /// Mark rendering as needing redraw
    fn mark_render_dirty(&mut self);

    /// Mark render as clean
    fn mark_render_clean(&mut self);

    /// Check if anything needs updating
    fn is_render_dirty(&self) -> bool;

    /// Get component's rectangular bounds
    fn rect(&self) -> &Option<Rect<Coord>>;

    /// Set component's rectangular bounds (for layout/resize)
    fn set_rect(&mut self, rect: Option<Rect<Coord>>);

    /// Check if point (x, y) is inside this component
    fn contains(&self, pos: crate::Point<Coord, 2>) -> bool;
}

/// This provides a render dirty flag thats set when the component content needs redraw
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(bound = "Coord: for<'des> serde::Deserialize<'des>")]
pub struct ComponentState<Coord: CoordType> {
    rect: Option<Rect<Coord>>,
    /// Rendering is invalid and needs redraw
    #[serde(skip)]
    render_dirty: bool,
}

impl<Coord: CoordType> ComponentState<Coord> {
    /// Create a new component state
    pub fn new(rect: Option<Rect<Coord>>) -> Self {
        Self {
            rect,
            render_dirty: true,
        }
    }
}

impl<Coord: CoordType> Default for ComponentState<Coord> {
    fn default() -> Self {
        Self::new(None)
    }
}

impl<Coord: CoordType> ComponentStateTrait<Coord> for ComponentState<Coord> {
    fn mark_render_dirty(&mut self) {
        self.render_dirty = true;
    }

    fn mark_render_clean(&mut self) {
        self.render_dirty = false;
    }

    fn is_render_dirty(&self) -> bool {
        self.render_dirty
    }

    fn rect(&self) -> &Option<Rect<Coord>> {
        &self.rect
    }

    fn set_rect(&mut self, rect: Option<Rect<Coord>>) {
        self.rect = rect
    }

    fn contains(&self, pos: crate::Point<Coord, 2>) -> bool {
        if let Some(rect) = self.rect {
            rect.contains(pos)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_none() {
        let state = ComponentState::<u16>::new(None);
        assert!(state.rect().is_none());
        assert!(state.is_render_dirty());
    }

    #[test]
    fn new_some() {
        let rect = Rect::new(5, 5, 10, 10);
        let state = ComponentState::new(Some(rect));
        assert_eq!(*state.rect(), Some(rect));
        assert!(state.is_render_dirty());
    }

    #[test]
    fn default() {
        let state = ComponentState::<u16>::default();
        assert!(state.rect().is_none());
        assert!(state.is_render_dirty());
        assert_eq!(state, ComponentState::new(None));
    }

    #[test]
    fn mark_dirty() {
        let mut state = ComponentState::<u16>::default();
        state.mark_render_dirty();
        assert!(state.is_render_dirty());
        state.mark_render_clean();
        assert!(!state.is_render_dirty());
        state.mark_render_dirty();
        assert!(state.is_render_dirty());
    }

    #[test]
    fn set_rect() {
        let mut state = ComponentState::<u16>::default();
        let rect = Rect::new(5, 5, 10, 10);
        state.set_rect(Some(rect));
        assert_eq!(*state.rect(), Some(rect));
        let new_rect = Rect::new(10, 10, 20, 20);
        state.set_rect(Some(new_rect));
        assert_eq!(*state.rect(), Some(new_rect));
        state.set_rect(None);
        assert!(state.rect().is_none());
    }

    #[test]
    fn contains_point() {
        let mut state = ComponentState::new(Some(Rect::new(5, 5, 10, 10)));
        assert!(state.contains((5, 5).into()));
        assert!(state.contains((14, 14).into()));
        assert!(!state.contains((15, 15).into()));
        state.set_rect(None);
        assert!(!state.contains((5, 5).into()));
    }

    #[test]
    fn equality() {
        assert_eq!(
            ComponentState::new(Some(Rect::new(5, 5, 10, 10))),
            ComponentState::new(Some(Rect::new(5, 5, 10, 10)))
        );
        assert_eq!(ComponentState::<u8>::new(None), ComponentState::<u8>::new(None));
        assert_ne!(
            ComponentState::new(Some(Rect::new(5, 5, 10, 10))),
            ComponentState::new(Some(Rect::new(5, 5, 10, 11)))
        );
        assert_ne!(
            ComponentState::new(Some(Rect::new(5, 5, 10, 10))),
            ComponentState::new(None)
        );
    }
}
