/// Window resize handle position
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandle {
    Bottom,
    Right,
    BottomRight,
}

/*TODO: Implement window dragging and resizing logic
    /// Start drag at given offset
    pub fn start_drag(&mut self, offset_x: i16, offset_y: i16) {
        self.drag_state = Some((offset_x, offset_y));
    }

    /// End drag
    pub fn end_drag(&mut self) {
        self.drag_state = None;
    }

    /// Is window being dragged?
    pub fn is_dragging(&self) -> bool {
        self.drag_state.is_some()
    }

    /// Start resize from given handle
    pub fn start_resize(&mut self, handle: ResizeHandle) {
        self.resize_state = Some((handle, self.rect));
    }

    /// End resize
    pub fn end_resize(&mut self) {
        self.resize_state = None;
    }

    /// Is window being resized?
    pub fn is_resizing(&self) -> bool {
        self.resize_state.is_some()
    }

    /// Check if point is in resize handle area (bottom-right corner, typically 2x2)
    pub fn hit_test_resize_handle(&self, x: u16, y: u16, handle_size: u16) -> Option<ResizeHandle> {
        let handle_size = handle_size as u16;
        let right = self.rect.right();
        let bottom = self.rect.bottom();

        if x >= right.saturating_sub(handle_size) && y >= bottom.saturating_sub(handle_size) {
            Some(ResizeHandle::BottomRight)
        } else if x >= right.saturating_sub(handle_size) && y < bottom {
            Some(ResizeHandle::Right)
        } else if x < right && y >= bottom.saturating_sub(handle_size) {
            Some(ResizeHandle::Bottom)
        } else {
            None
        }
    }
*/