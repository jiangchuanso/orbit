//! A panel taking space from its neighbour: the open and close slide.
//!
//! The sessions sidebar, the right dock (Review, a subagent tab, the Explorer,
//! session details) and the bottom terminal strip all change how much room their
//! neighbour gets. `DESIGN.md` §Motion System assigns that change to
//! `Motion::Layout` and names the animated property as *the panel's own width*,
//! so it is one clock shared by the panel and anything that rides its edge.
//!
//! [`Anchor`] carries the axis as well as the edge: the docks take a width along
//! a row and the terminal takes a height along a column. Nothing else differs.
//!
//! # The shape
//!
//! A panel cannot animate its own removal. gpui drops an element's animation
//! state with the element, and `with_animation` has no exit phase, so an
//! unmounted panel has nothing to interpolate — which is why the docks used to
//! swap on one frame. The slide therefore keeps the panel mounted while it
//! moves and eases a wrapper extent that **clips** a fixed-extent inner column:
//!
//! ```text
//! wrapper: extent × progress, overflow hidden, anchored to one edge
//! ┌──────────────────────────────────────┐
//! │                 ┌──── inner: extent ──┐│
//! │  revealed  ←────│ the panel's content ││
//! └─────────────────┴─────────────────────┘┘
//! ```
//!
//! The inner keeps its settled extent for the whole slide, so the panel never
//! reflows: it is *uncovered*, not squeezed. [`Anchor`] says which edge it stays
//! flush with, and therefore which side the wrapper clips.
//!
//! # Why a `Transition` and not a generation counter
//!
//! The first cut of this — the sidebar's — keyed `with_animation` on a counter
//! bumped by the toggle. That cannot be interrupted: a close caught by a reopen
//! restarts from zero or from the full extent, so a fast double-click snaps.
//! [`Transition`] remembers the value on screen instead, which is what
//! `DESIGN.md` requires of every reversible transition, and it supplies the
//! changing element id itself. This is the same contract the hover layer uses,
//! one surface up.
//!
//! # Reduce Motion
//!
//! The role's duration collapses to zero, so the panel takes its new extent on
//! the frame it changes and a closing dock is not mounted at all: the end state,
//! per `DESIGN.md`, not a disabled animation.

use std::time::Instant;

use gpui::{
    div, Animation, AnimationExt, AnyElement, Div, ElementId, InteractiveElement, IntoElement,
    ParentElement, Pixels, SharedString, Styled,
};

use super::registry::Transitions;
use super::transition::Transition;
use crate::theme::tokens::Motion;
use crate::theme::Theme;

/// The sessions sidebar's slide, keyed in [`Transitions`].
///
/// Named rather than local because three elements ride it: the panel itself,
/// the top bar's leading pad and the titlebar controls' left offset.
pub(crate) const SIDEBAR: &str = "panel-slide-sidebar";

/// The right dock's slide. Review, a subagent tab, the Explorer and session
/// details share one slot — they are mutually exclusive — so they share one
/// slide.
pub(crate) const RIGHT_DOCK: &str = "panel-slide-right-dock";

/// The bottom terminal strip's slide. It is the only panel that takes height
/// rather than width.
pub(crate) const TERMINAL: &str = "panel-slide-terminal";

/// The edge a sliding panel stays flush with, and therefore the side the
/// wrapper uncovers it from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Anchor {
    /// The sessions sidebar: flush left, revealed from the left, clipped right.
    Left,
    /// The right dock: flush right, revealed from the right, clipped left.
    Right,
    /// The terminal strip: flush with the window's bottom edge, revealed from
    /// below, clipped at the top. The only vertical panel.
    Bottom,
}

impl Anchor {
    /// Whether the panel takes its space along the column's height.
    fn vertical(self) -> bool {
        matches!(self, Self::Bottom)
    }
}

