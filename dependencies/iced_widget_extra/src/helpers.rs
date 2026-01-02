#[cfg(feature = "pair_grid")]
pub use crate::pair_grid::pair_grid;

#[cfg(feature = "table")]
pub use crate::table::table;

/// Creates a new [`Button`] with the provided content.
#[cfg(feature = "action_area")]
pub fn action_area<'a, Message, Theme, Renderer>(
    content: impl Into<iced_core::Element<'a, Message, Theme, Renderer>>,
) -> crate::ActionArea<'a, Message, Theme, Renderer>
where
    Theme: crate::action_area::Catalog + 'a,
    Renderer: iced_core::Renderer,
{
    crate::ActionArea::new(content)
}

/// Create a new [`PickListOption`]
#[cfg(feature = "pick_list_option")]
pub fn pick_list_option<'a, T, L, V, Message, Theme, Renderer>(
    options: L,
    selected: Option<V>,
    on_selected: impl Fn(Option<T>) -> Message + 'a,
) -> crate::PickListOption<'a, T, L, V, Message, Theme, Renderer>
where
    T: ToString + PartialEq + Clone + 'a,
    L: std::borrow::Borrow<[T]> + 'a,
    V: std::borrow::Borrow<T> + 'a,
    Message: Clone,
    Theme: crate::pick_list_option::Catalog + crate::overlay::menu::Catalog,
    Renderer: iced_core::text::Renderer,
{
    crate::PickListOption::new(options, selected, on_selected)
}

/// Create a new [`PickListMulti`]
#[cfg(feature = "pick_list_multi")]
pub fn pick_list_multi<'a, T, L, V, Message, Theme, Renderer>(
    options: L,
    selection: &'a Vec<(Option<V>, crate::pick_list_multi::SelectionState)>,
    on_selected: impl Fn((Option<T>, crate::pick_list_multi::SelectionState)) -> Message + 'a,
) -> crate::PickListMulti<'a, T, L, V, Message, Theme, Renderer>
where
    T: ToString + PartialEq + Clone + 'a,
    L: std::borrow::Borrow<[T]> + 'a,
    V: std::borrow::Borrow<T> + 'a,
    Message: Clone,
    Theme: crate::pick_list_multi::Catalog + crate::overlay::menu_multi::Catalog,
    Renderer: iced_core::text::Renderer,
{
    crate::pick_list_multi::PickListMulti::new(options, selection, on_selected)
}

/// Creates a new [`TextEditor`].
#[cfg(feature = "text_editor")]
pub fn text_editor<'a, Message, Theme, Renderer>(
    content: &'a crate::text_editor::Content<Renderer>,
) -> crate::TextEditor<'a, iced_core::text::highlighter::PlainText, Message, Theme, Renderer>
where
    Message: Clone,
    Theme: crate::text_editor::Catalog + 'a,
    Renderer: iced_core::text::Renderer,
{
    crate::TextEditor::new(content)
}

/// Creates a new [`TextInput`].
#[cfg(feature = "text_input")]
pub fn text_input<'a, Message, Theme, Renderer>(
    placeholder: &str,
    value: &str,
) -> crate::TextInput<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Theme: crate::text_input::Catalog + 'a,
    Renderer: iced_core::text::Renderer,
{
    crate::TextInput::new(placeholder, value)
}
