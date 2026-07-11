use std::cell::RefCell;
use std::rc::Rc;

use calloop::{EventLoop, LoopHandle};
use gpui_util::ResultExt;

use crate::linux::headless::window::{HeadlessDisplay, HeadlessWindow};
use crate::linux::{LinuxClient, LinuxCommon, LinuxKeyboardLayout};
use gpui::{
    AnyWindowHandle, CursorStyle, DisplayId, PlatformDisplay, PlatformKeyboardLayout,
    PlatformWindow, WindowParams,
};

pub struct HeadlessClientState {
    pub(crate) _loop_handle: LoopHandle<'static, HeadlessClient>,
    pub(crate) event_loop: Option<calloop::EventLoop<'static, HeadlessClient>>,
    pub(crate) common: LinuxCommon,
    pub(crate) display: Rc<dyn PlatformDisplay>,
}

#[derive(Clone)]
pub(crate) struct HeadlessClient(Rc<RefCell<HeadlessClientState>>);

impl HeadlessClient {
    pub(crate) fn new() -> Self {
        let event_loop = EventLoop::try_new().unwrap();

        let (common, main_receiver, wake_receiver) = LinuxCommon::new(event_loop.get_signal());

        let handle = event_loop.handle();

        handle
            .insert_source(main_receiver, |event, _, _: &mut HeadlessClient| {
                if let calloop::channel::Event::Msg(runnable) = event {
                    runnable.run();
                }
            })
            .ok();

        handle
            .insert_source(wake_receiver, |event, _, client: &mut HeadlessClient| {
                if let calloop::channel::Event::Msg(()) = event {
                    client.with_common(|common| common.handle_system_wake());
                }
            })
            .ok();

        HeadlessClient(Rc::new(RefCell::new(HeadlessClientState {
            event_loop: Some(event_loop),
            _loop_handle: handle,
            common,
            display: Rc::new(HeadlessDisplay::new()),
        })))
    }
}

impl LinuxClient for HeadlessClient {
    fn with_common<R>(&self, f: impl FnOnce(&mut LinuxCommon) -> R) -> R {
        f(&mut self.0.borrow_mut().common)
    }

    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        Box::new(LinuxKeyboardLayout::new("unknown".into()))
    }

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        vec![self.0.borrow().display.clone()]
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(self.0.borrow().display.clone())
    }

    fn display(&self, id: DisplayId) -> Option<Rc<dyn PlatformDisplay>> {
        let display = self.0.borrow().display.clone();
        (display.id() == id).then_some(display)
    }

    #[cfg(feature = "screen-capture")]
    fn screen_capture_sources(
        &self,
    ) -> futures::channel::oneshot::Receiver<anyhow::Result<Vec<Rc<dyn gpui::ScreenCaptureSource>>>>
    {
        let (tx, rx) = futures::channel::oneshot::channel();
        tx.send(Err(anyhow::anyhow!(
            "Headless mode does not support screen capture."
        )))
        .ok();
        rx
    }

    fn active_window(&self) -> Option<AnyWindowHandle> {
        None
    }

    fn window_stack(&self) -> Option<Vec<AnyWindowHandle>> {
        None
    }

    fn open_window(
        &self,
        _handle: AnyWindowHandle,
        params: WindowParams,
    ) -> anyhow::Result<Box<dyn PlatformWindow>> {
        Ok(Box::new(HeadlessWindow::new(
            params,
            self.0.borrow().display.clone(),
        )))
    }

    fn compositor_name(&self) -> &'static str {
        "headless"
    }

    fn set_cursor_style(&self, _style: CursorStyle) {}

    fn open_uri(&self, _uri: &str) {}

    fn reveal_path(&self, _path: std::path::PathBuf) {}

    fn write_to_primary(&self, _item: gpui::ClipboardItem) {}

    fn write_to_clipboard(&self, _item: gpui::ClipboardItem) {}

    fn read_from_primary(&self) -> Option<gpui::ClipboardItem> {
        None
    }

    fn read_from_clipboard(&self) -> Option<gpui::ClipboardItem> {
        None
    }

    fn run(&self) {
        let mut event_loop = self
            .0
            .borrow_mut()
            .event_loop
            .take()
            .expect("App is already running");

        event_loop.run(None, &mut self.clone(), |_| {}).log_err();
    }
}

