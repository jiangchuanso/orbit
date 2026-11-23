//! The shared switch control.
//!
//! From Uiverse.io by RaspberryBee - https://uiverse.io/RaspberryBee/calm-deer-81
//! MIT License, Copyright (c) RaspberryBee
//!
//! Adapted from HTML/CSS to GPUI: the em-based geometry is kept as ratios of
//! the track height, and the colors map to theme roles instead of literals.

use gpui::{
    div, px, AnyElement, App, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, SharedString, Styled, Window,
};

use crate::theme::{
    tokens::{DynamicSpacing, Motion},
    Theme,
};

/// The app's one switch: a rounded-rectangle track (3.5 × 2 em, with `em` half
/// the track height) and a rounded-square knob (1.4 em, inset 0.3 em) that
/// slides 1.5 em on [`Motion::Travel`] while the track cross-fades from
/// `text_disabled` (off) to `success` (on).
///
/// The settings switches (Reduce Motion, automatic updates, a skill's or a
/// subagent's enablement) all render through this, so a switch cannot drift in
/// shape, motion or behavior. `id` must be unique per switch — it keys the
/// transition, the element and the switch's focus.
///
/// It is a keyboard control as well: Tab reaches it, it draws the focus ring
/// while focused, and Space or Enter flips it (`SwitchToggle` in the `Switch`
/// context) through the same `on_toggle` a click runs. A native accessible
/// name is still a GPUI gap.
pub(crate) fn switch(
    id: impl Into<SharedString>,
    on: bool,
    theme: &Theme,
    on_toggle: impl Fn(&mut Window, &mut App) + 'static,
) -> AnyElement {
    Switch {
        id: id.into(),
        on,
        theme: *theme,
        on_toggle: std::rc::Rc::new(on_toggle),
    }
    .into_any_element()
}

/// What a switch runs when it flips, shared by its click and key paths.
type SwitchToggleFn = std::rc::Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
struct Switch {
    id: SharedString,
    on: bool,
    theme: Theme,
    on_toggle: SwitchToggleFn,
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Switch {
            id,
            on,
            theme,
            on_toggle,
        } = self;
        // The handle has to outlive the frame so focus survives a repaint;
        // keyed state lives as long as this switch keeps being drawn.
        let focus = window
            .use_keyed_state(
                gpui::ElementId::Name(format!("{id}-focus").into()),
                cx,
                |_, cx| cx.focus_handle().tab_stop(true),
            )
            .read(cx)
            .clone();
        let geometry = SwitchGeometry::new(&theme);
        let selector = id.clone();
        let track = div()
            .id(gpui::ElementId::Name(id.clone()))
            // Tests find a switch by its id.
            .debug_selector(move || selector.to_string())
            .track_focus(&focus)
            .key_context("Switch")
            .w(geometry.width)
            .h(geometry.height)
            .rounded(geometry.track_radius)
            // The 1px edge only shows the focus ring; the inset is measured
            // from the outer edge, so the padding gives that pixel back.
            .p(geometry.inset - px(1.))
            .border_1()
            .border_color(gpui::transparent_black())
            .flex()
            .items_center()
            .cursor_pointer()
            .on_mouse_up(MouseButton::Left, {
                let on_toggle = on_toggle.clone();
                move |_, window, cx| on_toggle(window, cx)
            })
            .on_action(move |_: &crate::SwitchToggle, window, cx| on_toggle(window, cx));
        crate::motion::state(
            track,
            id,
            &theme,
            Motion::Travel,
            if on { 1.0 } else { 0.0 },
            move |track, value| {
                track
                    .bg(crate::motion::blend(
                        theme.text_disabled,
                        theme.success,
                        value,
                    ))
                    .child(
                        div()
                            .flex_none()
                            .size(geometry.knob)
                            .rounded(geometry.knob_radius)
                            .bg(theme.toggle_knob)
                            // Relative, so the knob travels without re-laying out the
                            // track — and without moving the hitbox the click lands on.
                            .relative()
                            .left(geometry.travel * value),
                    )
            },
        )
    }
}

/// The switch's shape, in the source design's em ratios with `em` = half the
/// track height, so the whole control follows the spacing density.
#[derive(Clone, Copy)]
struct SwitchGeometry {
    width: Pixels,
    height: Pixels,
    knob: Pixels,
    inset: Pixels,
    travel: Pixels,
    track_radius: Pixels,
    knob_radius: Pixels,
}

impl SwitchGeometry {
    fn new(theme: &Theme) -> Self {
        let height = DynamicSpacing::Base20.px(theme);
        let em = height * 0.5;
        Self {
            width: em * 3.5,
            height,
            knob: em * 1.4,
            inset: em * 0.3,
            travel: em * 1.5,
            // 10px and 8px at the source's 17px font size.
            track_radius: em * (10. / 17.),
            knob_radius: em * (8. / 17.),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The knob is inset equally at both ends of its travel at every spacing
    /// density, so it lands flush at the far edge, never past it.
    #[test]
    fn a_switch_knob_travels_the_tracks_inner_span() {
        for density in crate::theme::SPACING_DENSITIES {
            let mut theme = Theme::dark();
            theme.ui.spacing_density = density;
            let g = SwitchGeometry::new(&theme);
            assert!(g.travel > px(0.), "no travel at {density}% density");
            // Track = inset + knob + travel + inset.
            let span = g.inset + g.knob + g.travel + g.inset;
            assert!(
                (span - g.width).abs() < px(0.01),
                "the knob must land flush with the far edge at {density}%"
            );
            // Vertically centered with the same inset.
            assert!((g.inset * 2.0 + g.knob - g.height).abs() < px(0.01));
        }
    }
}
