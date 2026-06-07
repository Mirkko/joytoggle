mod dbus;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dbus::JoyToggleDaemonProxyBlocking;
use gpui::{
    div, ease_in_out, hsla, prelude::*, px, size, white, Animation, AnimationExt, App, Application,
    Bounds, Context, FontWeight, MouseButton, SharedString, Window, WindowBounds, WindowOptions,
};
use joytoggle_core::{load_hidden, load_state, save_hidden, Device, DeviceType, InterfaceId};

// ── D-Bus helpers (zbus blocking-api, no tokio needed) ───────────────────────

fn fetch_devices() -> Vec<Device> {
    (|| -> anyhow::Result<Vec<Device>> {
        let conn = zbus::blocking::Connection::system()?;
        let proxy = JoyToggleDaemonProxyBlocking::new(&conn)?;
        Ok(serde_json::from_str(&proxy.list_devices()?)?)
    })()
    .unwrap_or_default()
}

fn call_toggle(iface_id: &str, enable: bool) -> bool {
    (|| -> anyhow::Result<()> {
        let conn = zbus::blocking::Connection::system()?;
        let proxy = JoyToggleDaemonProxyBlocking::new(&conn)?;
        if enable {
            proxy.enable_device(iface_id)?
        } else {
            proxy.disable_device(iface_id)?
        }
        Ok(())
    })()
    .is_ok()
}

// ── App state ─────────────────────────────────────────────────────────────────

struct DeviceItem {
    device: Device,
    enabled: bool,
    hidden: bool,
}

struct JoyToggleWindow {
    devices: Vec<DeviceItem>,
    manually_hidden: HashSet<String>, // iface IDs manually hidden by user
    show_hidden: bool,                // hidden section expanded/collapsed
    daemon_error: Option<String>,
    pending_refresh: Arc<Mutex<Option<Vec<Device>>>>,
}

