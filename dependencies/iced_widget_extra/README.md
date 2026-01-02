# Iced widget extra

This crate contains modified [Iced](https://github.com/hecrj/iced) widgets with additional functionality for my use cases.

Every widget is behind a feature. This allows you to cherry pick the widgets you want.

## Installation

Add this to your `Cargo.toml`

```toml
[dependencies]
iced_widget_extra = { git = "https://gitlab.com/mytdragon/iced_widget_extra.git", features = ["full"] }
```

It also uses iced's master branch (`#c952ea8`).

## Widgets

### `ActionArea`

This is a kind of merge between the original `Button` and `MouseArea`. Imagine a `Button` which you can change the `mouse::Interaction` and which interaction takes as priority the children, similar to how `MouseArea` does.

```rs
action_area("Click here")
    .interaction(Interaction::Text);
```

### PairGrid

This used `Table` as a base but modified to work more like a two-column grid, useful for property table. Also useful for forms with label on same line, that cannot have fixed width on label column because of localisation, which would look weird to move from a latin language to a language that use logographic characters, which tends to take much less space.

```rs
let pairs = [
    pair_grid::pair(text("Tag:"), text("test")),
    pair_grid::pair(text("Modified at:"), text("today")),
    pair_grid::pair(text("Label:"), text_input("placeholder", "value")),
];

pair_grid(pairs).spacing(10)
```

### `PickListOption`

This just moidified, from the original `PickList`, the message to use `Option<T>` instead of `T` which allows to pick `None`. You also can change the text of `None` and add a separator between it and the actual values.

```rs
pick_list_option(Language::ALL, self.language, Message::LanguageSelected)
    .optional(true)
    .none_label("System")
    .none_is_value(true)
    .none_separator(true)
    .text_shaping(Shaping::Advanced);
```

### PickListMulti

![Demo PickListMulti](./assets/demo_pick_list_multi.gif)

This is based on the `PickListOption` above but adds a `SelectionState` to the selection `&'a Vec<(Option<V>, SelectionState)>` and message `(Option<T>, SelectionState)` to allow a multi-selection with an exclusion mode (especially useful for filters).

```rs
pick_list_multi(Language::ALL, &self.languages, Message::LanguageSelected)
    .selection_label(format!("{} items selected", self.languages.len()))
    .symbols(["\u{e640}", "\u{f0132}", "\u{f0856}"])
    .symbols_font(Font::with_name("JetBrainsMono Nerd Font"))
    .optional(true)
    .none_label("None")
    .none_separator(true)
    .width(Length::Fill)
    .exclusion_mode(true)
    .text_shaping(Shaping::Advanced);
```

### Table

This added, to the original `Table`:

- Alignment settings for header and content (before was one for both)
- Edge padding disabling (useful when using table as layout and don't want paddings on edges without having to set it to 0 and add `iced::Space` on each element)

```rs
table::column(bold("#"), |(index, _person): (usize, &Person)| {
    text(format!("#{}", index + 1))
})
.header_align_x(Horizontal::Right);

table(columns, self.persons.iter().enumerate())
    .padding(10)
    .edge_padding(EdgePadding::none())
```

### TextEditor

This is added, to the original `TextInput`, 2 events: `on_focus`and `on_blur`.

```rs
text_editor(&self.content)
    .width(300)
    .on_action(Message::ActionPerformed)
    .on_focus(Message::InputFocused)
    .on_blur(Message::InputBlurred)
```

### TextInput

This is added, to the original `TextInput`, 3 events: `on_focus`, `on_blur` and `on_escape`.

```rs
text_input("Enter value...", &self.value)
    .on_focus(Message::InputFocused)
    .on_blur(Message::InputBlurred)
    .on_escape(Message::InputEscaped)
    .on_input(Message::UpdateValue),
```

## Examples

For complete examples, see `examples/`.

## Contributing

Contributions are welcome on current widgets.

## License

MIT

## Acknowledgements

- [iced](https://github.com/iced-rs/iced)
- [Rust programming language](https://www.rust-lang.org/)
- [Sweeten](https://github.com/airstrike/sweeten)
