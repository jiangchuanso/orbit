//! Keyboard keys for inline shortcut hints: one keycap, a chord of keycaps,
//! and the working-folder pickers' key legend.

use crate::theme::tokens::{picker, ButtonSize, DynamicSpacing, Radius, TextSize};
use crate::theme::Theme;
use gpui::{div, AnyElement, InteractiveElement, IntoElement, ParentElement, SharedString, Styled};

/// The bundled symbols font (`assets/fonts/SymbolsNerdFont-Regular.ttf`),
/// registered at launch, so a keycap never depends on what the OS has.
const KEY_ICON_FONT: &str = "Symbols Nerd Font";

/// The keyboard icon the bundled symbols font draws for a macOS key mark.
///
/// The marks themselves (`↵`, `⌥`, `⌘`, `⌃`, `⇧`) are text: each comes from
/// whichever font covers it and sits at that font's own height, and text
/// layout centres the line box, not the ink. `↵` in particular is drawn above
/// the middle, so in a chord it rode about 1.5px higher than the `⌥` beside it.
/// The icon font's keyboard glyphs are drawn on the middle of the em box, so
/// every cap centres its mark the same way. Words (`Esc`, `Tab`, `Enter`) and
/// letters stay text.
fn key_icon(label: &str) -> Option<char> {
    Some(match label {
        "↵" => '\u{F0311}', // keyboard-return
        "⌥" => '\u{F0635}', // apple-keyboard-option
        "⌘" => '\u{F0633}', // apple-keyboard-command
        "⌃" => '\u{F0634}', // apple-keyboard-control
        "⇧" => '\u{F0636}', // apple-keyboard-shift
        _ => return None,
    })
}

/// Render a key label supplied by the caller.
pub(crate) fn kbd_key(label: impl Into<SharedString>, theme: &Theme) -> AnyElement {
    let label = label.into();
    let cap = div()
        .h(ButtonSize::Compact.height(theme))
        .min_w(ButtonSize::Compact.height(theme))
        .px(DynamicSpacing::Base04.px(theme))
        .rounded(Radius::Small.px(theme))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .bg(theme.overlay_strong)
        .text_size(TextSize::XSmall.px(theme))
        .text_color(theme.text_secondary);
    match key_icon(&label) {
        Some(glyph) => cap
            .debug_selector(|| "kbd-key-icon".to_string())
            .child(div().font_family(KEY_ICON_FONT).child(glyph.to_string())),
        None => cap.child(label),
    }
    .into_any_element()
}

/// Group the platform-specific key labels of a canonical shortcut chord.
pub(crate) fn kbd_shortcut(chord: &str, theme: &Theme) -> AnyElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(DynamicSpacing::Base02.px(theme))
        .children(
            chord
                .split('-')
                .filter(|part| !part.is_empty())
                .map(|part| kbd_key(crate::platform::shortcuts::label(part), theme)),
        )
        .into_any_element()
}

/// Keyboard legend shared by the working-folder pickers, as their whole footer
/// (no note). The remote level uses Escape to go back; elsewhere it closes the
/// picker.
pub(crate) fn working_folder_keyboard_hints(theme: &Theme) -> gpui::Div {
    working_folder_footer(None, theme)
}

/// A picker's footer zone: **one** hairline, then an optional note (what the
/// choice does — "Choosing a folder starts a new task there"), then the key
/// legend. Note and legend share the header's and the rows' 10px edge, and the
/// note wraps rather than clipping a longer translation. Grouped under a single
/// rule so a note never stacks a second hairline on the legend's.
pub(crate) fn working_folder_footer(note: Option<SharedString>, theme: &Theme) -> gpui::Div {
    div()
        .debug_selector(|| "picker-footer".to_string())
        .flex_none()
        .w_full()
        .flex()
        .flex_col()
        .border_t_1()
        .border_color(theme.hairline())
        .children(note.map(|note| {
            div()
                .debug_selector(|| "picker-footer-note".to_string())
                .w_full()
                .px(picker::search_padding_x(theme))
                .pt(DynamicSpacing::Base06.px(theme))
                .whitespace_normal()
                .text_size(TextSize::XSmall.px(theme))
                .text_color(theme.text_muted)
                .child(
                    div()
                        .debug_selector(|| "picker-footer-note-text".to_string())
                        .w_full()
                        .child(note),
                )
        }))
        .child(keyboard_legend(theme))
}

