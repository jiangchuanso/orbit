//! Settings → Agent → Check spelling.
//!
//! Two promises are covered here: the switch exists only where the platform can
//! actually mark words, and the composer's gate follows the preference rather
//! than a value captured when the field was built.

use super::*;
use crate::theme::{Theme, ThemeId};

fn settings_app(cx: &mut gpui::TestAppContext) -> Entity<OrbitApp> {
    let app = cx.update(|cx| {
        cx.set_global(Theme::for_id(ThemeId::Orbit));
        cx.new(OrbitApp::new)
    });
    cx.update(|cx| {
        app.update(cx, |app, cx| {
            for dependency in &mut app.deps {
                dependency.installed = true;
            }
            app.setup_open = false;
            app.settings_open = true;
            app.set_settings_section(SettingsSection::Agent, cx);
        });
    });
    app
}

fn draw(cx: &mut gpui::VisualTestContext, app: &Entity<OrbitApp>) {
    let _ = cx.draw(
        point(px(0.), px(0.)),
        gpui::size(px(1100.), px(760.)),
        |_, _| app.clone(),
    );
}

/// `NSSpellChecker` is AppKit's, so the row appears on macOS and nowhere else.
/// A switch that could not mark anything would be a lie about the feature.
#[gpui::test]
fn the_switch_exists_only_where_the_checker_does(cx: &mut gpui::TestAppContext) {
    let app = settings_app(cx);
    let cx = cx.add_empty_window();
    draw(cx, &app);
    let offered = cx.debug_bounds("spell-check-toggle").is_some();
    assert_eq!(
        offered,
        cfg!(target_os = "macos"),
        "the spelling switch follows the platform's checker"
    );
}

/// The composer reads the preference per render, so the gate follows the theme
/// rather than a flag frozen at construction.
#[gpui::test]
fn the_gate_follows_the_preference(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        let mut theme = Theme::for_id(ThemeId::Orbit);
        theme.ui.spell_check = true;
        cx.set_global(theme);
        // On by preference, but the platform still has to be able to check.
        assert_eq!(
            crate::spellcheck::enabled(cx),
            cfg!(target_os = "macos"),
            "the preference cannot conjure a checker the platform lacks"
        );

        let mut theme = Theme::for_id(ThemeId::Orbit);
        theme.ui.spell_check = false;
        cx.set_global(theme);
        assert!(
            !crate::spellcheck::enabled(cx),
            "switching it off closes the gate on every platform"
        );
    });
}
