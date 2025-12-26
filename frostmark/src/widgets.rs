use iced::{Color, Element, Font, advanced, widget::{self, button::{self, Status}}};

pub fn link<'a, M: 'a, F>(
    e: impl Into<Element<'a, M>>,
    url: &str,
    msg: Option<&F>,
) -> widget::Button<'a, M>
where
    // T: widget::button::Catalog + widget::rule::Catalog + 'a,
    F: Fn(String) -> M,
{
    
    widget::button(underline(e))
        .on_press_maybe(msg.map(|n| n(url.to_owned())))
        .padding(0)
        .style(|_,s| {
            button::Style {
                background: Some(match s {
                    Status::Active | Status::Disabled => Color::TRANSPARENT,
                    Status::Hovered => Color::from_rgba8(255, 255, 255, 0.05),
                    Status::Pressed => Color::from_rgba8(0, 0, 0, 0.05)
                }.into()),
                // shadow: iced::Shadow{color: Color::TRANSPARENT, ..Default::default()},
                border: iced::Border::default().color(Color::TRANSPARENT),
                ..Default::default()
            }
        })
}

pub fn link_text<'a, M: 'a, F>(
    e: widget::text::Span<'a, M, Font>,
    url: String,
    msg: Option<&F>,
) -> widget::text::Span<'a, M, Font>
where
    F: Fn(String) -> M,
{
    e.link_maybe(msg.map(|n| n(url))).underline(true)
}

pub fn underline<'a, M: 'a, T: widget::rule::Catalog + 'a, R: advanced::Renderer + 'a>(
    e: impl Into<Element<'a, M, T, R>>,
) -> widget::Stack<'a, M, T, R> {
    widget::stack!(
        widget::column![e.into()],
        widget::column![
            widget::space(),
            widget::rule::horizontal(1),
            widget::Space::new().height(1),
        ]
    )
}
