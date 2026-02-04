use crate::device::io;
use crate::device::types::{FidoDeviceInfo, FullDeviceStatus, StoredCredential};
use crate::ui::components::{card::Card, page_view::PageView};
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{
    ActiveTheme, Disableable, Icon, Sizable, StyledExt, Theme, WindowExt, badge::Badge, h_flex,
    input::Input, input::InputState, v_flex,
};

pub struct PasskeysView {
    device_status: Option<FullDeviceStatus>,
    fido_info: Option<FidoDeviceInfo>,
    credentials: Vec<StoredCredential>,
    unlocked: bool,
    cached_pin: Option<String>,
    loading: bool,

    _task: Option<Task<()>>,
    active_modal: Option<ActiveModal>,
}

pub enum ActiveModal {
    Unlock(Entity<InputState>),
    Delete {
        cred: StoredCredential,
        pin: String,
    },
    ChangePin,
    MinPin,
}

pub enum PasskeysEvent {
    Notification(String),
    CloseDialog,
}

impl EventEmitter<PasskeysEvent> for PasskeysView {}

impl PasskeysView {
    pub fn new(
        _window: &mut Window,
        _cx: &mut Context<Self>,
        device_status: Option<FullDeviceStatus>,
        fido_info: Option<FidoDeviceInfo>,
    ) -> Self {
        Self {
            device_status,
            fido_info,
            credentials: Vec::new(),
            unlocked: false,
            cached_pin: None,
            loading: false,
            _task: None,
            active_modal: None,
        }
    }

    pub fn update_device_status(
        &mut self,
        status: Option<FullDeviceStatus>,
        fido_info: Option<FidoDeviceInfo>,
        cx: &mut Context<Self>,
    ) {
        if self.device_status == status && self.fido_info == fido_info {
            return;
        }
        self.device_status = status;
        self.fido_info = fido_info;
        cx.notify();
    }

    fn unlock_storage(&mut self, pin: String, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();

        let entity = cx.entity().downgrade();

        cx.spawn(async move |_, cx| {
            let result = io::get_credentials(pin.clone());

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(creds) => {
                        this.unlocked = true;
                        this.cached_pin = Some(pin);
                        this.credentials = creds;
                        cx.emit(PasskeysEvent::CloseDialog);
                    }
                    Err(e) => {
                        let msg = format!("Failed to unlock: {}", e);
                        cx.emit(PasskeysEvent::Notification(msg));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn lock_storage(&mut self, cx: &mut Context<Self>) {
        self.unlocked = false;
        self.cached_pin = None;
        self.credentials.clear();
        cx.notify();
    }

    fn execute_delete(
        &mut self,
        credential_id: String,
        pin: String,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();

        let entity = cx.entity().downgrade();

        self._task = Some(cx.spawn(async move |_, cx| {
            let result = io::delete_credential(pin.clone(), credential_id);

            let _ = entity.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        // Refresh credentials
                        let _ = this.refresh_credentials(pin, cx);
                        cx.emit(PasskeysEvent::CloseDialog);
                        cx.emit(PasskeysEvent::Notification("Credential deleted".to_string()));
                    }
                    Err(e) => {
                        this.loading = false;
                        let msg = format!("Error deleting: {}", e);
                        cx.emit(PasskeysEvent::Notification(msg));
                        cx.notify();
                    }
                }
            });
        }));
    }

