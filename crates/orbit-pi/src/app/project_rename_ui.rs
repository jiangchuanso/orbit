//! Rename a project (#166): the project header menu's **Rename…** opens a
//! one-field dialog, and the name is stored as the project's display name in
//! `~/.orbit-pi/workspaces.json` beside its icon and tint
//! ([`WorkspaceMark::name`]). The folder on disk and pi's sessions are never
//! touched; clearing the field brings the folder's own name back.
use super::*;
use crate::theme::tokens::{input, modal, ButtonSize, DynamicSpacing, TextSize};
use crate::ui::dialog::modal_button;
use crate::ui::field::TextField;
use crate::ui::prelude::*;
use crate::ui::prelude::{input_label, modal_footer, modal_header, ButtonVariant, ModalWidthExt};

/// The open Rename dialog.
pub(crate) struct ProjectRename {
    /// The project's folder — the key its mark is stored under.
    pub(crate) cwd: PathBuf,
    /// The folder's own name, which an empty field restores.
    pub(crate) folder: String,
    pub(crate) input: Entity<ComposerInput>,
}

impl OrbitApp {
    /// The header menu's **Rename…**: open the dialog on the menu's project,
    /// seeded with the name it shows now.
    pub(super) fn on_workspace_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.workspace_menu.take() else {
            return;
        };
        let current = self
            .workspace_marks
            .get(&menu.cwd)
            .map(|mark| mark.display_name(&menu.label).to_string())
            .unwrap_or_else(|| menu.label.clone());
        let input = cx.new(|cx| {
            ComposerInput::new(cx)
                .with_element_id("project-rename-input")
                .with_key_context("Composer Picker")
                .with_max_lines(1)
                .with_wrap(false)
                .with_placeholder(menu.label.clone())
                .with_text(current)
        });
        let target = input.read(cx).focus_handle(cx);
        self.project_rename_focus.open(&target, window, cx);
        self.project_rename = Some(ProjectRename {
            cwd: menu.cwd,
            folder: menu.label,
            input,
        });
        cx.notify();
    }

    /// Store the typed name (or clear it) and close.
    pub(super) fn project_rename_save(&mut self, cx: &mut Context<Self>) {
        let Some(rename) = self.project_rename.as_ref() else {
            return;
        };
        // The field holds the preedit while an IME candidate window is open.
        if crate::composer::any_composing(cx, &[&rename.input]) {
            return;
        }
        let typed = rename.input.read(cx).text();
        let name = workspace_mark::custom_name(&typed, &rename.folder);
        let cwd = rename.cwd.clone();
        let mark = WorkspaceMark {
            name,
            ..self.workspace_marks.get(&cwd).cloned().unwrap_or_default()
        };
        self.set_workspace_mark(cwd, mark);
        self.project_rename = None;
        cx.notify();
    }

    pub(super) fn project_rename_cancel(&mut self, cx: &mut Context<Self>) {
        if self.project_rename.take().is_some() {
            cx.notify();
        }
    }

    /// The Rename dialog, or `None` when closed.
    pub(super) fn project_rename_layer(
        &self,
        theme: Theme,
        this: Entity<OrbitApp>,
        cx: &Context<Self>,
    ) -> Option<AnyElement> {
        let rename = self.project_rename.as_ref()?;
        let theme = theme.for_floating();
        let body = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(input::gap(&theme))
            .child(
                input_label(tr!("sidebar.project_name"), &theme)
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text_secondary),
            )
            .child(TextField::new(rename.input.clone(), theme.background).render(&theme))
            .child(
                div()
                    .text_size(TextSize::Small.px(&theme))
                    .text_color(theme.text_muted)
                    .whitespace_normal()
                    .child(tr!("sidebar.project_name_hint", folder = rename.folder.clone())),
            );

        let cancel = modal_button(
            "project-rename-cancel",
            tr!("sidebar.cancel"),
            ButtonVariant::Secondary,
            &self.modal_cancel_focus,
        )
        .size(ButtonSize::Large)
        .on_click(&theme, {
            let this = this.clone();
            move |_, _, cx| this.update(cx, |app, cx| app.project_rename_cancel(cx))
        });
        let save = modal_button(
            "project-rename-save",
            tr!("sidebar.rename_project_save"),
            ButtonVariant::Primary,
            &self.modal_confirm_focus,
        )
        .size(ButtonSize::Large)
        .on_click(&theme, {
            let this = this.clone();
            move |_, _, cx| this.update(cx, |app, cx| app.project_rename_save(cx))
        });

        let card = modal_card(&theme)
            .debug_selector(|| "project-rename-card".to_string())
            .modal_width(px(420.))
            .max_h(relative(1.))
            .key_context("Picker")
            .on_action(cx.listener(|app, _: &crate::PickerCancel, _, cx| {
                app.project_rename_cancel(cx)
            }))
            .on_action(cx.listener(|app, _: &crate::PickerConfirm, _, cx| {
                app.project_rename_save(cx)
            }))
            .child(modal_header(&theme, tr!("sidebar.rename_project_title"), None))
            .child(
                div()
                    .px(modal::section_padding_x(&theme, false))
                    .py(DynamicSpacing::Base16.px(&theme))
                    .child(body),
            )
            .child(modal_footer(&theme).child(cancel).child(save));

        Some(
            modal_scrim(&theme)
                .id("project-rename-layer")
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    this.update(cx, |app, cx| app.project_rename_cancel(cx))
                })
                .child(crate::motion::enter(
                    card,
                    "project-rename-card-in",
                    &theme,
                    crate::theme::tokens::Motion::Layout,
                    None,
                ))
                .into_any_element(),
        )
    }
}
