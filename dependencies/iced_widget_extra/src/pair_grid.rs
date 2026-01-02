//! Display a grid of two columns.
use iced_core;
use iced_core::Clipboard;
use iced_core::Padding;
use iced_core::alignment::{Horizontal, Vertical};
use iced_core::layout;
use iced_core::mouse;
use iced_core::overlay;
use iced_core::renderer;
use iced_core::widget;
use iced_core::{
    Alignment, Background, Element, Event, Layout, Length, Pixels, Rectangle, Shell, Size, Vector,
    Widget,
};
use iced_renderer;
use iced_widget;

/// Creates a new [`PairGrid`] with the given pairs.
///
/// Pairs can be created using the [`pair()`] function.
pub fn pair_grid<'a, Message, Theme, Renderer>(
    pairs: impl IntoIterator<Item = Pair<'a, Message, Theme, Renderer>>,
) -> PairGrid<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: iced_core::Renderer,
{
    PairGrid::new(pairs)
}

/// Creates a new [`Pair`] with the given left and right elements.
pub fn pair<'a, Message, Theme, Renderer>(
    left: impl Into<Element<'a, Message, Theme, Renderer>>,
    right: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Pair<'a, Message, Theme, Renderer> {
    Pair {
        left: left.into(),
        right: right.into(),
        left_align_x: Horizontal::Left,
        left_align_y: Vertical::Top,
        right_align_x: Horizontal::Left,
        right_align_y: Vertical::Top,
    }
}

/// A two columns grid visual representation of pair of element (e.g. property table, horizontal form inputs).
pub struct PairGrid<'a, Message, Theme = iced_widget::Theme, Renderer = iced_renderer::Renderer>
where
    Theme: Catalog,
{
    pairs: Vec<Pair_>,
    cells: Vec<Element<'a, Message, Theme, Renderer>>,
    width: Length,
    height: Length,
    left_width: Length,
    right_width: Length,
    padding: Padding,
    spacing_x: f32,
    spacing_y: f32,
    separator_x: f32,
    separator_y: f32,
    class: Theme::Class<'a>,
}

struct Pair_ {
    left_align_x: Horizontal,
    left_align_y: Vertical,
    right_align_x: Horizontal,
    right_align_y: Vertical,
}

impl<'a, Message, Theme, Renderer> PairGrid<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: iced_core::Renderer,
{
    /// Creates a new [`PairGrid`] with the given pairs.
    ///
    /// Pairs can be created using the [`pair()`] function.
    pub fn new(pairs: impl IntoIterator<Item = Pair<'a, Message, Theme, Renderer>>) -> Self {
        let pairs = pairs.into_iter();

        let mut width = Length::Shrink;
        let mut height = Length::Shrink;

        let mut cells = Vec::with_capacity(pairs.size_hint().0 * 2);

        let pairs: Vec<_> = pairs
            .map(|pair| {
                width = width.enclose(pair.left.as_widget().size_hint().height);
                height = height.enclose(pair.right.as_widget().size_hint().height);

                cells.push(pair.left);
                cells.push(pair.right);

                Pair_ {
                    left_align_x: pair.left_align_x,
                    left_align_y: pair.left_align_y,
                    right_align_x: pair.right_align_x,
                    right_align_y: pair.right_align_y,
                }
            })
            .collect();

        Self {
            pairs,
            cells,
            width,
            height,
            left_width: Length::Shrink,
            right_width: Length::Shrink,
            padding: Padding::ZERO,
            spacing_x: 0.0,
            spacing_y: 0.0,
            separator_x: 0.0,
            separator_y: 0.0,
            class: Theme::default(),
        }
    }

    /// Sets the width of the [`PairGrid`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the width of the left column.
    pub fn left_width(mut self, width: impl Into<Length>) -> Self {
        self.left_width = width.into();
        self
    }

    /// Sets the width of the right column.
    pub fn right_width(mut self, width: impl Into<Length>) -> Self {
        self.right_width = width.into();
        self
    }

    /// Sets the padding of the [`PairGrid`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the spacing between columns and rows of the [`PairGrid`].
    pub fn spacing(self, spacing: impl Into<Pixels>) -> Self {
        let spacing = spacing.into();
        self.spacing_x(spacing).spacing_y(spacing)
    }

    /// Sets the horizontal spacing between columns of the [`PairGrid`].
    pub fn spacing_x(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing_x = spacing.into().0;
        self
    }

    /// Sets the vertical spacing between rows of the [`PairGrid`].
    pub fn spacing_y(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing_y = spacing.into().0;
        self
    }

    /// Sets the thickness of the line separator between the cells of the [`PairGrid`].
    pub fn separator(self, separator: impl Into<Pixels>) -> Self {
        let separator = separator.into();

        self.separator_x(separator).separator_y(separator)
    }

    /// Sets the thickness of the horizontal line separator between the cells of the [`PairGrid`].
    pub fn separator_x(mut self, separator: impl Into<Pixels>) -> Self {
        self.separator_x = separator.into().0;
        self
    }