    fn refresh_credentials(&mut self, pin: String, cx: &mut Context<Self>) -> Task<()> {
        let entity = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let result = io::get_credentials(pin);
            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                if let Ok(creds) = result {
                    this.credentials = creds;
                }
                cx.notify();
            });
        })
    }

    fn open_unlock_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pin_input = cx.new(|cx| InputState::new(window, cx).placeholder("Enter FIDO PIN"));
        self.active_modal = Some(ActiveModal::Unlock(pin_input));
        cx.notify();
    }

    fn open_delete_dialog(
        &mut self,
        cred: &StoredCredential,
        pin: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_modal = Some(ActiveModal::Delete {
            cred: cred.clone(),
            pin,
        });
        cx.notify();
    }

    fn open_change_pin_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // TODO: Implement Change PIN Dialog
        // Placeholder for now
        window.open_dialog(cx, |dialog, _, _| {
            dialog
                .title("Not Implemented")
                .child("Change PIN dialog coming soon")
        });
    }

    fn render_no_device(&self, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .h_64()
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .child(
                div()
                    .text_color(theme.muted_foreground)
                    .child("Connect your pico-key to manage passkeys."),
            )
            .into_any_element()
    }

    fn render_not_supported(&self, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .h_64()
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .child(
                div()
                    .text_color(theme.muted_foreground)
                    .child("FIDO Passkeys are not supported on this device."),
            )
            .into_any_element()
    }

    fn render_pin_management(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let status_row = self.render_pin_status_row(cx).into_any_element();
        let min_len_row = self.render_min_pin_length_row(cx).into_any_element();

        Card::new()
            .title("PIN Management")
            .icon(Icon::default().path("icons/key.svg"))
            .description("Configure FIDO2 PIN security")
            .child(
                v_flex()
                    .gap_4()
                    .child(status_row)
                    .child(min_len_row),
            )
    }

    fn render_pin_status_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let pin_set = self
            .fido_info
            .as_ref()
            .and_then(|f| f.options.get("clientPin").copied())
            .unwrap_or(false);

        let listener = cx.listener(|this, _, window, cx| {
            this.open_change_pin_dialog(window, cx);
        });

        let theme = cx.theme();

        div()
            .flex()
            .items_center()
            .justify_between()
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                v_flex()
                    .child(div().font_medium().child("Current PIN Status"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(if pin_set {
                                "PIN is set"
                            } else {
                                "No PIN configured"
                            }),
                    ),
            )
            .child(
                Button::new("change-pin-btn")
                    .outline()
                    .child(if pin_set { "Change PIN" } else { "Set PIN" })
                    .on_click(listener),
            )
    }

    fn render_min_pin_length_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let min_len = self
            .fido_info
            .as_ref()
            .map(|f| f.min_pin_length)
            .unwrap_or(4);
        let pin_set = self
            .fido_info
            .as_ref()
            .and_then(|f| f.options.get("clientPin").copied())
            .unwrap_or(false);

        let theme = cx.theme();

        div()
            .flex()
            .items_center()
            .justify_between()
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                v_flex()
                    .child(div().font_medium().child("Minimum PIN Length"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!("Current: {} characters", min_len)),
                    ),
            )
            .child(
                Button::new("update-pin-len-btn")
                    .outline()
                    .disabled(!pin_set)
                    .child("Update Minimum Length"),
            )
    }

    fn render_stored_passkeys(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.unlocked {
            self.render_locked_state(cx).into_any_element()
        } else {
            self.render_unlocked_state(cx).into_any_element()
        }
    }

    fn render_locked_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let listener = cx.listener(|this, _, window, cx| {
            this.open_unlock_dialog(window, cx);
        });
        let theme = cx.theme();

        Card::new()
            .title("Stored Passkeys")
            .icon(Icon::default().path("icons/key-round.svg"))
            .description("View and manage your resident credentials")
            .child(
                v_flex()
                    .items_center()
                    .justify_center()
                    .gap_4()
                    .py_8()
                    .child(
                        div().rounded_full().bg(theme.muted).p_4().child(
                            Icon::default()
                                .path("icons/shield.svg")
                                .size_8()
                                .text_color(theme.muted_foreground),
                        ),
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .child("Authentication Required"),
                    )
                    .child(
                        div()
                            .text_color(theme.muted_foreground)
                            .text_sm()
                            .child(
                                "Unlock your device to view and manage passkeys.",
                            ),
                    )
                    .child(
                        Button::new("unlock-btn")
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(Icon::default().path("icons/lock-open.svg"))
                                    .child("Unlock Storage"),
                            )
                            .on_click(listener),
                    ),
            )
    }

    fn render_unlocked_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let creds_len = self.credentials.len();
        let lock_listener = cx.listener(|this, _, _, cx| {
            this.lock_storage(cx);
        });

        // Prepare credential cards before getting theme
        // We need to iterate self.credentials
        // render_credential_card needs cx for listeners.
        // It also needs theme.
        // So render_credential_card must adhere to the pattern: create listeners, then get theme.
        
        // This is tricky for map.
        // We can create a list of listeners first?
        // Or simply `render_credential_card` should take `cx` and handle it.
        // Yes, `render_credential_card(&self, cred, cx)`.
        
        let mut cards = Vec::new();
        for cred in &self.credentials {
            cards.push(self.render_credential_card(cred, cx).into_any_element());
        }

        let theme = cx.theme();

        Card::new()
            .title("Stored Passkeys")
            .icon(Icon::default().path("icons/key-round.svg"))
            .description("View and manage your resident credentials")
            .child(
                v_flex()
                    .gap_6()
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                h_flex()
                                    .gap_4()
                                    .items_center()
                                    .child(
                                        Badge::new()
                                            .child(
                                                h_flex()
                                                    .gap_1()
                                                    .items_center()
                                                    .child(
                                                        Icon::default()
                                                            .path("icons/lock-open.svg")
                                                            .size_3p5(),
                                                    )
                                                    .child("Unlocked"),
                                            )
                                            .color(gpui::green()),
                                    )
                                    .child(div().w_px().h_4().bg(theme.border))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child(format!("{} credentials stored", creds_len)),
                                    ),
                            )
                            .child(
                                Button::new("lock-storage-btn")
                                    .outline()
                                    .small()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(Icon::default().path("icons/lock.svg").size_3p5())
                                            .child("Lock Storage"),
                                    )
                                    .on_click(lock_listener),
                            ),
                    )
                    .child(if self.credentials.is_empty() {
                        self.render_empty_credentials_with_theme(theme).into_any_element()
                    } else {
                        div()
                            .grid()
                            .grid_cols(3)
                            .gap_4()
                            .children(cards)
                            .into_any_element()
                    }),
            )
    }
    
    // Rework layout of render_empty_credentials to take theme, assuming call site handles it.
    fn render_empty_credentials_with_theme(&self, theme: &Theme) -> impl IntoElement {
        v_flex()
            .items_center()
            .justify_center()
            .py_12()
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .gap_4()
            .child(
                div()
                    .rounded_full()
                    .bg(theme.muted)
                    .p_4()
                    .child(
                        Icon::default()
                            .path("icons/key-round.svg")
                            .size_8()
                            .text_color(theme.muted_foreground),
                    ),
            )
            .child(div().text_lg().font_semibold().child("No Passkeys Found"))
            .child(
                div()
                    .text_color(theme.muted_foreground)
                    .text_sm()
                    .text_center()
                    .max_w(px(384.0))
                    .child("This device doesn't have any resident credentials stored yet. Create passkeys on websites to see them here."),
            )
    }

    fn render_modal(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(modal) = &self.active_modal {
             let theme = cx.theme();
             let content = match modal {
                ActiveModal::Unlock(pin_input) => {
                     let pin_input_clone = pin_input.clone();
                     v_flex()
                        .gap_4()
                        .child("Enter your device PIN to view saved passkeys")
                        .child(Input::new(pin_input))
                        .child(
                            h_flex()
                                .gap_2()
                                .justify_end()
                                .child(
                                    Button::new("cancel-unlock")
                                        .label("Cancel")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.active_modal = None;
                                            cx.notify();
                                        }))
                                )
                                .child(
                                    Button::new("confirm-unlock")
                                        .primary()
                                        .label("Unlock")
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            let pin = pin_input_clone.read(cx).text().to_string();
                                            if !pin.is_empty() {
                                                this.unlock_storage(pin, cx);
                                                this.active_modal = None; // Manual close on success
                                                cx.notify();
                                            }
                                        }))
                                )
                        )
                },
                ActiveModal::Delete { cred, pin } => {
                     let cred_id = cred.credential_id.clone();
                     let pin_str = pin.clone();
                     let name = cred.rp_id.clone();
                     
                     v_flex()
                        .gap_4()
                        .child(format!("Are you sure you want to delete the passkey for {}?", name))
                        .child(
                            h_flex()
                                .gap_2()
                                .justify_end()
                                .child(
                                    Button::new("cancel-del")
                                        .label("Cancel")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.active_modal = None;
                                            cx.notify();
                                        }))
                                )
                                .child(
                                    Button::new("confirm-del")
                                        .danger()
                                        .label("Delete")
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.execute_delete(cred_id.clone(), pin_str.clone(), cx);
                                            this.active_modal = None;
                                            cx.notify();
                                        }))
                                )
                        )
                }
                _ => div().child("Not Implemented"),
             };

             div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(gpui::black().opacity(0.5))
                .child(
                    div()
                        .w(px(400.0))
                        .bg(theme.background)
                        .border_1()
                        .border_color(theme.border)
                        .rounded_lg()
                        .shadow_lg()
                        .p_6()
                        .child(content)
                )
                .into_any_element()
        } else {
             div().into_any_element()
        }
    }

    fn render_credential_card(
        &self,
        cred: &StoredCredential,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let cred_clone = cred.clone();
        
        let delete_listener = cx.listener(move |this, _, window, cx| {
             this.open_ask_delete_pin(cred_clone.clone(), window, cx);
        });

        let theme = cx.theme();

        div()
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .p_4()
            .hover(|s| s.bg(theme.accent).border_color(theme.primary))
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(
                                div()
                                    .size_10()
                                    .rounded_md()
                                    .bg(theme.secondary)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        Icon::default()
                                            .path("icons/key-round.svg")
                                            .text_color(theme.primary)
                                            .size_5(),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .child(div().font_semibold().child(cred.rp_name.clone()))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child(cred.user_name.clone()),
                                    ),
                            ),
                    )
                    .child(
                        Button::new("delete-cred-btn")
                            .ghost()
                            .small()
                            .child(
                                Icon::default()
                                    .path("icons/trash-2.svg")
                                    .size_4()
                                    .text_color(theme.muted_foreground),
                            )
                            .on_click(delete_listener),
                    ),
            )
    }

    fn open_ask_delete_pin(
        &mut self,
        cred: StoredCredential,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(pin) = &self.cached_pin {
            self.open_delete_dialog(&cred, pin.clone(), window, cx);
        } else {
            window.push_notification("Session expired, please unlock again.", cx);
            self.lock_storage(cx);
        }
    }
}

