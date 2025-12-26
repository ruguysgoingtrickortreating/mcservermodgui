# 0.2.0

(for **iced 0.13.x**)

## Added

- Elements:
  - `<details>` (dropdown) / `<summary>`
  - `<mark>` (highlighted text)
- `MarkState::with_markdown_only` for filtering/sanitizing HTML inputs
  (useful for messaging apps for example)
- `MarkState::default()` for empty documents

### Styling
- `MarkWidget::text_size` and `MarkWidget::heading_scale`

---

## Changed

- iced's `wgpu` and `tiny-skia` backends now togglable via `iced-*` crate features
- default color of links (now blue)

### State updating

- `MarkWidget::on_updating_state` now accepts a `Fn(UpdateMsg) -> Message`.
- `MarkState::update` now accepts a `UpdateMsg`.
- Wrap `UpdateMsg` in your message type and pass it to the update function.

Example:

```diff
  MarkWidget::new(&self.state)
-     .on_updating_state(|| Message::UpdateState)
+     .on_updating_state(|action| Message::UpdateState(action))
```

```diff
- Message::UpdateState => self.state.update(),
+ Message::UpdateState(action) => self.state.update(action),
```

```diff
  enum Message {
-     UpdateState,
+     UpdateState(frostmark::UpdateMsg),
      // ...
  }
```

## Fixed

- Space/whitespace issues in formatted text

---

# 0.1.0

(for **iced 0.13.x**)

This has:

- Bold/Italic/Underline/Strikethrough/Monospace/Subscript
- Code blocks with text selection, Quotes
- Headings, `<hr>`, left/center/right alignment
- Images (bring-your-own-handler)
- A few examples and basic documentation