fn keyboard_legend(theme: &Theme) -> gpui::Div {
    let group = |id: &'static str, keys: &[&str], label: String| {
        div()
            .debug_selector(move || id.to_string())
            .flex_none()
            .flex()
            .items_center()
            .gap(DynamicSpacing::Base02.px(theme))
            .children(
                keys.iter()
                    .map(|key| kbd_key(crate::platform::shortcuts::label(key), theme)),
            )
            .child(div().whitespace_nowrap().child(label))
    };
    div()
        .debug_selector(|| "working-folder-keyboard-hints".to_string())
        .flex_none()
        .w_full()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(DynamicSpacing::Base08.px(theme))
        .px(picker::search_padding_x(theme))
        .py(DynamicSpacing::Base06.px(theme))
        .text_size(TextSize::XSmall.px(theme))
        .text_color(theme.text_muted)
        .child(group(
            "working-folder-hint-navigate",
            &["up", "down"],
            tr!("working_folder.hint_navigate"),
        ))
        .child(group(
            "working-folder-hint-select",
            &["enter"],
            tr!("working_folder.hint_select"),
        ))
        .child(group(
            "working-folder-hint-close",
            &["escape"],
            tr!("common.close"),
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every macOS modifier and the Return mark draw as the symbols font's
    /// keyboard icon, so the caps in one chord centre alike; words and
    /// letters stay text.
    #[test]
    fn key_marks_draw_as_centred_icons() {
        for mark in ["↵", "⌥", "⌘", "⌃", "⇧"] {
            assert!(key_icon(mark).is_some(), "{mark} keeps its text glyph");
        }
        for text in ["Esc", "Tab", "Enter", "Alt", "B", "1", "↑"] {
            assert!(key_icon(text).is_none(), "{text} must stay text");
        }
    }

    /// The icons come from the bundled symbols font, not the system's.
    #[test]
    fn key_icons_use_the_bundled_symbols_font() {
        use gpui::AssetSource as _;
        assert!(crate::assets::Assets
            .load("fonts/SymbolsNerdFont-Regular.ttf")
            .ok()
            .flatten()
            .is_some());
        assert_eq!(KEY_ICON_FONT, "Symbols Nerd Font");
    }
    use crate::ui::row::{picker_entry, PickerRow};
    use gpui::{point, px, Context, IntoElement, Render, TestAppContext, Window};

    /// A picker row over the shared footer, as a picker lays them out.
    struct FooterProbe;

    impl Render for FooterProbe {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = *crate::theme::get(cx);
            div()
                .w(px(300.))
                .flex()
                .flex_col()
                .child(
                    picker_entry(div(), &theme).child(
                        PickerRow::new("Row", theme.text_primary, theme.text_muted)
                            .icon("icons/folder.svg", theme.text_muted)
                            .into_element(&theme),
                    ),
                )
                .child(working_folder_footer(
                    Some("Choosing a folder starts a new task there.".into()),
                    &theme,
                ))
        }
    }

    /// The footer's note starts on the rows' edge — the glyph's — instead of
    /// the 4px inset a vertical padding token once gave it.
    #[gpui::test]
    fn the_footer_note_shares_the_rows_edge(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(crate::theme::Theme::for_id(crate::theme::ThemeId::Orbit))
        });
        let (view, cx) = cx.add_window_view(|_, _| FooterProbe);
        cx.draw(point(px(0.), px(0.)), gpui::size(px(400.), px(300.)), |_, _| {
            view.clone()
        });
        let glyph = cx.debug_bounds("picker-row-leading").expect("row glyph");
        let text = cx
            .debug_bounds("picker-footer-note-text")
            .expect("note text");
        assert!(
            (text.left() - glyph.left()).abs() <= px(1.),
            "the note's text starts on the rows' edge: {text:?} vs {glyph:?}"
        );
    }
}
