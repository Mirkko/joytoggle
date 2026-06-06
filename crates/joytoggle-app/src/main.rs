use gpui::{
    App, Application, Bounds, Context, FontWeight, MouseButton, SharedString, Window,
    WindowBounds, WindowOptions, div, hsla, prelude::*, px, size, white,
};
use joytoggle_core::{Device, DeviceType, InterfaceId};

// ── Mock data ─────────────────────────────────────────────────────────────────

fn mock_devices() -> Vec<Device> {
    vec![
        Device {
            event: "event0".into(),
            dev_path: "/dev/input/event0".into(),
            name: "VIRPIL Controls VPC Stick WarBRD-D".into(),
            device_type: DeviceType::Joystick,
            autohide: false,
            usb_path: Some("/sys/bus/usb/1-11.4.1:1.0".into()),
            interface_id: Some(InterfaceId::from("1-11.4.1:1.0")),
            vendor_id: Some("3344".into()),
            product_id: Some("0194".into()),
            enabled: true,
        },
        Device {
            event: "event1".into(),
            dev_path: "/dev/input/event1".into(),
            name: "VIRPIL Controls VPC VMAX Prime Throttle".into(),
            device_type: DeviceType::Throttle,
            autohide: false,
            usb_path: Some("/sys/bus/usb/1-11.4.3:1.0".into()),
            interface_id: Some(InterfaceId::from("1-11.4.3:1.0")),
            vendor_id: Some("3344".into()),
            product_id: Some("0195".into()),
            enabled: true,
        },
        Device {
            event: "event2".into(),
            dev_path: "/dev/input/event2".into(),
            name: "VIRPIL Controls VPC Rudder Pedals".into(),
            device_type: DeviceType::RudderPedals,
            autohide: false,
            usb_path: Some("/sys/bus/usb/1-6:1.0".into()),
            interface_id: Some(InterfaceId::from("1-6:1.0")),
            vendor_id: Some("3344".into()),
            product_id: Some("0196".into()),
            enabled: false,
        },
    ]
}

// ── App state ─────────────────────────────────────────────────────────────────

struct DeviceItem {
    device: Device,
    enabled: bool,
}

struct JoyToggleWindow {
    devices: Vec<DeviceItem>,
}

impl JoyToggleWindow {
    fn new() -> Self {
        Self {
            devices: mock_devices()
                .into_iter()
                .map(|d| {
                    let enabled = d.enabled;
                    DeviceItem { device: d, enabled }
                })
                .collect(),
        }
    }

    fn active_count(&self) -> usize {
        self.devices.iter().filter(|d| d.enabled).count()
    }
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
        let total = self.devices.len();

        let device_rows: Vec<_> = self
            .devices
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let enabled = item.enabled;
                let name = item.device.name.clone();
                let dtype = item.device.device_type.clone();
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
                let icon = device_icon(&dtype);
                let icolor = type_color(&dtype);
                let type_label = dtype.to_string();

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
                    // Icon
                    .child(
                        div()
                            .w_6()
                            .h_6()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(icolor)
                            .child(icon),
                    )
                    // Name + type
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
                                    .child(name),
                            )
                            .child(
                                div()
                                    .text_color(hsla(0.0, 0.0, 0.42, 1.0))
                                    .text_xs()
                                    .child(type_label),
                            ),
                    )
                    // Toggle
                    .child(
                        div()
                            .w(px(44.0))
                            .h(px(24.0))
                            .rounded_full()
                            .bg(toggle_track)
                            .flex()
                            .items_center()
                            .px(px(3.0))
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, _| {
                                if let Some(d) = this.devices.get_mut(i) {
                                    d.enabled = !d.enabled;
                                }
                            }))
                            .child(
                                div()
                                    .w(px(18.0))
                                    .h(px(18.0))
                                    .rounded_full()
                                    .bg(white())
                                    .when(enabled, |d| d.ml_auto()),
                            ),
                    )
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(hsla(0.0, 0.0, 0.07, 1.0))
            // Header
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px_5()
                    .py_4()
                    .border_b_1()
                    .border_color(hsla(0.0, 0.0, 0.15, 1.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_base()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(white())
                                    .child("JoyToggle"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                                    .child(SharedString::from(format!("{active} / {total} active"))),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(hsla(0.38, 0.55, 0.32, 1.0))
                                    .text_xs()
                                    .text_color(white())
                                    .cursor_pointer()
                                    .child("Enable All"),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(hsla(0.0, 0.52, 0.32, 1.0))
                                    .text_xs()
                                    .text_color(white())
                                    .cursor_pointer()
                                    .child("Disable All"),
                            ),
                    ),
            )
            // Device list
            .child(
                div()
                    .flex()
                    .flex_col()
                    .px_3()
                    .py_3()
                    .children(device_rows),
            )
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(500.0), px(460.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| JoyToggleWindow::new()),
        )
        .unwrap();
        cx.activate(true);
    });
}
