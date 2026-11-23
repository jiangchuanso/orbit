//! Tabs: a horizontally scrolling strip of [`Tab`]s, each a glyph, a
//! truncating label, an optional unsaved dot and a close button.
//!
//! The tab owns its look in both states — the active fill and primary ink,
//! the resting muted ink and quiet hover (faded through `motion::fade`, so a
//! strip of tabs reads like every other control under the pointer), the
//! press — and its density, which
//! follows its [`ButtonSize`]. The caller owns what the tab is (glyph, label,
//! localized close tooltip), what selecting it does (`on_click` on the returned
//! element) and what closing it does. [`tab_strip`] owns the row: one line,
//! never wrapping, scrolling sideways past its edge instead of crushing tabs.

use gpui::{
    div, px, AnyElement, App, ClickEvent, ElementId, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Pixels, ScrollHandle, SharedString, Stateful, StatefulInteractiveElement,
    Styled, Window,
};

use crate::theme::{
    tokens::{button, ButtonSize, DynamicSpacing, TextSize},
    Theme,
};
use crate::ui::button::IconButton;
use crate::ui::chip::{Badge, BadgeFill};
use crate::motion::{Hover, Hovered};
use crate::ui::style::control::press;

type CloseHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// The inline padding and the glyph-label gap for a tab of `size`: the
/// terminal's Default tabs are a step tighter than the file viewer's Medium.
fn density(size: ButtonSize, theme: &Theme) -> (Pixels, Pixels) {
    match size {
        ButtonSize::Medium | ButtonSize::Large => (
            DynamicSpacing::Base08.px(theme),
            DynamicSpacing::Base06.px(theme),
        ),
        _ => (
            DynamicSpacing::Base06.px(theme),
            DynamicSpacing::Base04.px(theme),
        ),
    }
}

/// One tab.
///
/// ```ignore
/// Tab::new("viewer-tab-3", name)
///     .size(ButtonSize::Medium)
///     .glyph(file_glyph)
///     .active(index == self.active)
///     .dirty(tab.dirty)
///     .close("viewer-tab-close-3", tr!("explorer.close_tab"), on_close)
///     .render(&theme)
///     .on_click(on_select)
/// ```
pub(crate) struct Tab {
    id: ElementId,
    label: SharedString,
    glyph: Option<AnyElement>,
    size: ButtonSize,
    max_w: Pixels,
    active: bool,
    dirty: bool,
    count: usize,
    close: Option<(ElementId, SharedString, CloseHandler)>,
}

impl Tab {
    pub(crate) fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            glyph: None,
            size: ButtonSize::Default,
            max_w: px(220.),
            active: false,
            dirty: false,
            count: 0,
            close: None,
        }
    }

    pub(crate) fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// The label truncates past this width.
    pub(crate) fn max_w(mut self, max_w: Pixels) -> Self {
        self.max_w = max_w;
        self
    }

    /// The leading mark (a file glyph, a shell profile icon), XSmall.
    pub(crate) fn glyph(mut self, glyph: impl IntoElement) -> Self {
        self.glyph = Some(glyph.into_any_element());
        self
    }

    pub(crate) fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Unsaved changes: an accent dot after the label.
    pub(crate) fn dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }

    /// How many items the tab holds (changed files, new issues): a quiet
    /// badge after the label. Zero shows nothing, so an empty tab stays calm.
    pub(crate) fn count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    /// A close button keyed `id`, named by `tip`. Its click does not reach the
    /// tab's own `on_click`: closing shrinks the strip, and a bubbling select
    /// would run against a stale index.
    pub(crate) fn close(
        mut self,
        id: impl Into<ElementId>,
        tip: impl Into<SharedString>,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.close = Some((id.into(), tip.into(), Box::new(on_close)));
        self
    }

    /// The tab; the caller attaches `on_click` (select) and any tooltip.
    ///
    /// The returned [`Hovered`] still takes every handler and style call a
    /// plain `Stateful<Div>` would, so callers chain after it unchanged.
    pub(crate) fn render(self, theme: &Theme) -> Hovered<Stateful<gpui::Div>> {
        let active = self.active;
        // The fade's key: stable per tab and namespaced, so a strip's tab
        // ids cannot collide with another surface's transition names.
        let fade_name: SharedString = format!("tab:{}", self.id).into();
        let (padding, gap) = density(self.size, theme);
        let ink = if active {
            theme.text_primary
        } else {
            theme.text_muted
        };
        let mut el = div()
            .id(self.id)
            .h(self.size.height(theme))
            .max_w(self.max_w)
            // Never shrink below a readable width: past the strip's edge the
            // row scrolls instead of crushing every tab.
            .flex_none()
            .px(padding)
            .flex()
            .items_center()
            .gap(gap)
            .rounded(button::RADIUS.px(theme))
            .cursor_pointer()
            .text_size(TextSize::Small.px(theme));
        // The open tab is already lit, so it has nothing to fade to; a
        // resting one fades in the quiet hover — the wash, and its inherited
        // ink lifting from muted to secondary.
        let hover = if active {
            el = el.bg(theme.pressed_wash());
            Hover::none()
        } else {
            el = el.text_color(theme.text_muted);
            Hover::wash(theme).with_ink(theme.text_muted, theme.text_secondary)
        };
        let el = crate::motion::fade(press(el), fade_name, theme, hover)
            .children(self.glyph)
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .font_weight(if active {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(ink)
                    .child(self.label),
            )
            .children((self.count > 0).then(|| {
                // Quiet in both states: the label already carries the tab's
                // emphasis, the number only says how much is behind it.
                Badge::new(
                    count_label(self.count),
                    if active {
                        theme.text_secondary
                    } else {
                        theme.text_muted
                    },
                    BadgeFill::Solid(theme.overlay),
                )
                .render(theme)
            }))
            .children(self.dirty.then(|| {
                div()
                    .size(DynamicSpacing::Base06.px(theme))
                    .flex_none()
                    .rounded_full()
                    .bg(theme.accent)
            }));
        match self.close {
            Some((id, tip, on_close)) => el.child(
                IconButton::new(id, "icons/x.svg")
                    .size(ButtonSize::None)
                    .ink(ink)
                    .tip(tip)
                    .on_click(theme, move |event, window, cx| {
                        on_close(event, window, cx);
                        cx.stop_propagation();
                    }),
            ),
            None => el,
        }
    }
}

