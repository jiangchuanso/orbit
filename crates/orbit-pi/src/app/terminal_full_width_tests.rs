//! The terminal's expand has to reach the app's layout: the panel flips its own
//! flag from its header, but only `OrbitApp::render` decides whether the shell
//! is a strip at the bottom of the chat or a page of its own.

use super::*;

/// Expanding from the panel's own header — not from an app method — is what the
/// user clicks, so the app has to hear about it through the observer and mount
/// the shell as the page on the next frame. The docks stay where they are: like
/// Git and Usage, a page is the column, not the window.
#[gpui::test]
fn expanding_the_terminal_from_its_own_header_fills_the_page(cx: &mut gpui::TestAppContext) {
    use crate::theme::{Theme, ThemeId};

    cx.update(|cx| cx.set_global(Theme::for_id(ThemeId::Orbit)));
    let cx = cx.add_empty_window();
    let app = cx.update(|_, cx| cx.new(OrbitApp::new));
    let viewport = cx.update(|window, _| window.viewport_size());

    cx.update(|window, cx| {
        app.update(cx, |app, cx| {
            // The dock is gated on the runtime probe; the test host is not
            // asked to have pi installed just to lay the window out.
            for dependency in &mut app.deps {
                dependency.installed = true;
            }
            app.sidepane.update(cx, |pane, cx| pane.toggle(cx));
            app.terminal_panel
                .update(cx, |panel, cx| panel.new_tab(window, cx));
        });
    });
    let _ = cx.draw(gpui::point(px(0.), px(0.)), viewport, |_, _| app.clone());
    let docked = cx
        .debug_bounds("terminal-panel")
        .expect("the shell is on screen");

    // Exactly what the header chip runs. Nothing here reaches the app directly:
    // the observer is the only path from the panel's flag to the shell's render.
    cx.update(|_, cx| {
        app.update(cx, |app, cx| {
            app.terminal_panel
                .update(cx, |panel, cx| panel.toggle_full_width(cx));
        });
    });
    let _ = cx.draw(gpui::point(px(0.), px(0.)), viewport, |_, _| app.clone());
    let expanded = cx
        .debug_bounds("terminal-panel")
        .expect("the shell is still on screen");

    assert!(
        expanded.size.height > docked.size.height,
        "the expanded shell owns the page: {:?} → {:?}",
        docked,
        expanded
    );
    assert!(
        expanded.size.height > viewport.height * 0.6,
        "a page, not a strip: {:?} in a {:?} window",
        expanded.size.height,
        viewport.height
    );

    // Back in the docked strip: the app has to follow that direction too.
    cx.update(|_, cx| {
        app.update(cx, |app, cx| {
            app.terminal_panel
                .update(cx, |panel, cx| panel.toggle_full_width(cx));
        });
    });
    let _ = cx.draw(gpui::point(px(0.), px(0.)), viewport, |_, _| app.clone());
    let restored = cx
        .debug_bounds("terminal-panel")
        .expect("the shell is still on screen");
    assert_eq!(
        restored.size.height, docked.size.height,
        "closing the page puts the same strip back"
    );
}

/// Closing the strip eases its own height away instead of dropping the row
/// between two frames: it stays mounted for `Motion::Layout`, flush with the
/// window's bottom edge, and the transcript above takes the height back on the
/// same curve. Nothing about the shell's own height moves, so reopening puts the
/// strip the reader dialled in back exactly where it was.
#[gpui::test]
fn closing_the_strip_slides_its_height_away(cx: &mut gpui::TestAppContext) {
    use crate::motion::panel::TERMINAL;
    use crate::theme::{Theme, ThemeId};

    cx.update(|cx| cx.set_global(Theme::for_id(ThemeId::Orbit)));
    let cx = cx.add_empty_window();
    let app = cx.update(|_, cx| cx.new(OrbitApp::new));
    let viewport = cx.update(|window, _| window.viewport_size());

    cx.update(|window, cx| {
        app.update(cx, |app, cx| {
            for dependency in &mut app.deps {
                dependency.installed = true;
            }
            app.terminal_panel
                .update(cx, |panel, cx| panel.new_tab(window, cx));
        });
    });
    let _ = cx.draw(gpui::point(px(0.), px(0.)), viewport, |_, _| app.clone());
    let open = cx
        .debug_bounds(TERMINAL)
        .expect("an open strip mounts its slide");
    let docked = cx
        .debug_bounds("terminal-panel")
        .expect("the shell is on screen");
    assert_eq!(
        open.size.height, docked.size.height,
        "the wrapper uncovers the strip at the shell's own height"
    );
    assert_eq!(
        open.bottom(),
        docked.bottom(),
        "the strip is flush with the bottom edge"
    );
    let height = cx.update(|_, cx| app.read(cx).terminal_panel.read(cx).height());
    assert_eq!(open.size.height, height);

    // ⌘J: the panel's own flag flips, and the row stays for the slide.
    cx.update(|window, cx| {
        app.update(cx, |app, cx| {
            app.terminal_panel
                .update(cx, |panel, cx| panel.toggle(window, cx));
        });
    });
    let _ = cx.draw(gpui::point(px(0.), px(0.)), viewport, |_, _| app.clone());
    let sliding = cx
        .debug_bounds(TERMINAL)
        .expect("the strip is still mounted on the first frame after the close");
    assert!(
        sliding.size.height > px(0.),
        "the row eases shut rather than vanishing: {sliding:?}"
    );
    assert!(
        cx.debug_bounds("terminal-panel").is_some(),
        "and it still paints the shell it is dismissing"
    );

    // Run the frames the slide asked for — this test has no clock of its own.
    // That the row then leaves the tree is `motion::panel`'s contract and is
    // asserted there, headless; `debug_bounds` reports the last frame the test
    // host swapped in, which does not follow a removed element here.
    crate::motion::panel::settle(TERMINAL, false);
    cx.update(|_, cx| app.update(cx, |_, cx| cx.notify()));
    let _ = cx.draw(gpui::point(px(0.), px(0.)), viewport, |_, _| app.clone());
    assert!(
        !cx.update(|_, cx| app.read(cx).terminal_panel.read(cx).is_open()),
        "the strip is closed"
    );
    assert_eq!(
        crate::motion::panel::progress(TERMINAL, 9.0),
        0.0,
        "and its slide has arrived at nothing"
    );

    // The shell keeps the height the reader dialled in, so reopening is the
    // same strip rather than a reset one.
    assert_eq!(
        cx.update(|_, cx| app.read(cx).terminal_panel.read(cx).height()),
        height,
        "closing does not forget the strip's height"
    );
}
