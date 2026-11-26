#![cfg(all(target_os = "linux", feature = "wayland", feature = "test-support"))]

use gpui::{
    AppContext as _, Application, Bounds, Context, IntoElement, ParentElement as _, Render,
    Styled as _, Window, WindowBounds, WindowOptions, div, px, rgb, size,
};
use std::{cell::Cell, rc::Rc};

struct CaptureView {
    blue: bool,
}

impl Render for CaptureView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(if self.blue { 0x0000ff } else { 0xff0000 }))
            .text_color(rgb(0xffffff))
            .child("Wayland capture exercises the glyph atlas")
    }
}

#[test]
#[ignore = "requires a Wayland compositor; run under headless Sway with --ignored"]
fn captures_a_wayland_window_without_presenting() {
    assert_eq!(gpui::guess_compositor(), "Wayland");
    let completed = Rc::new(Cell::new(false));
    Application::with_platform(gpui_linux::current_platform(false)).run({
        let completed = completed.clone();
        move |cx| {
            let handle = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                            gpui::point(px(0.), px(0.)),
                            size(px(321.), px(203.)),
                        ))),
                        show: false,
                        ..Default::default()
                    },
                    |_, cx| cx.new(|_| CaptureView { blue: false }),
                )
                .expect("open a Wayland window");
            let first = cx
                .update_window(handle.into(), |_, window, cx| {
                    // draw builds the scene; presentation is driven separately by the event loop.
                    window.draw(cx).clear(cx);
                    let first = window.render_to_image().expect("capture before presenting");
                    let dimensions = first.dimensions();
                    assert!(dimensions.0 > 0 && dimensions.1 > 0);
                    assert_eq!(
                        first.get_pixel(dimensions.0 / 2, dimensions.1 - 2).0,
                        [255, 0, 0, 255]
                    );
                    assert!(
                        first.pixels().any(|pixel| pixel[1] > 100),
                        "text must use the existing glyph atlas"
                    );
                    assert_eq!(window.render_to_image().expect("repeat capture"), first);
                    first
                })
                .expect("draw capture window");
            handle
                .update(cx, |view, _, cx| {
                    view.blue = true;
                    cx.notify();
                })
                .expect("change scene");
            cx.update_window(handle.into(), |_, window, cx| {
                window.draw(cx).clear(cx);
                let second = window.render_to_image().expect("capture updated scene");
                let dimensions = first.dimensions();
                assert_eq!(second.dimensions(), dimensions);
                assert_eq!(
                    second.get_pixel(dimensions.0 / 2, dimensions.1 - 2).0,
                    [0, 0, 255, 255]
                );
            })
            .expect("draw updated scene");
            completed.set(true);
            // The platform starts its event loop after the launch callback returns.
            cx.spawn(async |cx| cx.update(|cx| cx.quit())).detach();
        }
    });
    assert!(completed.get());
}
