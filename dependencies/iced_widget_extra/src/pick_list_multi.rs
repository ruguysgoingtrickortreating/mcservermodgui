//! Pick lists display a dropdown list of selectable options.
//!
//! CUSTOM: Supports multi selection with 3 states and custom selection icons.
use crate::overlay::menu_multi::{self, Menu};

use iced_core::alignment;
use iced_core::keyboard;
use iced_core::layout;
use iced_core::mouse;
use iced_core::overlay;
use iced_core::renderer;
use iced_core::text::paragraph;
use iced_core::text::{self, Text};
use iced_core::touch;
use iced_core::widget::tree::{self, Tree};
use iced_core::window;
use iced_core::{
    Background, Border, Clipboard, Color, Element, Event, Layout, Length, Padding, Pixels, Point,
    Rectangle, Shell, Size, Theme, Vector, Widget,
};

use std::borrow::Borrow;
use std::f32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionState {
    Unselected,
    Included,
    Excluded,
}

impl From<SelectionState> for i64 {
    fn from(value: SelectionState) -> Self {
        match value {
            SelectionState::Excluded => 0,
            SelectionState::Included => 1,
            SelectionState::Unselected => 2,
        }
    }
}

impl From<i64> for SelectionState {
    fn from(value: i64) -> Self {
        match value {
            0 => Self::Excluded,
            1 => Self::Included,
            2 => Self::Unselected,
            _ => Self::Unselected,
        }
    }
}

pub struct PickListMulti<
    'a,
    T,
    L,
    V,
    Message,
    Theme = iced_core::Theme,
    Renderer = iced_widget::Renderer,
> where
    T: ToString + PartialEq + Clone,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    on_select: Box<dyn Fn((Option<T>, SelectionState)) -> Message + 'a>,
    on_open: Option<Message>,
    on_close: Option<Message>,
    options: L,
    placeholder: Option<String>,
    symbols: [&'a str; 3],
    selection: &'a Vec<(Option<V>, SelectionState)>,
    selection_label: Option<String>,
    optional: bool,
    none_label: String,
    none_separator: bool,
    exclusion_mode: bool,
    width: Length,
    padding: Padding,
    text_size: Option<Pixels>,
    text_line_height: text::LineHeight,
    text_shaping: text::Shaping,
    font: Option<Renderer::Font>,
    symbols_font: Option<Renderer::Font>,
    handle: Handle<Renderer::Font>,
    class: <Theme as Catalog>::Class<'a>,
    menu_class: <Theme as menu_multi::Catalog>::Class<'a>,
    last_status: Option<Status>,
    menu_height: Length,
}

