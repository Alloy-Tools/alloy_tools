use crate::{ComponentType, CoordType, LayoutType, Rect, SizeConstraints};

/// Grid layout (fixed columns, rows wrap)
#[derive(Default, Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(bound = "Coord: for<'des> serde::Deserialize<'des>")]
pub struct GridLayout<Coord: CoordType> {
    rows: Coord,
    cols: Coord,
    gap_h: Coord,
    gap_v: Coord,
}

impl<Coord: CoordType> GridLayout<Coord> {
    pub fn new(rows: Coord, cols: Coord) -> Self {
        Self {
            rows,
            cols,
            gap_h: Coord::zero(),
            gap_v: Coord::zero(),
        }
    }

    pub fn gap(mut self, h: Coord, v: Coord) -> Self {
        self.gap_h = h;
        self.gap_v = v;
        self
    }
}

impl<Coord: CoordType> LayoutType<Coord> for GridLayout<Coord> {
    fn layout(&mut self, rect: Rect<Coord>, children: &mut [Box<dyn ComponentType<Coord>>]) {
        if children.is_empty() {
            return;
        }

        let total_gap_h = self.gap_h * self.cols.saturating_sub(Coord::one());
        let total_gap_v = self.gap_v * self.rows.saturating_sub(Coord::one());
        let cell_width = (rect.width.saturating_sub(total_gap_h) / self.cols).max(Coord::zero());
        let cell_height = (rect.height.saturating_sub(total_gap_v) / self.rows).max(Coord::zero());

        for (i, child) in children.iter_mut().enumerate() {
            let row = Coord::from_usize(i).unwrap_or_else(Coord::zero) / self.cols;
            let col = Coord::from_usize(i).unwrap_or_else(Coord::zero) % self.cols;

            if row >= self.rows {
                break; // don't place beyond the grid
            }

            let x = rect.x + col * (cell_width + self.gap_h);
            let y = rect.y + row * (cell_height + self.gap_v);
            child.set_rect(Some(Rect::new(x, y, cell_width, cell_height)));
        }
    }

    fn size(
        &self,
        children: &[Box<dyn ComponentType<Coord>>],
        constraints: SizeConstraints<Coord>,
    ) -> (Coord, Coord) {
        if children.is_empty() {
            return (Coord::zero(), Coord::zero());
        }

        let mut max_cell_width = Coord::zero();
        let mut max_cell_height = Coord::zero();

        for child in children {
            let (width, height) = child.size(SizeConstraints::unbounded());
            max_cell_width = max_cell_width.max(width);
            max_cell_height = max_cell_height.max(height);
        }

        let width =
            self.cols * max_cell_width + self.gap_h * (self.cols - Coord::one()).max(Coord::zero());
        let height = self.rows * max_cell_height
            + self.gap_v * (self.rows - Coord::one()).max(Coord::zero());

        constraints.clip(width, height)
    }

    fn clone_layout(&self) -> Box<dyn LayoutType<Coord>> {
        Box::new(*self)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layouts_equal(&self, other: &dyn LayoutType<Coord>) -> bool {
        if let Some(other_layout) = other.as_any().downcast_ref::<GridLayout<Coord>>() {
            self == other_layout
        } else {
            false
        }
    }
}