/// What a closed panel costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mount {
    /// Only while it shows or moves. A closed dock is out of the tree, so its
    /// entity is not rendered at all — Review, Explorer and session details all
    /// build real content every frame they are asked to.
    WhileOpen,
    /// Always, at zero extent. The sessions sidebar needs this: its focus handle
    /// has to stay in the tree for ⌘⇧B to move focus into the column, and its
    /// tree is built inline in the render anyway.
    Always,
}

/// A panel's open and close, as the slide needs it.
///
/// Named fields rather than an argument list because every call site is deep
/// inside a render tree, where six positional values are unreadable.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Slide {
    /// The transition key and the element id. Stable for the panel's lifetime,
    /// and what [`progress`] reads from another element.
    pub(crate) name: &'static str,
    pub(crate) open: bool,
    /// The extent the panel takes when it is open — its width, or its height
    /// for [`Anchor::Bottom`], including its own gutters, because that is what
    /// the wrapper has to uncover.
    pub(crate) extent: Pixels,
    pub(crate) anchor: Anchor,
    pub(crate) mount: Mount,
}

/// The panel, sliding.
///
/// Returns `None` only for [`Mount::WhileOpen`] once the panel is closed and at
/// rest: that is what keeps a closed dock free. The registry keeps the closed
/// value, so the next open travels the full extent. `build` runs only when
/// something will be painted.
pub(crate) fn slide(
    slide: Slide,
    theme: &Theme,
    build: impl FnOnce() -> AnyElement,
) -> Option<AnyElement> {
    // `Theme` is `Copy`, and the animator outlives this call.
    let theme = *theme;
    let Slide {
        name,
        open,
        extent,
        anchor,
        mount,
    } = slide;
    let key = ElementId::Name(SharedString::from(name));
    let now = Instant::now();
    // The role, named once: `Layout` is the layout change `DESIGN.md` assigns to
    // a panel's width, and it is also what makes Reduce Motion a no-op here.
    let role = Motion::Layout;
    let target = if open { 1.0 } else { 0.0 };

    let (generation, in_flight, progress) = Transitions::update(key.clone(), |current| {
        let mut transition =
            current.unwrap_or_else(|| Transition::settled(target, now).easing(role.easing()));
        // Retarget on a change of *state*, not of duration. A panel that mounts
        // already at its target — open on launch — must not start a slide from
        // its own value, and retargeting it would do exactly that.
        if (transition.to() - target).abs() > f32::EPSILON {
            transition.retarget(target, role.duration(&theme), now);
        }
        let in_flight = !transition.is_done(now);
        let progress = transition.value_at(now);
        (transition, (transition.generation(), in_flight, progress))
    });

    // Closed and at rest: the panel costs nothing to leave out of the tree.
    if !open && !in_flight && mount == Mount::WhileOpen {
        return None;
    }

    let body = build();
    let wrapper = match anchor {
        // `justify_start` / `justify_end` decides the clipped side: the inner is
        // pushed past the wrapper's edge the eye is not watching.
        Anchor::Left => div()
            .flex_none()
            .h_full()
            .overflow_hidden()
            .flex()
            .justify_start(),
        Anchor::Right => div()
            .flex_none()
            .h_full()
            .overflow_hidden()
            .flex()
            .justify_end(),
        // The one vertical panel: the transcript above it and the window's
        // bottom edge below, so the strip stays flush with the bottom and is
        // uncovered from below.
        Anchor::Bottom => div()
            .flex_none()
            .w_full()
            .overflow_hidden()
            .flex()
            .flex_col()
            .justify_end(),
    }
    // A test selector (a no-op outside test builds): the animated axis is the
    // wrapper's and the panel inside never moves, so this is the only way to
    // measure a slide at all. The name is already the selector.
    .debug_selector(move || name.to_string())
    .child(body);

    // A settled panel draws its extent and mounts no animation: an `Animation`
    // of zero duration divides by zero inside gpui, and one with nothing to do
    // would ask for frames it does not need.
    if !in_flight || role.duration(&theme).is_zero() {
        return Some(sized(wrapper, extent, progress, anchor).into_any_element());
    }

    Some(
        sized(wrapper, extent, progress, anchor)
            .with_animation(
                ElementId::NamedInteger(SharedString::from(name), generation),
                Animation::new(Motion::Layout.duration(&theme))
                    .with_easing(Motion::Layout.easing()),
                move |element, _delta| {
                    // The registry owns the clock. The animator only asks for
                    // frames: reading the transition is what makes a reversal
                    // resume from the extent on screen rather than from the
                    // role's start.
                    let progress = Transitions::get(ElementId::Name(SharedString::from(name)))
                        .map(|transition| transition.value_at(Instant::now()))
                        .unwrap_or(target);
                    sized(element, extent, progress, anchor)
                },
            )
            .into_any_element(),
    )
}