/// A tab count as printed: past 99 the exact number stops being useful and
/// would widen the tab, so it reads "99+".
fn count_label(count: usize) -> String {
    if count > 99 {
        "99+".to_owned()
    } else {
        count.to_string()
    }
}

/// The strip a run of [`Tab`]s of `size` sits in: full height, one line, its
/// own horizontal scroller tracked by `scroll` (so a caller can reveal a tab
/// with `scroll_to_item`).
pub(crate) fn tab_strip(
    id: impl Into<ElementId>,
    scroll: &ScrollHandle,
    size: ButtonSize,
    theme: &Theme,
) -> Stateful<gpui::Div> {
    div()
        .id(id.into())
        .h_full()
        .flex()
        .items_center()
        .gap(density(size, theme).1)
        .overflow_x_scroll()
        .track_scroll(scroll)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeId;

    #[test]
    fn active_tabs_fill_and_resting_tabs_stay_quiet() {
        for id in ThemeId::ALL {
            let theme = Theme::for_id(id);
            let mut active = Tab::new("a", "main.rs").active(true).render(&theme);
            let mut resting = Tab::new("b", "lib.rs").render(&theme);
            assert_eq!(
                active.style().background,
                Some(theme.surface_active.into()),
                "{id:?}"
            );
            assert!(resting.style().background.is_none(), "{id:?}");
            assert_eq!(
                resting.style().text.as_ref().and_then(|text| text.color),
                Some(theme.text_muted),
                "{id:?}"
            );
        }
    }

    /// A resting tab fades its hover on the shared layer instead of swapping
    /// it on one frame, keyed by its own id.
    #[gpui::test]
    fn a_resting_tab_fades_its_hover(cx: &mut gpui::TestAppContext) {
        use crate::motion::registry::Transitions;
        use gpui::{point, Modifiers};

        struct Probe(Theme);

        impl gpui::Render for Probe {
            fn render(
                &mut self,
                _: &mut gpui::Window,
                _: &mut gpui::Context<Self>,
            ) -> impl IntoElement {
                div().w_full().h_full().pt(px(200.0)).child(
                    Tab::new("tab-probe", "main.rs")
                        .render(&self.0)
                        .debug_selector(|| "tab-probe".into())
                        // Callers still chain handlers after `render`.
                        .on_click(|_, _, _| {}),
                )
            }
        }

        let (_probe, cx) = cx.add_window_view(|_, _| Probe(Theme::dark()));
        cx.update(|window, cx| window.draw(cx).clear());
        let tab = cx.debug_bounds("tab-probe").expect("the tab lays out");
        cx.simulate_mouse_move(tab.center(), None, Modifiers::none());
        cx.update(|window, cx| window.draw(cx).clear());
        let hovered = Transitions::get("tab:tab-probe").expect("the hover is tracked");
        assert_eq!(hovered.to(), 1.0);

        cx.simulate_mouse_move(point(px(600.0), px(10.0)), None, Modifiers::none());
        cx.update(|window, cx| window.draw(cx).clear());
        assert_eq!(Transitions::get("tab:tab-probe").map(|t| t.to()), Some(0.0));
    }

    #[test]
    fn a_count_caps_at_ninety_nine() {
        assert_eq!(count_label(1), "1");
        assert_eq!(count_label(99), "99");
        assert_eq!(count_label(100), "99+");
    }

    #[test]
    fn density_follows_the_tab_size() {
        let theme = Theme::for_id(ThemeId::ALL[0]);
        let (default_pad, default_gap) = density(ButtonSize::Default, &theme);
        let (medium_pad, medium_gap) = density(ButtonSize::Medium, &theme);
        assert!(medium_pad > default_pad);
        assert!(medium_gap > default_gap);
    }
}
