use crate::{ComponentType, CoordType, LayoutType, Rect, SizeConstraints};

/// Flex layout direction
#[derive(
    Default, Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum Direction {
    #[default]
    Row,
    Column,
}

/// Flex item flex properties
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(bound = "Coord: for<'des> serde::Deserialize<'des>")]
pub struct FlexItem<Coord: CoordType> {
    /// Flex grow factor (0 means no growth, > 0 means it can grow to fill available space)
    pub flex_grow: f32,
    /// Optional minimum size for this item
    pub min_size: Option<(Coord, Coord)>,
}

impl<Coord: CoordType> FlexItem<Coord> {
    pub fn new() -> Self {
        Self {
            flex_grow: 0.0,
            min_size: None,
        }
    }

    pub fn grow(mut self, factor: f32) -> Self {
        self.flex_grow = factor;
        self
    }

    pub fn min_size(mut self, w: Coord, h: Coord) -> Self {
        self.min_size = Some((w, h));
        self
    }
}

/// Flex layout (similar to CSS Flexbox)
#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(bound = "Coord: for<'des> serde::Deserialize<'des>")]
pub struct FlexLayout<Coord: CoordType> {
    direction: Direction,
    items: Vec<FlexItem<Coord>>,
    gap: Coord,
}

impl<Coord: CoordType> FlexLayout<Coord> {
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            items: Vec::new(),
            gap: Coord::zero(),
        }
    }

    pub fn add_item(mut self, item: FlexItem<Coord>) -> Self {
        self.items.push(item);
        self
    }

    pub fn gap(mut self, gap: Coord) -> Self {
        self.gap = gap;
        self
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn items(&self) -> &[FlexItem<Coord>] {
        &self.items
    }
}

impl<Coord: CoordType> LayoutType<Coord> for FlexLayout<Coord> {
    fn layout(&mut self, available: Rect<Coord>, children: &mut [Box<dyn ComponentType<Coord>>]) {
        if children.is_empty() {
            return;
        }

        match self.direction {
            Direction::Row => {
                // Distribute width among children
                let total_gap = Coord::from_usize(children.len().saturating_sub(1))
                    .unwrap_or_else(Coord::zero)
                    * self.gap;
                let available_width = available.width.saturating_sub(total_gap).max(Coord::zero());

                let mut x = available.x;
                let total_flex_grow: f32 = self.items.iter().map(|i| i.flex_grow.max(0.0)).sum();

                let children_len = children.len();
                for (i, child) in children.iter_mut().enumerate() {
                    let item = self.items.get(i).copied().unwrap_or_else(FlexItem::new);
                    let width = if total_flex_grow > 0.0 {
                        let proportion = item.flex_grow.max(0.0) / total_flex_grow;
                        Coord::from_f32(available_width.to_f32().unwrap_or(0.0) * proportion)
                            .unwrap_or_else(Coord::zero)
                    } else {
                        Coord::from_f32(
                            available_width.to_f32().unwrap_or(0.0) / children_len as f32,
                        )
                        .unwrap_or_else(Coord::zero)
                    };

                    let constraints = SizeConstraints {
                        width: Some(width),
                        height: Some(available.height),
                    };
                    let (_, actual_height) = child.size(constraints);

                    child.set_rect(Some(Rect::new(x, available.y, width, actual_height)));
                    x = x + width + self.gap;
                }
            }
            Direction::Column => {
                // Distribute height among children
                let total_gap = Coord::from_usize(children.len().saturating_sub(1))
                    .unwrap_or_else(Coord::zero)
                    * self.gap;
                let available_height = available
                    .height
                    .saturating_sub(total_gap)
                    .max(Coord::zero());

                let mut y = available.y;
                let total_flex_grow: f32 = self.items.iter().map(|i| i.flex_grow.max(0.0)).sum();

                let children_len = children.len();
                for (i, child) in children.iter_mut().enumerate() {
                    let item = self.items.get(i).copied().unwrap_or_else(FlexItem::new);
                    let height = if total_flex_grow > 0.0 {
                        let proportion = item.flex_grow.max(0.0) / total_flex_grow;
                        Coord::from_f32(available_height.to_f32().unwrap_or(0.0) * proportion)
                            .unwrap_or_else(Coord::zero)
                    } else {
                        Coord::from_f32(
                            available_height.to_f32().unwrap_or(0.0) / children_len as f32,
                        )
                        .unwrap_or_else(Coord::zero)
                    };

                    let constraints = SizeConstraints {
                        width: Some(available.width),
                        height: Some(height),
                    };
                    let (actual_width, _) = child.size(constraints);

                    child.set_rect(Some(Rect::new(available.x, y, actual_width, height)));
                    y = y + height + self.gap;
                }
            }
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

        match self.direction {
            Direction::Row => {
                // Sum widths, max height
                let mut total_width = Coord::zero();
                let mut max_height = Coord::zero();
                let gaps = Coord::from_usize(children.len().saturating_sub(1))
                    .unwrap_or_else(Coord::zero)
                    * self.gap;

                for child in children {
                    let (width, height) = child.size(SizeConstraints::unbounded());
                    total_width += width;
                    max_height = max_height.max(height);
                }

                total_width += gaps;
                constraints.clip(total_width, max_height)
            }
            Direction::Column => {
                // Sum heights, max width
                let mut total_height = Coord::zero();
                let mut max_width = Coord::zero();
                let gaps = Coord::from_usize(children.len().saturating_sub(1))
                    .unwrap_or_else(Coord::zero)
                    * self.gap;

                for child in children {
                    let (width, height) = child.size(SizeConstraints::unbounded());
                    total_height += height;
                    max_width = max_width.max(width);
                }

                total_height += gaps;
                constraints.clip(max_width, total_height)
            }
        }
    }

    fn clone_layout(&self) -> Box<dyn LayoutType<Coord>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layouts_equal(&self, other: &dyn LayoutType<Coord>) -> bool {
        if let Some(other_layout) = other.as_any().downcast_ref::<FlexLayout<Coord>>() {
            self == other_layout
        } else {
            false
        }
    }
}
