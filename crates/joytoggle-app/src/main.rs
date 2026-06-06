mod dbus;

use gpui::{
    App, Application, Bounds, Context, FontWeight, MouseButton, SharedString, Window,
    WindowBounds, WindowOptions, div, hsla, prelude::*, px, size, white,
};
use joytoggle_core::{Device, DeviceType, InterfaceId};
use tokio::runtime::Handle;

// ── D-Bus helpers ─────────────────────────────────────────────────────────────

fn fetch_devices(handle: &Handle) -> Vec<Device> {
    handle.block_on(async {
        let conn  = zbus::Connection::system().await?;
        let proxy = dbus::JoyToggleDaemonProxy::new(&conn).await?;
        let json  = proxy.list_devices().await?;
        let devs: Vec<Device> = serde_json::from_str(&json)?;
        anyhow::Ok(devs)
    })
    .unwrap_or_default()
}

fn call_toggle(handle: &Handle, iface_id: &str, enable: bool) -> bool {
    handle.block_on(async {
        let conn  = zbus::Connection::system().await?;
        let proxy = dbus::JoyToggleDaemonProxy::new(&conn).await?;
        if enable {
            proxy.enable_device(iface_id).await?;
        } else {
            proxy.disable_device(iface_id).await?;
        }
        anyhow::Ok(())
    })
    .is_ok()
}

// ── App state ─────────────────────────────────────────────────────────────────

struct DeviceItem {
    device:  Device,
    enabled: bool,
}

struct JoyToggleWindow {
    devices:      Vec<DeviceItem>,
    tokio_handle: Handle,
    daemon_error: Option<String>,
}

impl JoyToggleWindow {
    fn new(tokio_handle: Handle) -> Self {
        let devs = fetch_devices(&tokio_handle);
        let (devices, daemon_error) = if devs.is_empty() {
            (
                mock_devices(),
                Some("joytoggle-daemon not reachable — showing mock data".to_owned()),
            )
        } else {
            (devs, None)
        };

        Self {
            devices: devices.into_iter().map(|d| {
                let enabled = d.enabled;
                DeviceItem { device: d, enabled }
            }).collect(),
            tokio_handle,
            daemon_error,
        }
    }

    fn active_count(&self) -> usize {
        self.devices.iter().filter(|d| d.enabled).count()
    }

    fn refresh(&mut self) {
        let fresh = fetch_devices(&self.tokio_handle);
        if !fresh.is_empty() {
            self.daemon_error = None;
            self.devices = fresh.into_iter().map(|d| {
                let enabled = d.enabled;
                DeviceItem { device: d, enabled }
            }).collect();
        }
    }
}

// ── Fallback mock data ────────────────────────────────────────────────────────

fn mock_devices() -> Vec<Device> {
    vec![
        Device {
            event: "event0".into(), dev_path: "/dev/input/event0".into(),
            name: "VIRPIL VPC Stick WarBRD-D (mock)".into(),
            device_type: DeviceType::Joystick, autohide: false,
            usb_path: Some("/sys/bus/usb/1-11.4.1:1.0".into()),
            interface_id: Some(InterfaceId::from("1-11.4.1:1.0")),
            vendor_id: None, product_id: None, enabled: true,
        },
        Device {
            event: "event1".into(), dev_path: "/dev/input/event1".into(),
            name: "VIRPIL VPC VMAX Throttle (mock)".into(),
            device_type: DeviceType::Throttle, autohide: false,
            usb_path: Some("/sys/bus/usb/1-11.4.3:1.0".into()),
            interface_id: Some(InterfaceId::from("1-11.4.3:1.0")),
            vendor_id: None, product_id: None, enabled: true,
        },
    ]
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn device_icon(t: &DeviceType) -> &'static str {
    match t {
        DeviceType::Joystick      => "◈",
        DeviceType::Throttle      => "▶",
        DeviceType::RudderPedals  => "↕",
        DeviceType::Gamepad       => "⊕",
        DeviceType::SteeringWheel => "◎",
        DeviceType::Unknown       => "?",
    }
}

fn type_color(t: &DeviceType) -> gpui::Hsla {
    match t {
        DeviceType::Joystick      => hsla(0.55, 0.7, 0.6, 1.0),
        DeviceType::Throttle      => hsla(0.08, 0.7, 0.6, 1.0),
        DeviceType::RudderPedals  => hsla(0.75, 0.6, 0.6, 1.0),
        DeviceType::Gamepad       => hsla(0.33, 0.6, 0.55, 1.0),
        DeviceType::SteeringWheel => hsla(0.15, 0.65, 0.6, 1.0),
        DeviceType::Unknown       => hsla(0.0, 0.0, 0.5, 1.0),
    }
}

