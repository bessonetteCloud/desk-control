#![cfg(target_os = "macos")]

use cocoa::appkit::{
    NSApp, NSApplication, NSMenu, NSMenuItem, NSRunningApplication, NSStatusBar,
    NSStatusItem, NSVariableStatusItemLength,
};
use cocoa::base::{id, nil, selector};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{msg_send, sel, sel_impl};
use std::sync::{Arc, Mutex};

use crate::config::{Config, DrinkSize};
use super::icons;

/// Callback handler for menu item actions
pub trait MenuCallback: Send + Sync {
    fn on_preset_selected(&self, preset: DrinkSize);
    fn on_configure_desk(&self);
    fn on_configure_presets(&self);
    fn on_quit(&self);
}

/// macOS menu bar application
pub struct MenuBarApp {
    #[allow(dead_code)]
    status_item: id,
    #[allow(dead_code)]
    pool: id,
}

impl MenuBarApp {
    /// Create a new menu bar application
    pub fn new(config: Config, callback: Arc<dyn MenuCallback>) -> Self {
        unsafe {
            let pool = NSAutoreleasePool::new(nil);

            // Get the shared application instance
            let app = NSApp();
            app.setActivationPolicy_(
                cocoa::appkit::NSApplicationActivationPolicyAccessory,
            );

            // Create status bar item
            let status_bar = NSStatusBar::systemStatusBar(nil);
            let status_item = status_bar.statusItemWithLength_(
                NSVariableStatusItemLength,
            );

            // Set the status bar button title/icon
            let button = status_item.button();
            let title = NSString::alloc(nil)
                .init_str("⌘"); // Desk control symbol
            let _: () = msg_send![button, setTitle: title];

            // Create menu
            let menu = NSMenu::new(nil).autorelease();

            // Add title item
            let title_item = create_menu_item("Desk Control", None, None);
            let _: () = msg_send![title_item, setEnabled: false];
            menu.addItem_(title_item);

            // Add separator
            menu.addItem_(NSMenuItem::separatorItem(nil));

            // Add preset menu items
            for preset in DrinkSize::all() {
                let height_mm = config.get_preset(preset);
                let height_cm = height_mm as f32 / 10.0;

                let label = format!(
                    "{} {} - {:.1}cm",
                    icons::get_emoji_for_size(preset.name()),
                    preset.name(),
                    height_cm
                );

                let item = create_menu_item(
                    &label,
                    Some(sel!(presetAction:)),
                    None,
                );

                // Store the preset as a tag
                let tag = preset as i64;
                let _: () = msg_send![item, setTag: tag];

                menu.addItem_(item);
            }

            // Add separator
            menu.addItem_(NSMenuItem::separatorItem(nil));

            // Add configuration items
            let configure_desk = create_menu_item(
                "Configure Desk...",
                Some(sel!(configureDeskAction:)),
                None,
            );
            menu.addItem_(configure_desk);

            let configure_presets = create_menu_item(
                "Configure Presets...",
                Some(sel!(configurePresetsAction:)),
                None,
            );
            menu.addItem_(configure_presets);

            // Add separator
            menu.addItem_(NSMenuItem::separatorItem(nil));

            // Add quit item
            let quit = create_menu_item(
                "Quit",
                Some(sel!(quitAction:)),
                Some("q"),
            );
            menu.addItem_(quit);

            // Set the menu
            status_item.setMenu_(menu);

            // Create and set the delegate
            let delegate = create_delegate(callback);
            let _: () = msg_send![app, setDelegate: delegate];

            Self {
                status_item,
                pool,
            }
        }
    }

    /// Run the application event loop
    pub fn run() {
        unsafe {
            let app = NSApp();
            app.run();
        }
    }
}

/// Create a menu item
unsafe fn create_menu_item(
    title: &str,
    action: Option<Sel>,
    key_equivalent: Option<&str>,
) -> id {
    let title = NSString::alloc(nil).init_str(title);
    let key = if let Some(k) = key_equivalent {
        NSString::alloc(nil).init_str(k)
    } else {
        NSString::alloc(nil).init_str("")
    };

    let item = NSMenuItem::alloc(nil);
    let item = msg_send![item, initWithTitle:title action:action keyEquivalent:key];
    item
}

/// Create application delegate with callback
fn create_delegate(callback: Arc<dyn MenuCallback>) -> id {
    static mut DELEGATE_CLASS: Option<&Class> = None;

    unsafe {
        // Define the delegate class once
        if DELEGATE_CLASS.is_none() {
            let superclass = Class::get("NSObject").unwrap();
            let mut decl = ClassDecl::new("AppDelegate", superclass).unwrap();

            // Add callback storage
            decl.add_ivar::<*mut std::ffi::c_void>("callback");

            // Add action methods
            extern "C" fn preset_action(this: &Object, _cmd: Sel, sender: id) {
                unsafe {
                    let callback_ptr: *mut std::ffi::c_void =
                        *this.get_ivar("callback");
                    let callback = &*(callback_ptr
                        as *const Arc<dyn MenuCallback>);

                    let tag: i64 = msg_send![sender, tag];
                    let preset = match tag {
                        0 => DrinkSize::Short,
                        1 => DrinkSize::Tall,
                        2 => DrinkSize::Grande,
                        3 => DrinkSize::Venti,
                        _ => return,
                    };

                    callback.on_preset_selected(preset);
                }
            }

            extern "C" fn configure_desk_action(
                this: &Object,
                _cmd: Sel,
                _sender: id,
            ) {
                unsafe {
                    let callback_ptr: *mut std::ffi::c_void =
                        *this.get_ivar("callback");
                    let callback = &*(callback_ptr
                        as *const Arc<dyn MenuCallback>);
                    callback.on_configure_desk();
                }
            }

            extern "C" fn configure_presets_action(
                this: &Object,
                _cmd: Sel,
                _sender: id,
            ) {
                unsafe {
                    let callback_ptr: *mut std::ffi::c_void =
                        *this.get_ivar("callback");
                    let callback = &*(callback_ptr
                        as *const Arc<dyn MenuCallback>);
                    callback.on_configure_presets();
                }
            }

            extern "C" fn quit_action(_this: &Object, _cmd: Sel, _sender: id) {
                unsafe {
                    let app = NSApp();
                    let _: () = msg_send![app, terminate: nil];
                }
            }

            unsafe {
                decl.add_method(
                    sel!(presetAction:),
                    preset_action
                        as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(configureDeskAction:),
                    configure_desk_action
                        as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(configurePresetsAction:),
                    configure_presets_action
                        as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(quitAction:),
                    quit_action as extern "C" fn(&Object, Sel, id),
                );
            }

            DELEGATE_CLASS = Some(decl.register());
        }

        // Create an instance
        let delegate: id = msg_send![DELEGATE_CLASS.unwrap(), new];

        // Store the callback
        let callback_box = Box::new(callback);
        let callback_ptr = Box::into_raw(callback_box) as *mut std::ffi::c_void;
        (*delegate).set_ivar("callback", callback_ptr);

        delegate
    }
}