impl<'a, T, L, V, Message, Theme, Renderer> PickListMulti<'a, T, L, V, Message, Theme, Renderer>
where
    T: ToString + PartialEq + Clone,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
    Message: Clone,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    /// Creates a new [`PickListMulti`] with the given list of options, the current
    /// selected value, and the message to produce when an option is selected.
    pub fn new(
        options: L,
        selection: &'a Vec<(Option<V>, SelectionState)>,
        on_select: impl Fn((Option<T>, SelectionState)) -> Message + 'a,
    ) -> Self {
        Self {
            on_select: Box::new(on_select),
            on_open: None,
            on_close: None,
            options,
            placeholder: None,
            symbols: ["", "✓", "✗"],
            selection,
            selection_label: None,
            optional: false,
            none_separator: false,
            none_label: Default::default(),
            exclusion_mode: false,
            width: Length::Shrink,
            padding: DEFAULT_PADDING,
            text_size: None,
            text_line_height: text::LineHeight::default(),
            text_shaping: text::Shaping::default(),
            font: None,
            symbols_font: None,
            handle: Handle::default(),
            class: <Theme as Catalog>::default(),
            menu_class: <Theme as Catalog>::default_menu(),
            last_status: None,
            menu_height: Length::Shrink,
        }
    }

    /// Sets the placeholder of the [`PickListMulti`].
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Sets the symbols used for selection states in the [`Menu`] of the
    /// [`PickListMulti`].
    ///
    /// Format: [unselected, included, excluded]
    pub fn symbols(mut self, symbols: [&'a str; 3]) -> Self {
        self.symbols = symbols;
        self
    }

    /// Sets the selection label of the [`PickListMulti`] to show when selection
    /// is not empty.
    pub fn selection_label(mut self, selection_label: impl Into<String>) -> Self {
        self.selection_label = Some(selection_label.into());
        self
    }

    /// Sets the optional of the [`PickListMulti`].
    pub fn optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }

    /// Sets the None label of the [`PickListMulti`].
    pub fn none_label(mut self, none_label: impl Into<String>) -> Self {
        self.none_label = none_label.into();
        self
    }

    /// Sets whether a separator is drawn between None and first option of the [`PickListMulti`].
    pub fn none_separator(mut self, none_separator: bool) -> Self {
        self.none_separator = none_separator;
        self
    }

    /// Sets if it has the exclusion state of the [`PickListMulti`].
    pub fn exclusion_mode(mut self, exclusion_mode: bool) -> Self {
        self.exclusion_mode = exclusion_mode;
        self
    }

    /// Sets the width of the [`PickListMulti`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Menu`].
    pub fn menu_height(mut self, menu_height: impl Into<Length>) -> Self {
        self.menu_height = menu_height.into();
        self
    }

    /// Sets the [`Padding`] of the [`PickListMulti`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the text size of the [`PickListMulti`].
    pub fn text_size(mut self, size: impl Into<Pixels>) -> Self {
        self.text_size = Some(size.into());
        self
    }

    /// Sets the text [`text::LineHeight`] of the [`PickListMulti`].
    pub fn text_line_height(mut self, line_height: impl Into<text::LineHeight>) -> Self {
        self.text_line_height = line_height.into();
        self
    }

    /// Sets the [`text::Shaping`] strategy of the [`PickListMulti`].
    pub fn text_shaping(mut self, shaping: text::Shaping) -> Self {
        self.text_shaping = shaping;
        self
    }

    /// Sets the font of the [`PickListMulti`].
    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the symbols font of the [`PickListMulti`].
    pub fn symbols_font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.symbols_font = Some(font.into());
        self
    }

    /// Sets the [`Handle`] of the [`PickListMulti`].
    pub fn handle(mut self, handle: Handle<Renderer::Font>) -> Self {
        self.handle = handle;
        self
    }

    /// Sets the message that will be produced when the [`PickListMulti`] is opened.
    pub fn on_open(mut self, on_open: Message) -> Self {
        self.on_open = Some(on_open);
        self
    }

    /// Sets the message that will be produced when the [`PickListMulti`] is closed.
    pub fn on_close(mut self, on_close: Message) -> Self {
        self.on_close = Some(on_close);
        self
    }

    /// Sets the style of the [`PickListMulti`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        <Theme as Catalog>::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style of the [`Menu`].
    #[must_use]
    pub fn menu_style(mut self, style: impl Fn(&Theme) -> menu_multi::Style + 'a) -> Self
    where
        <Theme as menu_multi::Catalog>::Class<'a>: From<menu_multi::StyleFn<'a, Theme>>,
    {
        self.menu_class = (Box::new(style) as menu_multi::StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`PickListMulti`].
    #[cfg(feature = "advanced")]
    #[must_use]
    pub fn class(mut self, class: impl Into<<Theme as Catalog>::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }

    /// Sets the style class of the [`Menu`].
    #[must_use]
    pub fn menu_class(
        mut self,
        class: impl Into<<Theme as menu_multi::Catalog>::Class<'a>>,
    ) -> Self {
        self.menu_class = class.into();
        self
    }
}

impl<'a, T, L, V, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for PickListMulti<'a, T, L, V, Message, Theme, Renderer>
where
    T: Clone + ToString + PartialEq + 'a,
    L: Borrow<[T]>,
    V: Borrow<T>,
    Message: Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: text::Renderer + 'a,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer::Paragraph>::new())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Shrink,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        let font = self.font.unwrap_or_else(|| renderer.default_font());
        let text_size = self.text_size.unwrap_or_else(|| renderer.default_size());
        let options = self.options.borrow();

        state.options.resize_with(options.len(), Default::default);

        let option_text = Text {
            content: "",
            bounds: Size::new(
                f32::INFINITY,
                self.text_line_height.to_absolute(text_size).into(),
            ),
            size: text_size,
            line_height: self.text_line_height,
            font,
            align_x: text::Alignment::Default,
            align_y: alignment::Vertical::Center,
            shaping: self.text_shaping,
            wrapping: text::Wrapping::default(),
        };

        for (option, paragraph) in options.iter().zip(state.options.iter_mut()) {
            let label = option.to_string();

            paragraph.update(Text {
                content: &label,
                ..option_text
            });
        }

        if let Some(placeholder) = &self.placeholder {
            state.placeholder.update(Text {
                content: placeholder,
                ..option_text
            });
        }

        let max_width = match self.width {
            Length::Shrink => {
                let labels_width = state.options.iter().fold(0.0, |width, paragraph| {
                    f32::max(width, paragraph.min_width())
                });

                labels_width.max(
                    self.placeholder
                        .as_ref()
                        .map(|_| state.placeholder.min_width())
                        .unwrap_or(0.0),
                )
            }
            _ => 0.0,
        };

        let size = {
            let intrinsic = Size::new(
                max_width + text_size.0 + self.padding.left,
                f32::from(self.text_line_height.to_absolute(text_size)),
            );

            limits
                .width(self.width)
                .shrink(self.padding)
                .resolve(self.width, Length::Shrink, intrinsic)
                .expand(self.padding)
        };

        layout::Node::new(size)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if state.is_open {
                    // Event wasn't processed by overlay, so cursor was clicked either outside its
                    // bounds or on the drop-down, either way we close the overlay.
                    state.is_open = false;

                    if let Some(on_close) = &self.on_close {
                        shell.publish(on_close.clone());
                    }

                    shell.capture_event();
                } else if cursor.is_over(layout.bounds()) {
                    state.is_open = true;

                    state.hovered_option = if !self.options.borrow().is_empty() {
                        Some(0)
                    } else {
                        None
                    };

                    if let Some(on_open) = &self.on_open {
                        shell.publish(on_open.clone());
                    }

                    shell.capture_event();
                }
            }
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.keyboard_modifiers = *modifiers;
            }
            _ => {}
        };

        let status = {
            let is_hovered = cursor.is_over(layout.bounds());

            if state.is_open {
                Status::Opened { is_hovered }
            } else if is_hovered {
                Status::Hovered
            } else {
                Status::Active
            }
        };

        if let Event::Window(window::Event::RedrawRequested(_now)) = event {
            self.last_status = Some(status);
        } else if self
            .last_status
            .is_some_and(|last_status| last_status != status)
        {
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        let is_mouse_over = cursor.is_over(bounds);

        if is_mouse_over {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let font = self.font.unwrap_or_else(|| renderer.default_font());
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

        let bounds = layout.bounds();
        let has_selection = !self.selection.is_empty();

        let style = Catalog::style(
            theme,
            &self.class,
            self.last_status.unwrap_or(Status::Active),
        );

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: style.border,
                ..renderer::Quad::default()
            },
            style.background,
        );

        let handle = match &self.handle {
            Handle::Arrow { size } => Some((
                Renderer::ICON_FONT,
                Renderer::ARROW_DOWN_ICON,
                *size,
                text::LineHeight::default(),
                text::Shaping::default(),
            )),
            Handle::Static(Icon {
                font,
                code_point,
                size,
                line_height,
                shaping,
            }) => Some((*font, *code_point, *size, *line_height, *shaping)),
            Handle::Dynamic { open, closed } => {
                if state.is_open {
                    Some((
                        open.font,
                        open.code_point,
                        open.size,
                        open.line_height,
                        open.shaping,
                    ))
                } else {
                    Some((
                        closed.font,
                        closed.code_point,
                        closed.size,
                        closed.line_height,
                        closed.shaping,
                    ))
                }
            }
            Handle::None => None,
        };

        if let Some((font, code_point, size, line_height, shaping)) = handle {
            let size = size.unwrap_or_else(|| renderer.default_size());

            renderer.fill_text(
                Text {
                    content: code_point.to_string(),
                    size,
                    line_height,
                    font,
                    bounds: Size::new(bounds.width, f32::from(line_height.to_absolute(size))),
                    align_x: text::Alignment::Right,
                    align_y: alignment::Vertical::Center,
                    shaping,
                    wrapping: text::Wrapping::default(),
                },
                Point::new(
                    bounds.x + bounds.width - self.padding.right,
                    bounds.center_y(),
                ),
                style.handle_color,
                *viewport,
            );
        }

        let label = has_selection
            .then(|| {
                if let Some(selection_label) = self.selection_label.as_ref() {
                    return Some(selection_label.clone());
                }

                let mut items = Vec::new();
                let show_none = self.optional && !self.none_label.is_empty();

                for (selection, state) in self.selection.iter() {
                    let symbol = match state {
                        SelectionState::Unselected => self.symbols[0],
                        SelectionState::Included => self.symbols[1],
                        SelectionState::Excluded => self.symbols[2],
                    };

                    match (selection, self.exclusion_mode) {
                        (None, true) if show_none => {
                            if !symbol.is_empty() {
                                items.push(format!("({}) {}", symbol, self.none_label));
                            }
                        }
                        (None, false) if show_none && state == &SelectionState::Included => {
                            items.push(self.none_label.clone());
                        }
                        (Some(value), true) => {
                            if !symbol.is_empty() {
                                items.push(format!("({}) {}", symbol, value.borrow().to_string()));
                            }
                        }
                        (Some(value), false) if state == &SelectionState::Included => {
                            items.push(value.borrow().to_string());
                        }
                        _ => {}
                    }
                }

                (!items.is_empty()).then(|| items.join(", "))
            })
            .flatten();

        if let Some(label) = label.or_else(|| self.placeholder.clone()) {
            let text_size = self.text_size.unwrap_or_else(|| renderer.default_size());

            renderer.fill_text(
                Text {
                    content: label,
                    size: text_size,
                    line_height: self.text_line_height,
                    font,
                    bounds: Size::new(
                        bounds.width - self.padding.x(),
                        f32::from(self.text_line_height.to_absolute(text_size)),
                    ),
                    align_x: text::Alignment::Left,
                    align_y: alignment::Vertical::Center,
                    shaping: self.text_shaping,
                    wrapping: text::Wrapping::default(),
                },
                Point::new(bounds.x + self.padding.left, bounds.center_y()),
                if has_selection {
                    style.text_color
                } else {
                    style.placeholder_color
                },
                *viewport,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        let font = self.font.unwrap_or_else(|| renderer.default_font());
        let symbols_font = self.symbols_font.unwrap_or(font);

        if state.is_open {
            let bounds = layout.bounds();

            let on_select = &self.on_select;

            let mut menu = Menu::new(
                &mut state.menu,
                self.options.borrow(),
                self.symbols,
                self.optional,
                &self.none_label,
                self.none_separator,
                self.exclusion_mode,
                &mut state.hovered_option,
                self.selection,
                on_select,
                None,
                &self.menu_class,
            )
            .width(bounds.width)
            .padding(self.padding)
            .font(font)
            .symbols_font(symbols_font);

            if let Some(text_size) = self.text_size {
                menu = menu.text_size(text_size);
            }

            Some(menu.overlay(
                layout.position() + translation,
                *viewport,
                bounds.height,
                self.menu_height,
            ))
        } else {
            None
        }
    }
}