/// The wrapper at a point on the slide: `extent` along the panel's own axis when
/// open, nothing when closed.
fn sized(element: Div, extent: Pixels, progress: f32, anchor: Anchor) -> Div {
    let size = extent * progress.clamp(0.0, 1.0);
    if anchor.vertical() {
        element.h(size)
    } else {
        element.w(size)
    }
}

/// The slide's progress, 0 (closed) to 1 (open).
///
/// An element that rides the panel's edge reads this at render time instead of
/// animating a second clock, so it cannot drift from the panel it is tracking.
/// It needs no animation of its own: the panel's slide asks for the frames it is
/// drawn on.
///
/// `fallback` answers before the panel has ever rendered, where the caller knows
/// the state but the registry has nothing stored yet.
pub(crate) fn progress(name: &'static str, fallback: f32) -> f32 {
    Transitions::get(ElementId::Name(SharedString::from(name)))
        .map(|transition| transition.value_at(Instant::now()))
        .unwrap_or(fallback)
}

/// Put a slide at its end state with nothing in flight.
///
/// For the one case where a panel must not move: the sessions sidebar snapping
/// open behind a full-width Review pane, so the session view never peeks
/// through the gap mid-slide. Settling before the panel renders leaves
/// [`slide`] nothing to retarget.
pub(crate) fn settle(name: &'static str, open: bool) {
    let now = Instant::now();
    let target = if open { 1.0 } else { 0.0 };
    Transitions::update(ElementId::Name(SharedString::from(name)), |_| {
        (
            Transition::settled(target, now).easing(Motion::Layout.easing()),
            (),
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::MotionPreference;
    use gpui::px;

    const EXTENT: Pixels = px(460.);
    /// A panel extent that fits inside the axis probe's box on either axis.
    const STRIP: Pixels = px(120.);

    fn dock(name: &'static str, open: bool) -> Slide {
        Slide {
            name,
            open,
            extent: EXTENT,
            anchor: Anchor::Right,
            mount: Mount::WhileOpen,
        }
    }

    fn painted(name: &'static str) -> Option<Transition> {
        Transitions::get(ElementId::Name(SharedString::from(name)))
    }

    fn draw(name: &'static str, theme: &Theme, open: bool) -> Option<AnyElement> {
        slide(dock(name, open), theme, || div().into_any_element())
    }

    /// A dock that is already open when it first renders is where it belongs:
    /// mounting must not play an entrance, and a closed one must not run a close
    /// nothing asked for.
    #[test]
    fn a_panel_mounts_at_its_own_state() {
        let theme = Theme::dark();
        assert!(draw("panel-mount-open", &theme, true).is_some());
        let opened = painted("panel-mount-open").expect("tracked");
        assert_eq!(opened.to(), 1.0);
        assert_eq!(opened.generation(), 0, "mounting must not start a slide");
        assert!(
            opened.is_done(Instant::now()),
            "an open-on-launch panel sits still"
        );

        assert!(
            draw("panel-mount-closed", &theme, false).is_none(),
            "a closed dock is not in the tree at all"
        );
        assert!(
            painted("panel-mount-closed")
                .expect("tracked")
                .is_done(Instant::now()),
            "and it is resting, not mid-close"
        );
    }

    /// A panel that must stay mounted (the sidebar, for its focus handle) is
    /// still a settled, zero-extent frame when it is closed.
    #[test]
    fn a_panel_that_mounts_always_stays_in_the_tree_when_closed() {
        let theme = Theme::dark();
        let slide_in_place = Slide {
            mount: Mount::Always,
            ..dock("panel-always", false)
        };
        assert!(
            slide(slide_in_place, &theme, || div().into_any_element()).is_some(),
            "a closed panel that must keep its focus handle stays rendered"
        );
        let closed = painted("panel-always").expect("tracked");
        assert!(closed.is_done(Instant::now()));
        assert_eq!(progress("panel-always", 1.0), 0.0);
    }

    /// The point of the primitive: a closing panel stays mounted so it has
    /// something to interpolate, and leaves only once it has arrived.
    #[test]
    fn a_closing_panel_stays_mounted_until_it_settles() {
        let theme = Theme::dark();
        assert!(draw("panel-close", &theme, true).is_some());
        assert!(
            draw("panel-close", &theme, false).is_some(),
            "the first frame of a close still paints the panel"
        );
        let closing = painted("panel-close").expect("tracked");
        assert_eq!(closing.to(), 0.0);
        assert_eq!(closing.generation(), 1, "the state changed, so the id must");
        assert!(!closing.is_done(Instant::now()), "it is on its way out");

        // The frames it asked for have run: now it is gone.
        settle("panel-close", false);
        assert!(draw("panel-close", &theme, false).is_none());
    }

    /// `DESIGN.md` requires a reversal to resume from the value on screen. The
    /// registry is seeded with a half-open panel to stand in for a close caught
    /// mid-slide, because a test cannot wait on the wall clock.
    #[test]
    fn a_reversal_resumes_from_the_extent_on_screen() {
        let theme = Theme::dark();
        let now = Instant::now();
        let key = ElementId::Name(SharedString::from("panel-reversal"));
        Transitions::update(key, |_| {
            let half_open = Transition::new(
                0.4,
                1.0,
                Motion::Layout.duration(&theme),
                Motion::Layout.easing(),
                now,
            );
            (half_open, ())
        });

        // Close is clicked while the open is still running.
        assert!(draw("panel-reversal", &theme, false).is_some());
        let reversed = painted("panel-reversal").expect("tracked");
        assert_eq!(reversed.to(), 0.0);
        assert!(
            (0.39..0.5).contains(&reversed.from()),
            "a reversal must start from the half-open extent on screen, not from \
             the full extent: {}",
            reversed.from()
        );
    }

    /// Settling is for the one panel that must not move — the sidebar snapping
    /// open behind a full-width Review pane.
    #[test]
    fn settling_leaves_nothing_to_animate() {
        let theme = Theme::dark();
        settle("panel-settled", true);
        assert!(draw("panel-settled", &theme, true).is_some());
        let settled = painted("panel-settled").expect("tracked");
        assert_eq!(
            settled.generation(),
            0,
            "settling then rendering is a no-op"
        );
        assert!(settled.is_done(Instant::now()));
        assert_eq!(progress("panel-settled", 0.0), 1.0);
    }

    /// Reduce Motion takes the new extent on the frame it changes: no animation
    /// is mounted, and a closed dock is simply out of the tree.
    #[test]
    fn reduced_motion_takes_the_end_state_at_once() {
        let mut theme = Theme::dark();
        theme.ui.motion = MotionPreference::Reduced;

        assert!(draw("panel-reduced", &theme, true).is_some());
        let opened = painted("panel-reduced").expect("tracked");
        assert_eq!(opened.to(), 1.0);
        assert!(
            opened.is_done(Instant::now()),
            "Reduce Motion must not leave the panel in flight"
        );
        assert_eq!(progress("panel-reduced", 0.0), 1.0);
        assert!(draw("panel-reduced", &theme, false).is_none());
    }

    /// A surface that has never rendered answers with the state its caller
    /// knows, so an edge element cannot be drawn in the wrong place on the
    /// first frame.
    #[test]
    fn an_untracked_slide_answers_with_its_fallback() {
        assert_eq!(progress("panel-never-drawn", 0.0), 0.0);
        assert_eq!(progress("panel-never-drawn", 1.0), 1.0);
    }

    /// The animated property is the wrapper's, along the panel's own axis: the
    /// panel inside keeps its extent and only gets uncovered. Laid out for real,
    /// because an axis is geometry rather than a value.
    #[gpui::test]
    fn a_panel_takes_its_extent_along_its_own_axis(cx: &mut gpui::TestAppContext) {
        use gpui::{point, size, AppContext as _};

        struct Probe {
            theme: Theme,
            anchor: Anchor,
        }

        impl gpui::Render for Probe {
            fn render(
                &mut self,
                _: &mut gpui::Window,
                _: &mut gpui::Context<Self>,
            ) -> impl IntoElement {
                // The real container: the chat column for a bottom strip, the
                // docked row for a side panel. The slide is the last item in
                // both, which is where its edge comes from — the app relies on
                // that too, so a wrapper without it would measure elsewhere.
                let root = div().w(px(400.)).h(px(300.)).flex();
                let root = if self.anchor.vertical() {
                    root.flex_col()
                } else {
                    root
                };
                root
                    // The space the panel takes from, as the transcript is for
                    // the terminal.
                    .child(div().flex_1())
                    .children(slide(
                        Slide {
                            name: match self.anchor {
                                Anchor::Bottom => "panel-axis-bottom",
                                _ => "panel-axis-right",
                            },
                            open: true,
                            extent: STRIP,
                            anchor: self.anchor,
                            // Mounted while closed so the closed frame can be
                            // measured too; `WhileOpen` paints nothing then.
                            mount: Mount::Always,
                        },
                        &self.theme,
                        || {
                            div()
                                .size_full()
                                .debug_selector(|| "panel-axis-inner".into())
                                .into_any_element()
                        },
                    ))
            }
        }

        let theme = Theme::dark();

        // The terminal: height, flush with the bottom of the column.
        let cx = cx.add_empty_window();
        cx.draw(point(px(0.), px(0.)), size(px(400.), px(300.)), |_, cx| {
            cx.new(|_| Probe {
                theme,
                anchor: Anchor::Bottom,
            })
        });
        let wrapper = cx
            .debug_bounds("panel-axis-bottom")
            .expect("the slide lays out");
        let inner = cx
            .debug_bounds("panel-axis-inner")
            .expect("the panel lays out");
        assert_eq!(wrapper.size.width, px(400.), "a bottom panel spans the row");
        assert_eq!(
            wrapper.size.height, STRIP,
            "its extent is the height it takes"
        );
        assert_eq!(wrapper.bottom(), px(300.), "flush with the window's edge");
        assert_eq!(
            (inner.bottom(), inner.size.height),
            (wrapper.bottom(), STRIP),
            "the panel itself never moves; the wrapper uncovers it"
        );

        // A dock: width, flush with the right edge.
        let cx = cx.add_empty_window();
        cx.draw(point(px(0.), px(0.)), size(px(400.), px(300.)), |_, cx| {
            cx.new(|_| Probe {
                theme,
                anchor: Anchor::Right,
            })
        });
        let wrapper = cx
            .debug_bounds("panel-axis-right")
            .expect("the slide lays out");
        let inner = cx
            .debug_bounds("panel-axis-inner")
            .expect("the panel lays out");
        assert_eq!(wrapper.size.height, px(300.), "a dock spans the column");
        assert_eq!(
            wrapper.size.width, STRIP,
            "its extent is the width it takes"
        );
        assert_eq!(wrapper.right(), px(400.), "flush with the window's edge");
        assert_eq!(
            (inner.right(), inner.size.width),
            (wrapper.right(), STRIP),
            "the panel itself never moves; the wrapper uncovers it"
        );
    }
}
