use crate::{ComponentType, CoordType, FlexLayout, GridLayout, Rect, SizeConstraints};

pub trait LayoutType<Coord: CoordType>:
    Send + Sync + std::any::Any + std::fmt::Debug + erased_serde::Serialize
{
    /// Given the available rectangle and children, compute rectangles for each child.
    fn layout(&mut self, available: Rect<Coord>, children: &mut [Box<dyn ComponentType<Coord>>]);

    /// Compute the size this layout would occupy given constraints and children.
    fn size(
        &self,
        children: &[Box<dyn ComponentType<Coord>>],
        constraints: SizeConstraints<Coord>,
    ) -> (Coord, Coord);

    /// Get preferred size for this layout
    fn preferred_size(&self, children: &[Box<dyn ComponentType<Coord>>]) -> (Coord, Coord) {
        self.size(children, SizeConstraints::unbounded())
    }

    /// Clone this layout type
    fn clone_layout(&self) -> Box<dyn LayoutType<Coord>>;

    /// Get a reference to this layout as Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;

    /// Check if this layout is equal to another (for PartialEq)
    fn layouts_equal(&self, other: &dyn LayoutType<Coord>) -> bool;
}

impl<Coord: CoordType> Clone for Box<dyn LayoutType<Coord>> {
    fn clone(&self) -> Box<dyn LayoutType<Coord>> {
        self.clone_layout()
    }
}

impl<Coord: CoordType> PartialEq for dyn LayoutType<Coord> {
    fn eq(&self, other: &Self) -> bool {
        self.layouts_equal(other)
    }
}

impl<Coord: CoordType> serde::Serialize for dyn LayoutType<Coord> {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!()
    }
}

impl<'de, Coord: CoordType> serde::Deserialize<'de> for Box<dyn LayoutType<Coord>> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}

#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(bound = "Coord: for<'des> serde::Deserialize<'des>")]
pub enum Layout<Coord: CoordType> {
    /// Default block layout (stacked vertically, full width)
    #[default]
    Block,
    /// Inline layout (claims only needed space)
    Inline,
    /// Flex layout (similar to CSS Flexbox)
    Flex(FlexLayout<Coord>),
    /// Grid layout
    Grid(GridLayout<Coord>),
    /// Custom layout with user-provided implementation
    Custom(Box<dyn LayoutType<Coord>>),
}

fn block_layout<Coord: CoordType>(
    available: Rect<Coord>,
    children: &mut [Box<dyn ComponentType<Coord>>],
) {
    let mut y = available.y;
    let mut total_height = Coord::zero();

    for child in children {
        if total_height < available.height {
            let child_constraints = SizeConstraints {
                width: Some(available.width),
                height: Some(total_height - available.height),
            };
            let (width, height) = child.size(child_constraints);
            total_height += height;
            y += height;
            child.set_rect(Some(Rect::new(available.x, y, width, height)));
        } else {
            child.set_rect(None);
        }
    }
}

/// Returns prefered size of the block layout
fn block_size<Coord: CoordType>(
    children: &[Box<dyn ComponentType<Coord>>],
    constraints: SizeConstraints<Coord>,
) -> (Coord, Coord) {
    if children.is_empty() {
        return (Coord::zero(), Coord::zero());
    }

    let mut total_width = Coord::zero();
    let mut total_height = Coord::zero();
    let max_height = constraints.height.unwrap_or_else(|| Coord::max_value());
    let max_width = constraints.width.unwrap_or_else(|| Coord::max_value());

    for child in children {
        if total_height >= max_height {
            break;
        }
        let (width, height) =
            child.size(SizeConstraints::new(max_width, max_height - total_height));
        total_height += height;
        total_width = total_width.max(width);
    }

    constraints.clip(total_width, total_height)
}

fn inline_layout<Coord: CoordType>(
    available: Rect<Coord>,
    children: &mut [Box<dyn ComponentType<Coord>>],
) {
    let mut x = available.x;
    let mut y = available.y;
    let mut line_height = Coord::zero();

    for child in children {
        let (width, height) = child.size(SizeConstraints::unbounded());

        // Check if needs to wrap
        if x + width > available.x + available.width && x > available.x {
            y += line_height;
            x = available.x;
            line_height = Coord::zero();
        }

        child.set_rect(Some(Rect::new(x, y, width, height)));
        x += width;
        line_height = line_height.max(height);
    }
}

fn inline_size<Coord: CoordType>(
    children: &[Box<dyn ComponentType<Coord>>],
    constraints: SizeConstraints<Coord>,
) -> (Coord, Coord) {
    if children.is_empty() {
        return (Coord::zero(), Coord::zero());
    }

    let line_width = constraints.width.unwrap_or(Coord::max_value());
    let mut x = Coord::zero();
    let mut y = Coord::zero();
    let mut line_height = Coord::zero();
    let mut max_width = Coord::zero();

    for child in children {
        let (child_width, child_height) = child.size(SizeConstraints::unbounded());

        // Check if needs to wrap
        if x + child_width > line_width && x > Coord::zero() {
            // Move to next line
            y += line_height;
            x = Coord::zero();
            line_height = Coord::zero();
        }

        x += child_width;
        line_height = line_height.max(child_height);
        max_width = max_width.max(x);
    }

    y += line_height;
    constraints.clip(max_width, y)
}

impl<Coord: CoordType> LayoutType<Coord> for Layout<Coord> {
    fn layout(&mut self, available: Rect<Coord>, children: &mut [Box<dyn ComponentType<Coord>>]) {
        match self {
            Layout::Block => block_layout(available, children),
            Layout::Inline => inline_layout(available, children),
            Layout::Flex(flex) => flex.layout(available, children),
            Layout::Grid(grid) => grid.layout(available, children),
            Layout::Custom(custom) => custom.layout(available, children),
        }
    }

    fn size(
        &self,
        children: &[Box<dyn ComponentType<Coord>>],
        constraints: SizeConstraints<Coord>,
    ) -> (Coord, Coord) {
        match self {
            Layout::Block => block_size(children, constraints),
            Layout::Inline => inline_size(children, constraints),
            Layout::Flex(flex) => flex.size(children, constraints),
            Layout::Grid(grid) => grid.size(children, constraints),
            Layout::Custom(custom) => custom.size(children, constraints),
        }
    }

    fn clone_layout(&self) -> Box<dyn LayoutType<Coord>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layouts_equal(&self, other: &dyn LayoutType<Coord>) -> bool {
        if let Some(other_layout) = other.as_any().downcast_ref::<Layout<Coord>>() {
            self == other_layout
        } else {
            false
        }
    }
}