impl<'a, T, L, V, Message, Theme, Renderer>
    From<PickListMulti<'a, T, L, V, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    T: Clone + ToString + PartialEq + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
    Message: Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(pick_list: PickListMulti<'a, T, L, V, Message, Theme, Renderer>) -> Self {
        Self::new(pick_list)
    }
}

#[derive(Debug)]
struct State<P: text::Paragraph> {
    menu: menu_multi::State,
    keyboard_modifiers: keyboard::Modifiers,
    is_open: bool,
    hovered_option: Option<usize>,
    options: Vec<paragraph::Plain<P>>,
    placeholder: paragraph::Plain<P>,
}

impl<P: text::Paragraph> State<P> {
    /// Creates a new [`State`] for a [`PickListMulti`].
    fn new() -> Self {
        Self {
            menu: menu_multi::State::default(),
            keyboard_modifiers: keyboard::Modifiers::default(),
            is_open: bool::default(),
            hovered_option: Option::default(),
            options: Vec::new(),
            placeholder: paragraph::Plain::default(),
        }
    }
}

impl<P: text::Paragraph> Default for State<P> {
    fn default() -> Self {
        Self::new()
    }
}

/// The handle to the right side of the [`PickListMulti`].
#[derive(Debug, Clone, PartialEq)]
pub enum Handle<Font> {
    /// Displays an arrow icon (▼).
    ///
    /// This is the default.
    Arrow {
        /// Font size of the content.
        size: Option<Pixels>,
    },
    /// A custom static handle.
    Static(Icon<Font>),
    /// A custom dynamic handle.
    Dynamic {
        /// The [`Icon`] used when [`PickListMulti`] is closed.
        closed: Icon<Font>,
        /// The [`Icon`] used when [`PickListMulti`] is open.
        open: Icon<Font>,
    },
    /// No handle will be shown.
    None,
}

