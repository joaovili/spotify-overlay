use spotify_overlay_core::layout::Rect;
use spotify_overlay_core::{Config, OverlaySize};
use winit::window::Window;

pub fn set_accessory_policy() {
    platform_impl::set_accessory_policy();
}

pub fn set_regular_policy() {
    platform_impl::set_regular_policy();
}

pub fn primary_work_area() -> Rect {
    platform_impl::primary_work_area()
}

pub fn work_area_for(config: &Config, size: OverlaySize) -> Rect {
    platform_impl::work_area_for(config, size)
}

pub fn work_area_at_point(x: f64, y: f64) -> Rect {
    platform_impl::work_area_at_point(x, y)
}

pub fn apply_overlay_style(window: &Window) {
    platform_impl::apply_overlay_style(window)
}

pub fn show(window: &Window) {
    window.set_visible(true);
    platform_impl::order_front(window);
}

pub fn hide(window: &Window) {
    window.set_visible(false);
}

pub fn set_level(window: &Window, floating: bool) {
    window.set_window_level(if floating {
        winit::window::WindowLevel::AlwaysOnTop
    } else {
        winit::window::WindowLevel::Normal
    });
    platform_impl::set_level(window, floating);
}

#[cfg(target_os = "macos")]
mod platform_impl {
    use super::*;
    use objc2::runtime::AnyObject;
    use objc2::{define_class, ClassType};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSFloatingWindowLevel, NSPanel, NSScreen,
        NSView, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
    };
    use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSRect};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    #[repr(C)]
    struct PanelIvars;

    define_class!(
        #[unsafe(super = NSPanel)]
        #[name = "SpotifyOverlayPanel"]
        #[ivars = PanelIvars]
        struct OverlayPanel;

        unsafe impl NSObjectProtocol for OverlayPanel {}

        impl OverlayPanel {
            #[unsafe(method(canBecomeKeyWindow))]
            fn can_become_key_window(&self) -> bool {
                false
            }

            #[unsafe(method(canBecomeMainWindow))]
            fn can_become_main_window(&self) -> bool {
                false
            }
        }
    );

    fn ns_window(window: &Window) -> Option<objc2::rc::Retained<NSWindow>> {
        let handle = window.window_handle().ok()?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return None;
        };
        let view = unsafe { &*(handle.ns_view.as_ptr() as *const NSView) };
        view.window()
    }

    pub fn set_accessory_policy() {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    }

    pub fn set_regular_policy() {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        app.activate();
    }

    pub fn apply_overlay_style(window: &Window) {
        let Some(ns_window) = ns_window(window) else {
            return;
        };
        let object: &AnyObject = unsafe { &*(&*ns_window as *const NSWindow as *const AnyObject) };
        unsafe {
            AnyObject::set_class(object, OverlayPanel::class());
        }
        ns_window.setStyleMask(ns_window.styleMask() | NSWindowStyleMask::NonactivatingPanel);
        ns_window.setLevel(NSFloatingWindowLevel);
        ns_window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
        ns_window.setHidesOnDeactivate(false);
        ns_window.setHasShadow(false);
        let panel: &NSPanel = unsafe { &*(&*ns_window as *const NSWindow as *const NSPanel) };
        panel.setBecomesKeyOnlyIfNeeded(true);
        panel.setFloatingPanel(true);
    }

    pub fn order_front(window: &Window) {
        if let Some(ns_window) = ns_window(window) {
            ns_window.orderFrontRegardless();
        }
    }

    pub fn set_level(window: &Window, floating: bool) {
        if let Some(ns_window) = ns_window(window) {
            ns_window.setLevel(if floating { NSFloatingWindowLevel } else { 0 });
        }
    }

    fn screen_work_area(screen: &NSScreen, main: NSRect) -> Rect {
        let visible = screen.visibleFrame();
        Rect::new(
            visible.origin.x - main.origin.x,
            (main.origin.y + main.size.height) - (visible.origin.y + visible.size.height),
            visible.size.width,
            visible.size.height,
        )
    }

    fn main_screen() -> Option<(objc2::rc::Retained<NSScreen>, NSRect)> {
        let mtm = MainThreadMarker::new()?;
        let screen = NSScreen::mainScreen(mtm)?;
        let frame = screen.frame();
        Some((screen, frame))
    }

    pub fn primary_work_area() -> Rect {
        match main_screen() {
            Some((screen, frame)) => screen_work_area(&screen, frame),
            None => Rect::new(0.0, 0.0, 1440.0, 900.0),
        }
    }

    pub fn work_area_at_point(x: f64, y: f64) -> Rect {
        let Some(mtm) = MainThreadMarker::new() else {
            return primary_work_area();
        };
        let Some(main) = NSScreen::mainScreen(mtm) else {
            return primary_work_area();
        };
        let main_frame = main.frame();
        let screens = NSScreen::screens(mtm);
        for index in 0..screens.count() {
            let screen = screens.objectAtIndex(index);
            let area = screen_work_area(&screen, main_frame);
            if x >= area.x && x <= area.x + area.width && y >= area.y && y <= area.y + area.height {
                return area;
            }
        }
        screen_work_area(&main, main_frame)
    }

    pub fn work_area_for(config: &Config, _size: OverlaySize) -> Rect {
        let Some(mtm) = MainThreadMarker::new() else {
            return primary_work_area();
        };
        let Some(main) = NSScreen::mainScreen(mtm) else {
            return primary_work_area();
        };
        let main_frame = main.frame();
        if config.position == "custom" {
            let screens = NSScreen::screens(mtm);
            for index in 0..screens.count() {
                let screen = screens.objectAtIndex(index);
                let area = screen_work_area(&screen, main_frame);
                let x = config.custom_x as f64;
                let y = area.y + config.custom_y as f64;
                if x >= area.x
                    && x <= area.x + area.width
                    && y >= area.y
                    && y <= area.y + area.height
                {
                    return area;
                }
            }
        }
        screen_work_area(&main, main_frame)
    }
}

