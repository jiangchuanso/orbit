//! Choice rows: one option in a "decision needed" card — a radio (or check)
//! mark, the option's label with an optional *Recommended* badge, and its
//! description underneath. The questionnaire panel above the composer and the
//! extension `select` card both list their options with it, so a question
//! reads the same wherever pi asks it.
//!
//! The row owns geometry, marks and ink; the caller owns ids, clicks and the
//! highlight, so keyboard and pointer behavior stay with the surface that
//! answers the request.

use gpui::{
    div, prelude::FluentBuilder, px, ElementId, FontWeight, InteractiveElement,
    ParentElement, SharedString, Stateful, Styled,
};

use crate::theme::{
    tokens::{IconSize, Radius, TextSize},
    Theme,
};
use crate::ui::chip::{Badge, BadgeFill};
use crate::ui::icon::icon;
use crate::ui::style::control::ControlExt;

/// The mark at the start of a choice row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChoiceMark {
    /// Single choice: an outlined ring, filled with a dot when chosen.
    Radio(bool),
    /// Multiple choice: an outlined square, filled with a check when on.
    Check(bool),
}

/// Split a trailing "(Recommended)" marker off an option label. Agents mark
/// their pick this way in plain text; the card shows it as a badge instead,
/// so the label itself stays clean. Case-insensitive; returns the label
/// unchanged and `false` when there is no marker.
pub(crate) fn split_recommended(label: &str) -> (&str, bool) {
    const MARKER: &str = "(recommended)";
    let trimmed = label.trim_end();
    if trimmed.len() >= MARKER.len() {
        let cut = trimmed.len() - MARKER.len();
        if trimmed.is_char_boundary(cut) && trimmed[cut..].eq_ignore_ascii_case(MARKER) {
            return (trimmed[..cut].trim_end(), true);
        }
    }
    (label, false)
}

/// One option row of a decision card.
pub(crate) struct ChoiceRow {
    id: ElementId,
    mark: ChoiceMark,
    label: SharedString,
    description: Option<SharedString>,
    recommended: bool,
    highlighted: bool,
    interactive: bool,
}

impl ChoiceRow {
    pub(crate) fn new(id: impl Into<ElementId>, mark: ChoiceMark, label: &str) -> Self {
        let (label, recommended) = split_recommended(label);
        Self {
            id: id.into(),
            mark,
            label: SharedString::from(label.to_string()),
            description: None,
            recommended,
            highlighted: false,
            interactive: true,
        }
    }

    /// Supporting copy under the label; blank text is ignored.
    pub(crate) fn description(mut self, description: &str) -> Self {
        let description = description.trim();
        if !description.is_empty() {
            self.description = Some(SharedString::from(description.to_string()));
        }
        self
    }

    /// Mark the option as the agent's recommendation when the caller found
    /// the marker somewhere other than the label (never clears it).
    pub(crate) fn recommended(mut self, recommended: bool) -> Self {
        self.recommended |= recommended;
        self
    }

    /// The keyboard/pointer cursor is on this row.
    pub(crate) fn highlighted(mut self, highlighted: bool) -> Self {
        self.highlighted = highlighted;
        self
    }