impl<Font> Default for Handle<Font> {
    fn default() -> Self {
        Self::Arrow { size: None }
    }
}

/// The icon of a [`Handle`].
#[derive(Debug, Clone, PartialEq)]
pub struct Icon<Font> {
    /// Font that will be used to display the `code_point`,
    pub font: Font,
    /// The unicode code point that will be used as the icon.
    pub code_point: char,
    /// Font size of the content.
    pub size: Option<Pixels>,
    /// Line height of the content.
    pub line_height: text::LineHeight,
    /// The shaping strategy of the icon.
    pub shaping: text::Shaping,
}

/// The possible status of a [`PickListMulti`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The [`PickListMulti`] can be interacted with.
    Active,
    /// The [`PickListMulti`] is being hovered.
    Hovered,
    /// The [`PickList`] is open.
    Opened {
        /// Whether the [`PickList`] is hovered, while open.
        is_hovered: bool,
    },
}

/// The appearance of a pick list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The text [`Color`] of the pick list.
    pub text_color: Color,
    /// The placeholder [`Color`] of the pick list.
    pub placeholder_color: Color,
    /// The handle [`Color`] of the pick list.
    pub handle_color: Color,
    /// The [`Background`] of the pick list.
    pub background: Background,
    /// The [`Border`] of the pick list.
    pub border: Border,
}