#[cfg(feature = "test-support")]
pub struct LinuxHeadlessRenderer {
    renderer: gpui_wgpu::WgpuHeadlessRenderer,
}

#[cfg(feature = "test-support")]
impl LinuxHeadlessRenderer {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            renderer: gpui_wgpu::WgpuHeadlessRenderer::new()?,
        })
    }
}

#[cfg(feature = "test-support")]
impl gpui::PlatformHeadlessRenderer for LinuxHeadlessRenderer {
    fn render_scene_to_image(
        &mut self,
        scene: &gpui::Scene,
        size: gpui::Size<gpui::DevicePixels>,
    ) -> anyhow::Result<image::RgbaImage> {
        self.renderer.render_scene_to_image(scene, size)
    }

    fn render_scene(
        &mut self,
        scene: &gpui::Scene,
        size: gpui::Size<gpui::DevicePixels>,
    ) -> anyhow::Result<()> {
        self.renderer.render_scene(scene, size)
    }

    fn sprite_atlas(&self) -> std::sync::Arc<dyn gpui::PlatformAtlas> {
        self.renderer.sprite_atlas()
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use std::sync::Arc;

    use gpui::{
        AppContext as _, Context, HeadlessAppContext, IntoElement, NoopTextSystem, ParentElement,
        Render, Styled, Window, div, px, rgb, size,
    };

    use super::LinuxHeadlessRenderer;

    struct CaptureView;

    impl Render for CaptureView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(0x102030))
                .child(
                    div()
                        .size(px(32.))
                        .rounded_lg()
                        .shadow_lg()
                        .bg(rgb(0xff0000)),
                )
        }
    }

    fn capture_test_view() -> image::RgbaImage {
        let mut cx = HeadlessAppContext::with_platform(
            Arc::new(NoopTextSystem::new()),
            Arc::new(()),
            || {
                LinuxHeadlessRenderer::new()
                    .ok()
                    .map(|renderer| Box::new(renderer) as Box<dyn gpui::PlatformHeadlessRenderer>)
            },
        );
        let window = cx
            .open_window(size(px(96.), px(64.)), |_, cx| cx.new(|_| CaptureView))
            .expect("failed to open headless test window");
        let window = window.into();

        cx.update_window(window, |_, window, cx| {
            let _ = window.draw(cx);
        })
        .expect("failed to draw headless test window");

        let screenshot = cx
            .capture_screenshot(window)
            .expect("failed to capture headless screenshot");
        assert!(screenshot.width() >= 96);
        assert!(screenshot.height() >= 64);
        assert!(
            screenshot.pixels().any(|pixel| {
                let [red, green, blue, alpha] = pixel.0;
                red > 200 && green < 80 && blue < 80 && alpha > 200
            }),
            "expected at least one opaque red pixel in headless screenshot"
        );

        for capture_index in 1..=8 {
            let repeated_screenshot = cx
                .capture_screenshot(window)
                .expect("failed to capture repeated headless screenshot");
            assert_captures_equal(
                &screenshot,
                &repeated_screenshot,
                &format!("capture {capture_index} within one renderer instance"),
            );
        }

        screenshot
    }

    fn assert_captures_equal(
        expected: &image::RgbaImage,
        actual: &image::RgbaImage,
        context: &str,
    ) {
        assert_eq!(
            actual.dimensions(),
            expected.dimensions(),
            "{context}: capture dimensions differed"
        );

        let first_difference = expected
            .pixels()
            .zip(actual.pixels())
            .enumerate()
            .find(|(_, (expected, actual))| expected != actual);
        assert!(
            first_difference.is_none(),
            "{context}: first differing pixel: {first_difference:?}"
        );
    }

    #[test]
    #[ignore = "requires a working Vulkan or OpenGL adapter"]
    fn captures_are_deterministic_for_selected_adapter() {
        let screenshot = capture_test_view();
        for renderer_index in 2..=5 {
            let repeated_screenshot = capture_test_view();
            assert_captures_equal(
                &screenshot,
                &repeated_screenshot,
                &format!("renderer instance {renderer_index}"),
            );
        }
    }
}