impl Render for PasskeysView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let device_connected = self.device_status.is_some();
        if !device_connected {
            // Static render, need theme.
            // Since we don't need listeners here, we can just get theme.
            let theme = cx.theme(); // borrow cx
            return 
                PageView::build(
                    "Passkeys",
                     "Manage your security PIN and the FIDO credentials (passkeys) stored on your device.",
                    self.render_no_device(theme).into_any_element(),
                    theme
                ).into_any_element();
        }

        let has_fido = self
            .device_status
            .as_ref()
            .map(|s| s.method == crate::device::types::DeviceMethod::Fido)
            .unwrap_or(false)
            || self.fido_info.is_some();
            
        if !has_fido {
             let theme = cx.theme();
             return PageView::build(
                    "Passkeys",
                     "Manage your security PIN and the FIDO credentials (passkeys) stored on your device.",
                    self.render_not_supported(theme).into_any_element(),
                    theme
                ).into_any_element();
        }

        // Main view
        // 1. Render content (calls helpers which access cx for listeners and theme)
        let content = v_flex()
             .gap_6()
             .child(self.render_pin_management(cx))
             .child(self.render_stored_passkeys(cx));
             
        // 2. Get theme for PageView wrapper
        let theme = cx.theme();
        
        div()
            .size_full()
            .relative() // Critical for absolute modal
            .child(
                PageView::build(
                    "Passkeys",
                    "Manage your security PIN and the FIDO credentials (passkeys) stored on your device.",
                    content.into_any_element(),
                    theme,
                )
            )
            .child(self.render_modal(cx))
            .into_any_element()
    }
}