/// The theme catalog of a [`PickListMulti`].
pub trait Catalog: menu_multi::Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> <Self as Catalog>::Class<'a>;

    /// The default class for the menu of the [`PickListMulti`].
    fn default_menu<'a>() -> <Self as menu_multi::Catalog>::Class<'a> {
        <Self as menu_multi::Catalog>::default()
    }

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &<Self as Catalog>::Class<'_>, status: Status) -> Style;
}

/// A styling function for a [`PickListMulti`].
///
/// This is just a boxed closure: `Fn(&Theme, Status) -> Style`.
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, Status) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> StyleFn<'a, Self> {
        Box::new(default)
    }

    fn style(&self, class: &StyleFn<'_, Self>, status: Status) -> Style {
        class(self, status)
    }
}

/// The default style of the field of a [`PickListMulti`].
pub fn default(theme: &Theme, status: Status) -> Style {
    let palette = theme.extended_palette();

    let active = Style {
        text_color: palette.background.weak.text,
        background: palette.background.weak.color.into(),
        placeholder_color: palette.secondary.base.color,
        handle_color: palette.background.weak.text,
        border: Border {
            radius: 2.0.into(),
            width: 1.0,
            color: palette.background.strong.color,
        },
    };

    match status {
        Status::Active => active,
        Status::Hovered | Status::Opened { .. } => Style {
            border: Border {
                color: palette.primary.strong.color,
                ..active.border
            },
            ..active
        },
    }
}

/// The default [`Padding`] of a [`PickListMulti`].
pub const DEFAULT_PADDING: Padding = Padding {
    top: 5.0,
    bottom: 5.0,
    right: 10.0,
    left: 10.0,
};

pub fn update_selection<T: PartialEq>(
    list: &mut Vec<(T, SelectionState)>,
    value: T,
    new_state: SelectionState,
) {
    if new_state == SelectionState::Unselected {
        list.retain(|(item, _)| item != &value);
    } else if let Some((_, state)) = list.iter_mut().find(|(item, _)| item == &value) {
        *state = new_state;
    } else {
        list.push((value, new_state));
    }
}