    /// Sets the thickness of the vertical line separator between the cells of the [`PairGrid`].
    pub fn separator_y(mut self, separator: impl Into<Pixels>) -> Self {
        self.separator_y = separator.into().0;
        self
    }
}

struct Metrics {
    columns: Vec<f32>,
    rows: Vec<f32>,
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for PairGrid<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: iced_core::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<Metrics>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(Metrics {
            columns: Vec::new(),
            rows: Vec::new(),
        })
    }

    fn children(&self) -> Vec<widget::Tree> {
        self.cells
            .iter()
            .map(|cell| widget::Tree::new(cell.as_widget()))
            .collect()
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&self.cells);
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let metrics = tree.state.downcast_mut::<Metrics>();
        let rows = self.pairs.len();

        let limits = limits.width(self.width).height(self.height);
        let available = limits.max();

        let mut cells = Vec::with_capacity(self.cells.len());
        cells.resize(self.cells.len(), layout::Node::default());

        metrics.columns = vec![0.0; 2];
        metrics.rows = vec![0.0; rows];

        let mut column_factors = [0; 2];
        let mut total_row_factors = 0;
        let mut total_fluid_height = 0.0;
        let mut row_factor = 0;

        // FIRST PASS
        // Lay out non-fluid cells
        let mut x = self.padding.left;
        let mut y = self.padding.top;

        for (i, (cell, state)) in self.cells.iter_mut().zip(&mut tree.children).enumerate() {
            let row = i / 2;
            let column = i % 2;

            let width = if column == 0 {
                self.left_width
            } else {
                self.right_width
            };

            let size = cell.as_widget().size();

            if column == 0 {
                x = self.padding.left;

                if row > 0 {
                    y += metrics.rows[row - 1] + self.spacing_y;

                    if row_factor != 0 {
                        total_fluid_height += metrics.rows[row - 1];
                        total_row_factors += row_factor;
                        row_factor = 0;
                    }
                }
            }

            let width_factor = width.fill_factor();
            let height_factor = size.height.fill_factor();

            if width_factor != 0 || height_factor != 0 || size.width.is_fill() {
                column_factors[column] = column_factors[column].max(width_factor);
                row_factor = row_factor.max(height_factor);
                continue;
            }

            let limits = layout::Limits::new(
                Size::ZERO,
                Size::new(available.width - x, available.height - y),
            )
            .width(width);

            let layout = cell.as_widget_mut().layout(state, renderer, &limits);
            let size = limits.resolve(width, Length::Shrink, layout.size());

            metrics.columns[column] = metrics.columns[column].max(size.width);
            metrics.rows[row] = metrics.rows[row].max(size.height);
            cells[i] = layout;

            x += size.width + self.spacing_x;
        }

        // SECOND PASS
        // Lay out fluid cells, using metrics from the first pass as limits
        let left = Size::new(
            available.width
                - metrics
                    .columns
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| column_factors[*i] == 0)
                    .map(|(_, width)| width)
                    .sum::<f32>(),
            available.height - total_fluid_height,
        );

        let width_unit = (left.width
            - self.spacing_x * (2_usize).saturating_sub(1) as f32
            - self.padding.left
            - self.padding.right)
            / column_factors.iter().sum::<u16>() as f32;

        let height_unit = (left.height
            - self.spacing_y * rows.saturating_sub(1) as f32
            - self.padding.top
            - self.padding.bottom)
            / total_row_factors as f32;

        let mut y = self.padding.top;

        for (i, (cell, state)) in self.cells.iter_mut().zip(&mut tree.children).enumerate() {
            let row = i / 2;
            let column = i % 2;

            let size = cell.as_widget().size();

            let width = if column == 0 {
                self.left_width
            } else {
                self.right_width
            };

            let width_factor = width.fill_factor();
            let height_factor = size.height.fill_factor();

            if column == 0 {
                x = self.padding.left;
                if row > 0 {
                    y += metrics.rows[row - 1] + self.spacing_y;
                }
            }

            if width_factor == 0 && size.width.fill_factor() == 0 && size.height.fill_factor() == 0
            {
                continue;
            }

            let max_width = if width_factor > 0 {
                width_unit * width_factor as f32
            } else {
                metrics.columns[column].max(0.0)
            };

            let max_height = if height_factor == 0 {
                if size.height.is_fill() {
                    metrics.rows[row]
                } else {
                    (available.height - y).max(0.0)
                }
            } else {
                height_unit * height_factor as f32
            };

            let limits = layout::Limits::new(Size::ZERO, Size::new(max_width, max_height));

            let layout = cell.as_widget_mut().layout(state, renderer, &limits);
            let size = limits.resolve(width, Length::Shrink, layout.size());

            metrics.columns[column] = metrics.columns[column].max(size.width);
            metrics.rows[row] = metrics.rows[row].max(size.height);
            cells[i] = layout;

            x += size.width + self.spacing_x;
        }

        // THIRD PASS
        // Position each cell
        let mut x = self.padding.left;
        let mut y = self.padding.top;

        for (i, cell) in cells.iter_mut().enumerate() {
            let row = i / 2;
            let column = i % 2;

            if column == 0 {
                x = self.padding.left;
                if row > 0 {
                    y += metrics.rows[row - 1] + self.spacing_y;
                }
            }

            let Pair_ {
                left_align_x,
                left_align_y,
                right_align_x,
                right_align_y,
                ..
            } = &self.pairs[row];

            let (align_x, align_y) = if column == 0 {
                (left_align_x, left_align_y)
            } else {
                (right_align_x, right_align_y)
            };

            cell.move_to_mut((x, y));
            cell.align_mut(
                Alignment::from(*align_x),
                Alignment::from(*align_y),
                Size::new(metrics.columns[column], metrics.rows[row]),
            );

            x += metrics.columns[column] + self.spacing_x;
        }

        let intrinsic = limits.resolve(
            self.width,
            self.height,
            Size::new(
                x - self.spacing_x + self.padding.right,
                y + metrics
                    .rows
                    .last()
                    .copied()
                    .map(|height| height + self.padding.bottom)
                    .unwrap_or_default(),
            ),
        );

        layout::Node::with_children(intrinsic, cells)
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        for ((cell, tree), layout) in self
            .cells
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            cell.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        for ((cell, state), layout) in self.cells.iter().zip(&tree.children).zip(layout.children())
        {
            cell.as_widget()
                .draw(state, renderer, theme, style, layout, cursor, viewport);
        }

        let bounds = layout.bounds();
        let metrics = tree.state.downcast_ref::<Metrics>();
        let style = theme.style(&self.class);

        if self.separator_x > 0.0 {
            let mut x = self.padding.left;
            for width in &metrics.columns[..metrics.columns.len().saturating_sub(1)] {
                x += width + (self.spacing_x - self.separator_x) / 2.0;

                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: bounds.x + x,
                            y: bounds.y,
                            width: self.separator_x,
                            height: bounds.height,
                        },
                        ..renderer::Quad::default()
                    },
                    style.separator_x,
                );

                x += self.separator_x + (self.spacing_x - self.separator_x) / 2.0;
            }
        }

        if self.separator_y > 0.0 {
            let mut y = self.padding.top;
            for height in &metrics.rows[..metrics.rows.len().saturating_sub(1)] {
                y += height + (self.spacing_y - self.separator_y) / 2.0;

                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: bounds.x,
                            y: bounds.y + y,
                            width: bounds.width,
                            height: self.separator_y,
                        },
                        ..renderer::Quad::default()
                    },
                    style.separator_y,
                );

                y += self.separator_y + (self.spacing_y - self.separator_y) / 2.0;
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.cells
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((cell, tree), layout)| {
                cell.as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        for ((cell, state), layout) in self
            .cells
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            cell.as_widget_mut()
                .operate(state, layout, renderer, operation);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(
            &mut self.cells,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<PairGrid<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: Catalog + 'a,
    Renderer: iced_core::Renderer + 'a,
{
    fn from(table: PairGrid<'a, Message, Theme, Renderer>) -> Self {
        Element::new(table)
    }
}

/// A horizontal visualization of some data in two columns.
pub struct Pair<'a, Message, Theme = iced_widget::Theme, Renderer = iced_renderer::Renderer> {
    left: Element<'a, Message, Theme, Renderer>,
    right: Element<'a, Message, Theme, Renderer>,
    left_align_x: Horizontal,
    left_align_y: Vertical,
    right_align_x: Horizontal,
    right_align_y: Vertical,
}

impl<'a, Message, Theme, Renderer> Pair<'a, Message, Theme, Renderer> {
    /// Sets the alignment for the horizontal axis of the left.
    pub fn left_align_x(mut self, alignment: impl Into<Horizontal>) -> Self {
        self.left_align_x = alignment.into();
        self
    }

    /// Sets the alignment for the vertical axis of the left.
    pub fn left_align_y(mut self, alignment: impl Into<Vertical>) -> Self {
        self.left_align_y = alignment.into();
        self
    }

    /// Sets the alignment for the horizontal axis of the right.
    pub fn right_align_x(mut self, alignment: impl Into<Horizontal>) -> Self {
        self.right_align_x = alignment.into();
        self
    }

    /// Sets the alignment for the vertical axis of the right.
    pub fn right_align_y(mut self, alignment: impl Into<Vertical>) -> Self {
        self.right_align_y = alignment.into();
        self
    }
}

/// The appearance of a [`PairGrid`].
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// The background color of the horizontal line separator between cells.
    pub separator_x: Background,
    /// The background color of the vertical line separator between cells.
    pub separator_y: Background,
}

/// The theme catalog of a [`PairGrid`].
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>) -> Style;
}

/// A styling function for a [`PairGrid`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> Style + 'a>;

impl<Theme> From<Style> for StyleFn<'_, Theme> {
    fn from(style: Style) -> Self {
        Box::new(move |_theme| style)
    }
}

impl Catalog for iced_widget::Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

/// The default style of a [`PairGrid`].
pub fn default(theme: &iced_widget::Theme) -> Style {
    let palette = theme.extended_palette();
    let separator = palette.background.strong.color.into();

    Style {
        separator_x: separator,
        separator_y: separator,
    }
}