#[cfg(target_os = "windows")]
mod platform_impl {
    pub fn set_accessory_policy() {}
    pub fn set_regular_policy() {}

    use super::*;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA};

    pub fn primary_work_area() -> Rect {
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 1440,
            bottom: 900,
        };
        unsafe {
            SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut rect as *mut _ as *mut _, 0);
        }
        Rect::new(
            rect.left as f64,
            rect.top as f64,
            (rect.right - rect.left) as f64,
            (rect.bottom - rect.top) as f64,
        )
    }

    pub fn work_area_for(_config: &Config, _size: OverlaySize) -> Rect {
        primary_work_area()
    }

    pub fn work_area_at_point(_x: f64, _y: f64) -> Rect {
        primary_work_area()
    }

    pub fn apply_overlay_style(window: &Window) {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(handle) = window.window_handle() {
            if let RawWindowHandle::Win32(handle) = handle.as_raw() {
                spotify_overlay_media::windows::apply_overlay_window_style(
                    handle.hwnd.get() as isize
                );
            }
        }
    }

    pub fn order_front(_window: &Window) {}

    pub fn set_level(_window: &Window, _floating: bool) {}
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform_impl {
    use super::*;

    pub fn set_accessory_policy() {}
    pub fn set_regular_policy() {}

    pub fn primary_work_area() -> Rect {
        Rect::new(0.0, 0.0, 1440.0, 900.0)
    }

    pub fn work_area_for(_config: &Config, _size: OverlaySize) -> Rect {
        primary_work_area()
    }

    pub fn work_area_at_point(_x: f64, _y: f64) -> Rect {
        primary_work_area()
    }

    pub fn apply_overlay_style(_window: &Window) {}

    pub fn order_front(_window: &Window) {}

    pub fn set_level(_window: &Window, _floating: bool) {}
}