impl JoyToggleWindow {
    fn new(cx: &mut Context<Self>) -> Self {
        // Load persisted hidden list and saved device state
        let manually_hidden: HashSet<String> = load_hidden();
        let saved_state = load_state();

        let devs = fetch_devices();
        let error = if devs.is_empty() {
            Some("joytoggle-daemon not reachable — showing mock data".to_owned())
        } else {
            None
        };
        let devices = if devs.is_empty() {
            mock_devices()
        } else {
            devs
        };

        let pending = Arc::new(Mutex::new(None::<Vec<Device>>));
        let bg = pending.clone();

        // Background thread: re-fetch every 2s
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(2));
            *bg.lock().unwrap() = Some(fetch_devices());
        });

        // GPUI timer: drain pending refresh into view state
        cx.spawn(async move |weak, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(750))
                .await;
            weak.update(cx, |view, cx| {
                let mut guard = view.pending_refresh.lock().unwrap();
                if let Some(fresh) = guard.take() {
                    if !fresh.is_empty() {
                        view.daemon_error = None;
                    }
                    let changed = fresh.len() != view.devices.len()
                        || fresh.iter().zip(view.devices.iter()).any(|(f, d)| {
                            f.iface_id().as_str() != d.device.iface_id().as_str()
                                || f.enabled != d.enabled
                        });
                    if changed {
                        let hidden = &view.manually_hidden;
                        view.devices = fresh
                            .into_iter()
                            .map(|d| {
                                let e = d.enabled;
                                let h = d.autohide || hidden.contains(d.iface_id().as_str());
                                DeviceItem {
                                    device: d,
                                    enabled: e,
                                    hidden: h,
                                }
                            })
                            .collect();
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();

        Self {
            devices: devices
                .into_iter()
                .map(|d| {
                    let iface = d.iface_id().as_str().to_owned();
                    // Saved state overrides live state when daemon not running
                    let enabled = saved_state
                        .get(&joytoggle_core::InterfaceId::from(iface.as_str()))
                        .copied()
                        .unwrap_or(d.enabled);
                    let hidden = d.autohide || manually_hidden.contains(&iface);
                    DeviceItem {
                        device: d,
                        enabled,
                        hidden,
                    }
                })
                .collect(),
            manually_hidden,
            show_hidden: false,
            daemon_error: error,
            pending_refresh: pending,
        }
    }

    fn visible_devices(&self) -> Vec<(usize, &DeviceItem)> {
        self.devices
            .iter()
            .enumerate()
            .filter(|(_, d)| !d.hidden)
            .collect()
    }

    fn hidden_devices(&self) -> Vec<(usize, &DeviceItem)> {
        self.devices
            .iter()
            .enumerate()
            .filter(|(_, d)| d.hidden)
            .collect()
    }

    fn active_count(&self) -> usize {
        self.visible_devices()
            .iter()
            .filter(|(_, d)| d.enabled)
            .count()
    }

    fn refresh_now(&mut self) {
        let fresh = fetch_devices();
        if !fresh.is_empty() {
            self.daemon_error = None;
            let hidden = &self.manually_hidden;
            self.devices = fresh
                .into_iter()
                .map(|d| {
                    let e = d.enabled;
                    let h = d.autohide || hidden.contains(d.iface_id().as_str());
                    DeviceItem {
                        device: d,
                        enabled: e,
                        hidden: h,
                    }
                })
                .collect();
        }
    }
}

// ── Mock fallback ─────────────────────────────────────────────────────────────

fn mock_devices() -> Vec<Device> {
    vec![
        Device {
            event: "event0".into(),
            dev_path: "/dev/input/event0".into(),
            name: "VIRPIL VPC Stick WarBRD-D (mock)".into(),
            device_type: DeviceType::Joystick,
            autohide: false,
            usb_path: Some("/sys/bus/usb/1-11.4.1:1.0".into()),
            interface_id: Some(InterfaceId::from("1-11.4.1:1.0")),
            vendor_id: None,
            product_id: None,
            enabled: true,
        },
        Device {
            event: "event1".into(),
            dev_path: "/dev/input/event1".into(),
            name: "VIRPIL VPC VMAX Throttle (mock)".into(),
            device_type: DeviceType::Throttle,
            autohide: false,
            usb_path: Some("/sys/bus/usb/1-11.4.3:1.0".into()),
            interface_id: Some(InterfaceId::from("1-11.4.3:1.0")),
            vendor_id: None,
            product_id: None,
            enabled: true,
        },
        Device {
            event: "event2".into(),
            dev_path: "/dev/input/event2".into(),
            name: "Glorious GMMK Pro ISO Consumer Control (mock)".into(),
            device_type: DeviceType::Gamepad,
            autohide: true,
            usb_path: Some("/sys/bus/usb/1-5:1.1".into()),
            interface_id: Some(InterfaceId::from("1-5:1.1")),
            vendor_id: None,
            product_id: None,
            enabled: true,
        },
    ]
}

// ── Rendering helpers ─────────────────────────────────────────────────────────

fn device_icon(t: &DeviceType) -> &'static str {
    match t {
        DeviceType::Joystick => "◈",
        DeviceType::Throttle => "▶",
        DeviceType::RudderPedals => "↕",
        DeviceType::Gamepad => "⊕",
        DeviceType::SteeringWheel => "◎",
        DeviceType::Unknown => "?",
    }
}

fn type_color(t: &DeviceType) -> gpui::Hsla {
    match t {
        DeviceType::Joystick => hsla(0.55, 0.7, 0.6, 1.0),
        DeviceType::Throttle => hsla(0.08, 0.7, 0.6, 1.0),
        DeviceType::RudderPedals => hsla(0.75, 0.6, 0.6, 1.0),
        DeviceType::Gamepad => hsla(0.33, 0.6, 0.55, 1.0),
        DeviceType::SteeringWheel => hsla(0.15, 0.65, 0.6, 1.0),
        DeviceType::Unknown => hsla(0.0, 0.0, 0.5, 1.0),
    }
}

impl Render for JoyToggleWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.active_count();
        let total = self.visible_devices().len();
        let error_banner = self.daemon_error.clone();
        let n_hidden = self.hidden_devices().len();
        let show_hidden = self.show_hidden;

        // ── Visible device rows ───────────────────────────────────────────────
        let vis_rows: Vec<_> = self
            .visible_devices()
            .into_iter()
            .map(|(i, item)| {
                let enabled = item.enabled;
                let name = item.device.name.clone();
                let dtype = item.device.device_type.clone();
                let iface_id = item.device.iface_id().as_str().to_owned();
                let bg = if enabled {
                    hsla(0.0, 0.0, 0.13, 1.0)
                } else {
                    hsla(0.0, 0.0, 0.09, 1.0)
                };
                let name_color = if enabled {
                    hsla(0.0, 0.0, 0.92, 1.0)
                } else {
                    hsla(0.0, 0.0, 0.4, 1.0)
                };
                let toggle_track = if enabled {
                    hsla(0.38, 0.6, 0.38, 1.0)
                } else {
                    hsla(0.0, 0.0, 0.22, 1.0)
                };
                let iface_disp = iface_id.clone();

                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py(px(10.0))
                    .mb(px(2.0))
                    .rounded_lg()
                    .bg(bg)
                    .child(
                        div()
                            .w_6()
                            .h_6()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(type_color(&dtype))
                            .child(device_icon(&dtype)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .child(
                                div()
                                    .text_color(name_color)
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(name.clone()),
                            )
                            .child(
                                div().text_color(hsla(0.0, 0.0, 0.35, 1.0)).text_xs().child(
                                    SharedString::from(format!("{} — {}", dtype, iface_disp)),
                                ),
                            ),
                    )
                    .child(
                        // Hide button
                        div()
                            .px_2()
                            .py(px(3.0))
                            .rounded_md()
                            .mr_2()
                            .bg(hsla(0.0, 0.0, 0.18, 1.0))
                            .text_xs()
                            .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.manually_hidden.insert(iface_id.clone());
                                    if let Some(d) = this.devices.get_mut(i) {
                                        d.hidden = true;
                                    }
                                    let _ = save_hidden(&this.manually_hidden);
                                    cx.notify();
                                }),
                            )
                            .child("hide"),
                    )
                    .child({
                        // Toggle switch — thumb slides 20px (44 - 2*3padding - 18thumb)
                        let anim_id = SharedString::from(format!("toggle-{i}-{enabled}"));
                        div()
                            .w(px(44.0))
                            .h(px(24.0))
                            .rounded_full()
                            .bg(toggle_track)
                            .flex()
                            .items_center()
                            .px(px(3.0))
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    let want_on = !this.devices[i].enabled;
                                    let iface =
                                        this.devices[i].device.iface_id().as_str().to_owned();
                                    if call_toggle(&iface, want_on) {
                                        this.devices[i].enabled = want_on;
                                    }
                                    cx.notify();
                                }),
                            )
                            .child(
                                div()
                                    .w(px(18.0))
                                    .h(px(18.0))
                                    .rounded_full()
                                    .bg(white())
                                    .with_animation(
                                        anim_id,
                                        Animation::new(Duration::from_millis(150))
                                            .with_easing(ease_in_out),
                                        move |thumb, delta| {
                                            let pos = if enabled {
                                                delta * 20.0
                                            } else {
                                                (1.0 - delta) * 20.0
                                            };
                                            thumb.ml(px(pos))
                                        },
                                    ),
                            )
                    })
            })
            .collect();

        // ── Hidden devices section ────────────────────────────────────────────
        let hidden_rows: Vec<_> = if show_hidden {
            self.hidden_devices()
                .into_iter()
                .map(|(i, item)| {
                    let name = item.device.name.clone();
                    let iface_id = item.device.iface_id().as_str().to_owned();
                    let is_auto = item.device.autohide;

                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .px_4()
                        .py(px(8.0))
                        .mb(px(2.0))
                        .rounded_lg()
                        .bg(hsla(0.0, 0.0, 0.08, 1.0))
                        .child(
                            div()
                                .text_color(hsla(0.0, 0.0, 0.35, 1.0))
                                .text_xs()
                                .child("⊘"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(hsla(0.0, 0.0, 0.35, 1.0))
                                        .child(name),
                                )
                                .child(
                                    div().text_xs().text_color(hsla(0.0, 0.0, 0.25, 1.0)).child(
                                        if is_auto {
                                            "auto-hidden"
                                        } else {
                                            "hidden by user"
                                        },
                                    ),
                                ),
                        )
                        .child(
                            div()
                                .px_2()
                                .py(px(3.0))
                                .rounded_md()
                                .bg(hsla(0.0, 0.0, 0.18, 1.0))
                                .text_xs()
                                .text_color(hsla(0.0, 0.0, 0.6, 1.0))
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, _, _, cx| {
                                        this.manually_hidden.remove(&iface_id);
                                        if let Some(d) = this.devices.get_mut(i) {
                                            d.hidden = false;
                                        }
                                        let _ = save_hidden(&this.manually_hidden);
                                        cx.notify();
                                    }),
                                )
                                .child("unhide"),
                        )
                })
                .collect()
        } else {
            vec![]
        };

        div().flex().flex_col().w_full().h_full().bg(hsla(0.0,0.0,0.07,1.0))
            // Error banner
            .when_some(error_banner, |el, msg| {
                el.child(
                    div().px_4().py(px(6.0)).bg(hsla(0.08,0.55,0.22,1.0))
                        .text_xs().text_color(hsla(0.1,0.9,0.8,1.0))
                        .child(SharedString::from(format!("⚠ {msg}"))),
                )
            })
            // Header
            .child(
                div().flex().flex_row().items_center().justify_between()
                    .px_5().py_4().border_b_1().border_color(hsla(0.0,0.0,0.15,1.0))
                    .child(
                        div().flex().flex_col()
                            .child(div().text_base().font_weight(FontWeight::SEMIBOLD)
                                .text_color(white()).child("JoyToggle"))
                            .child(div().text_xs().text_color(hsla(0.0,0.0,0.5,1.0))
                                .child(SharedString::from(format!("{active} / {total} active")))),
                    )
                    .child(
                        div().flex().gap_2()
                            .child(
                                div().px_3().py_1().rounded_md().bg(hsla(0.0,0.0,0.18,1.0))
                                    .text_xs().text_color(hsla(0.0,0.0,0.6,1.0)).cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.refresh_now(); cx.notify();
                                    }))
                                    .child("↻"),
                            )
                            .child(
                                div().px_3().py_1().rounded_md()
                                    .bg(hsla(0.38,0.55,0.32,1.0))
                                    .text_xs().text_color(white()).cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        let ids: Vec<_> = this.visible_devices().into_iter()
                                            .map(|(_, d)| d.device.iface_id().as_str().to_owned())
                                            .collect();
                                        for id in &ids { call_toggle(id, true); }
                                        for d in this.devices.iter_mut() {
                                            if !d.hidden { d.enabled = true; }
                                        }
                                        cx.notify();
                                    }))
                                    .child("Enable All"),
                            )
                            .child(
                                div().px_3().py_1().rounded_md()
                                    .bg(hsla(0.0,0.52,0.32,1.0))
                                    .text_xs().text_color(white()).cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        let ids: Vec<_> = this.visible_devices().into_iter()
                                            .map(|(_, d)| d.device.iface_id().as_str().to_owned())
                                            .collect();
                                        for id in &ids { call_toggle(id, false); }
                                        for d in this.devices.iter_mut() {
                                            if !d.hidden { d.enabled = false; }
                                        }
                                        cx.notify();
                                    }))
                                    .child("Disable All"),
                            ),
                    ),
            )
            // Device list
            .child(div().flex().flex_col().px_3().pt_3().children(vis_rows))
            // Hidden section
            .when(n_hidden > 0, |el| {
                el.child(
                    div().mx_3().mt_2().mb_3()
                        .child(
                            // Collapsible header
                            div().flex().flex_row().items_center().gap_2()
                                .px_3().py(px(7.0)).rounded_lg()
                                .bg(hsla(0.0,0.0,0.10,1.0)).cursor_pointer()
                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                    this.show_hidden = !this.show_hidden;
                                    cx.notify();
                                }))
                                .child(div().text_xs().text_color(hsla(0.0,0.0,0.5,1.0))
                                    .child(if show_hidden { "▾" } else { "▸" }))
                                .child(div().text_xs().text_color(hsla(0.0,0.0,0.45,1.0))
                                    .child(SharedString::from(
                                        format!("Hidden ({n_hidden})")
                                    ))),
                        )
                        .when(show_hidden, |el| {
                            el.child(div().flex().flex_col().mt(px(2.0)).children(hidden_rows))
                        }),
                )
            })
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(520.0), px(520.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("JoyToggle".into()),
                    ..Default::default()
                }),
                app_id: Some("joytoggle".to_string()),
                ..Default::default()
            },
            |_, cx| cx.new(JoyToggleWindow::new),
        )
        .unwrap();
        cx.activate(true);
    });
}
