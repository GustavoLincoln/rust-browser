use crate::presentation::browser_shell_slint::controller::{
    BrowserShellSlintController, BrowserShellUiAction,
};
use crate::presentation::browser_shell_slint::AppWindow;
use slint::SharedString;
use std::cell::RefCell;
use std::rc::Rc;

pub struct BrowserShellSlintBridge {
    window: AppWindow,
    controller: Rc<RefCell<BrowserShellSlintController>>,
}

impl BrowserShellSlintBridge {
    pub fn new(
        controller: Rc<RefCell<BrowserShellSlintController>>,
    ) -> Result<Self, slint::PlatformError> {
        let window = AppWindow::new()?;
        let bridge = Self { window, controller };
        bridge.bind_callbacks();
        Ok(bridge)
    }

    pub fn window(&self) -> &AppWindow {
        &self.window
    }

    pub fn sync_from_controller(&self) {
        let view_model = self.controller.borrow().view_model().clone();
        self.window
            .set_window_title(SharedString::from(view_model.window_title));
        self.window
            .set_address_value(SharedString::from(view_model.toolbar.address_value));
        self.window
            .set_active_tab_title(SharedString::from(view_model.toolbar.active_tab_title));
        self.window
            .set_active_tab_url(SharedString::from(view_model.toolbar.active_tab_url));
        self.window
            .set_selected_profile(SharedString::from(view_model.options_dialog.selected_profile));
        self.window.set_active_settings_tab(SharedString::from(
            view_model.options_dialog.active_section,
        ));
        self.window
            .set_page_title(SharedString::from(view_model.options_dialog.page_title));
        self.window
            .set_current_url(SharedString::from(view_model.options_dialog.current_url));
        self.window
            .set_page_meta(SharedString::from(view_model.options_dialog.page_meta));
        self.window
            .set_page_summary(SharedString::from(view_model.options_dialog.page_summary));
        self.window
            .set_history_text(SharedString::from(view_model.options_dialog.history_text));
        self.window
            .set_preview_text(SharedString::from(view_model.options_dialog.preview_text));
        self.window
            .set_status_message(SharedString::from(view_model.status.message));
        self.window
            .set_options_open(view_model.options_dialog.is_open);
        self.window.set_is_loading(view_model.toolbar.is_loading);
    }

    fn bind_callbacks(&self) {
        let controller = Rc::clone(&self.controller);
        self.window.on_go_back(move || {
            controller
                .borrow_mut()
                .enqueue_action(BrowserShellUiAction::Back);
        });

        let controller = Rc::clone(&self.controller);
        self.window.on_go_forward(move || {
            controller
                .borrow_mut()
                .enqueue_action(BrowserShellUiAction::Forward);
        });

        let controller = Rc::clone(&self.controller);
        self.window.on_reload_page(move || {
            controller
                .borrow_mut()
                .enqueue_action(BrowserShellUiAction::Reload);
        });

        let controller = Rc::clone(&self.controller);
        self.window.on_open_options(move || {
            controller
                .borrow_mut()
                .enqueue_action(BrowserShellUiAction::ToggleOptions);
        });

        let controller = Rc::clone(&self.controller);
        self.window.on_close_options(move || {
            controller
                .borrow_mut()
                .enqueue_action(BrowserShellUiAction::CloseOptions);
        });

        let controller = Rc::clone(&self.controller);
        self.window.on_create_tab(move || {
            controller
                .borrow_mut()
                .enqueue_action(BrowserShellUiAction::NewTab);
        });

        let controller = Rc::clone(&self.controller);
        self.window.on_navigate(move |value| {
            controller
                .borrow_mut()
                .enqueue_action(BrowserShellUiAction::Navigate(value.to_string()));
        });

        let controller = Rc::clone(&self.controller);
        self.window.on_minimize_window(move || {
            controller
                .borrow_mut()
                .enqueue_action(BrowserShellUiAction::MinimizeWindow);
        });

        let controller = Rc::clone(&self.controller);
        self.window.on_maximize_window(move || {
            controller
                .borrow_mut()
                .enqueue_action(BrowserShellUiAction::MaximizeWindow);
        });

        let controller = Rc::clone(&self.controller);
        self.window.on_close_window(move || {
            controller
                .borrow_mut()
                .enqueue_action(BrowserShellUiAction::CloseWindow);
        });

        let controller = Rc::clone(&self.controller);
        self.window.on_select_profile(move |value| {
            controller
                .borrow_mut()
                .enqueue_action(BrowserShellUiAction::SelectProfile(value.to_string()));
        });

        let controller = Rc::clone(&self.controller);
        self.window.on_select_settings_tab(move |value| {
            controller
                .borrow_mut()
                .enqueue_action(BrowserShellUiAction::SelectSettingsTab(value.to_string()));
        });
    }
}