    /// A read-only row (answers already sent) takes no hover or pointer.
    pub(crate) fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    pub(crate) fn render(self, theme: &Theme) -> Stateful<gpui::Div> {
        let chosen = match self.mark {
            ChoiceMark::Radio(on) | ChoiceMark::Check(on) => on,
        };
        let strong = self.highlighted || chosen;
        let line_h = theme.ui_px(18.);

        // The label takes the line's remaining width (`flex_1`, a zero basis)
        // and wraps inside it; the badge keeps its own width beside the first
        // line. A label with an auto basis in a wrapping row was free to shrink
        // to nothing under `min_w_0`, and then wrapped one glyph per line.
        let mut title = div()
            .min_w_0()
            .flex()
            .items_start()
            .gap(px(6.))
            .child(
                div()
                    .debug_selector(|| "choice-label".to_string())
                    .flex_1()
                    .min_w_0()
                    .whitespace_normal()
                    .text_size(TextSize::Default.px(theme))
                    .line_height(line_h)
                    .font_weight(if strong {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if strong {
                        theme.text_primary
                    } else {
                        theme.text_secondary
                    })
                    .child(self.label),
            );
        if self.recommended {
            title = title.child(
                div()
                    .debug_selector(|| "choice-recommended".to_string())
                    .flex_none()
                    .child(
                        Badge::new(
                            tr!("ask.recommended"),
                            theme.accent_text,
                            BadgeFill::Solid(theme.accent.opacity(0.14)),
                        )
                        .leading(icon(
                            "icons/star.svg",
                            IconSize::Indicator.px(theme),
                            theme.accent_text,
                        ))
                        .render(theme),
                    ),
            );
        }

        let mut copy = div()
            .min_w_0()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(title);
        if let Some(description) = self.description {
            copy = copy.child(
                div()
                    .whitespace_normal()
                    .text_size(TextSize::Small.px(theme))
                    .line_height(theme.ui_px(16.))
                    .text_color(theme.text_muted)
                    .child(description),
            );
        }

        div()
            .id(self.id)
            .w_full()
            .min_w_0()
            .px(px(10.))
            .py(px(7.))
            .rounded(Radius::Large.px(theme))
            .flex()
            .items_start()
            .gap(px(10.))
            .when(self.highlighted, |row| row.bg(theme.selected_wash()))
            .when(self.interactive, |row| row.cursor_pointer().hover_on_glass(theme))
            .child(
                // Centre the mark on the label's first line.
                div()
                    .flex_none()
                    .h(line_h)
                    .flex()
                    .items_center()
                    .child(mark(self.mark, theme)),
            )
            .child(copy)
    }
}

/// The drawn radio / check mark. Accent only when on, so an unanswered card
/// stays quiet and the chosen row is found at a glance.
fn mark(mark: ChoiceMark, theme: &Theme) -> gpui::Div {
    let size = theme.ui_px(16.);
    match mark {
        ChoiceMark::Radio(on) => div()
            .size(size)
            .flex_none()
            .rounded_full()
            .border_1()
            .border_color(if on { theme.accent } else { theme.border_strong })
            .flex()
            .items_center()
            .justify_center()
            .when(on, |ring| {
                ring.child(
                    div()
                        .size(theme.ui_px(8.))
                        .rounded_full()
                        .bg(theme.accent),
                )
            }),
        ChoiceMark::Check(on) => div()
            .size(size)
            .flex_none()
            .rounded(Radius::Small.px(theme))
            .border_1()
            .border_color(if on { theme.accent } else { theme.border_strong })
            .when(on, |square| square.bg(theme.accent))
            .flex()
            .items_center()
            .justify_center()
            .when(on, |square| {
                square.child(icon(
                    "icons/check.svg",
                    IconSize::Indicator.px(theme),
                    theme.accent_fg,
                ))
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::split_recommended;
    use super::{ChoiceMark, ChoiceRow};
    use crate::theme::{Theme, ThemeId};
    use gpui::{div, point, prelude::*, px, ElementId};

    /// The decision card's chain: a full-width card, the scrolling options
    /// column, a row — the label must lay out on one line at that width, never
    /// one glyph per line.
    #[gpui::test]
    fn a_choice_label_wraps_at_the_row_width_not_per_glyph(cx: &mut gpui::TestAppContext) {
        struct Probe(&'static str, Option<&'static str>, bool);
        impl gpui::Render for Probe {
            fn render(
                &mut self,
                _: &mut gpui::Window,
                _: &mut gpui::Context<Self>,
            ) -> impl IntoElement {
                let theme = Theme::for_id(ThemeId::Orbit);
                let mut row = ChoiceRow::new(
                    ElementId::Name("probe-row".into()),
                    ChoiceMark::Radio(true),
                    self.0,
                )
                .recommended(self.2)
                .highlighted(true);
                if let Some(description) = self.1 {
                    row = row.description(description);
                }
                div().w(px(600.)).flex().flex_col().child(
                    div()
                        .id("probe-options")
                        .max_h(px(400.))
                        .overflow_y_scroll()
                        .px(px(6.))
                        .flex()
                        .flex_col()
                        .child(row.render(&theme)),
                )
            }
        }

        cx.update(|cx| cx.set_global(Theme::for_id(ThemeId::Orbit)));
        for (label, description, recommended) in [
            ("Return only", None, false),
            ("Return only", Some("Hand the review back without merging"), false),
            ("Return only", None, true),
        ] {
            let (view, cx) =
                cx.add_window_view(|_, _| Probe(label, description, recommended));
            let _ = cx.draw(point(px(0.), px(0.)), gpui::size(px(700.), px(500.)), |_, _| {
                view.clone()
            });
            let label_bounds = cx.debug_bounds("choice-label").expect("label laid out");
            assert!(
                label_bounds.size.width > px(60.) && label_bounds.size.height < px(24.),
                "{label:?} (description {description:?}, recommended {recommended}) must be one line: {label_bounds:?}"
            );
        }
    }

    #[test]
    fn recommended_marker_becomes_a_flag() {
        assert_eq!(split_recommended("Leave it (Recommended)"), ("Leave it", true));
        assert_eq!(split_recommended("Leave it (recommended)  "), ("Leave it", true));
        assert_eq!(split_recommended("Build it next"), ("Build it next", false));
        assert_eq!(split_recommended("(Recommended)"), ("", true));
        assert_eq!(split_recommended("é"), ("é", false));
    }
}