impl Render for JoyToggleWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active       = self.active_count();
        let total        = self.devices.len();
        let error_banner = self.daemon_error.clone();

        let rows: Vec<_> = self.devices.iter().enumerate().map(|(i, item)| {
            let enabled      = item.enabled;
            let name         = item.device.name.clone();
            let dtype        = item.device.device_type.clone();
            let iface_id     = item.device.iface_id().as_str().to_owned();
            let bg           = if enabled { hsla(0.0, 0.0, 0.13, 1.0) } else { hsla(0.0, 0.0, 0.09, 1.0) };
            let name_color   = if enabled { hsla(0.0, 0.0, 0.92, 1.0) } else { hsla(0.0, 0.0, 0.4, 1.0) };
            let toggle_track = if enabled { hsla(0.38, 0.6, 0.38, 1.0) } else { hsla(0.0, 0.0, 0.22, 1.0) };

            div()
                .flex().flex_row().items_center().gap_3()
                .px_4().py(px(10.0)).mb(px(2.0)).rounded_lg().bg(bg)
                .child(
                    div().w_6().h_6().flex().items_center().justify_center()
                        .text_color(type_color(&dtype)).child(device_icon(&dtype)),
                )
                .child(
                    div().flex().flex_col().flex_1()
                        .child(div().text_color(name_color).text_sm()
                            .font_weight(FontWeight::MEDIUM).child(name))
                        .child(div().text_color(hsla(0.0, 0.0, 0.42, 1.0)).text_xs()
                            .child(SharedString::from(format!("{} — {}", dtype, iface_id)))),
                )
                .child(
                    div()
                        .w(px(44.0)).h(px(24.0)).rounded_full().bg(toggle_track)
                        .flex().items_center().px(px(3.0)).cursor_pointer()
                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                            let want_on = !this.devices[i].enabled;
                            let ok = call_toggle(&this.tokio_handle, &iface_id, want_on);
                            if ok {
                                this.devices[i].enabled = want_on;
                            }
                            cx.notify();
                        }))
                        .child(
                            div().w(px(18.0)).h(px(18.0)).rounded_full().bg(white())
                                .when(enabled, |d| d.ml_auto()),
                        ),
                )
        }).collect();

        div().flex().flex_col().w_full().h_full().bg(hsla(0.0, 0.0, 0.07, 1.0))
            .when_some(error_banner, |el, msg| {
                el.child(
                    div().px_4().py(px(6.0)).bg(hsla(0.08, 0.55, 0.22, 1.0))
                        .text_xs().text_color(hsla(0.1, 0.9, 0.8, 1.0))
                        .child(SharedString::from(format!("⚠ {msg}"))),
                )
            })
            .child(
                div().flex().flex_row().items_center().justify_between()
                    .px_5().py_4().border_b_1().border_color(hsla(0.0, 0.0, 0.15, 1.0))
                    .child(
                        div().flex().flex_col()
                            .child(div().text_base().font_weight(FontWeight::SEMIBOLD)
                                .text_color(white()).child("JoyToggle"))
                            .child(div().text_xs().text_color(hsla(0.0, 0.0, 0.5, 1.0))
                                .child(SharedString::from(format!("{active} / {total} active")))),
                    )
                    .child(
                        div().flex().gap_2()
                            .child(
                                div().px_3().py_1().rounded_md()
                                    .bg(hsla(0.0, 0.0, 0.18, 1.0))
                                    .text_xs().text_color(hsla(0.0, 0.0, 0.6, 1.0)).cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.refresh();
                                        cx.notify();
                                    }))
                                    .child("↻ Refresh"),
                            )
                            .child(
                                div().px_3().py_1().rounded_md()
                                    .bg(hsla(0.38, 0.55, 0.32, 1.0))
                                    .text_xs().text_color(white()).cursor_pointer()
                                    .child("Enable All"),
                            )
                            .child(
                                div().px_3().py_1().rounded_md()
                                    .bg(hsla(0.0, 0.52, 0.32, 1.0))
                                    .text_xs().text_color(white()).cursor_pointer()
                                    .child("Disable All"),
                            ),
                    ),
            )
            .child(div().flex().flex_col().px_3().py_3().children(rows))
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    // Single tokio runtime for all D-Bus calls.
    // GPUI callbacks use handle.block_on — acceptable for a spike.
    // TODO: non-blocking via cx.spawn + tokio channel when GPUI executor matures.
    let rt     = tokio::runtime::Runtime::new().expect("tokio runtime");
    let handle = rt.handle().clone();

    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(520.0), px(500.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| JoyToggleWindow::new(handle)),
        )
        .unwrap();
        cx.activate(true);
    });
}
