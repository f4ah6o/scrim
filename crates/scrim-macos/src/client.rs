//! macOS accessibility client implementation.

use crate::permissions::{is_trusted, request_trust};
use scrim_core::{AccessibilityClient, AccessibilityError, ElementFilter, Rect, Result, UIElement};

use accessibility::{AXAttribute, AXUIElement};
use accessibility_sys::{
    kAXFocusedApplicationAttribute, kAXFocusedUIElementAttribute, kAXPositionAttribute,
    kAXRoleAttribute, kAXSizeAttribute, kAXTitleAttribute, kAXValueTypeCGPoint, kAXValueTypeCGSize,
    AXValueGetValue, AXValueRef,
};
use core_foundation::{
    array::CFArray,
    base::{CFType, TCFType},
    string::CFString,
};
use core_graphics::display::CGDisplay;
use core_graphics::geometry::{CGPoint, CGSize};
use std::mem::MaybeUninit;
use tracing::warn;

/// macOS accessibility client using the AX API.
pub struct MacOSAccessibilityClient;

impl MacOSAccessibilityClient {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacOSAccessibilityClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AccessibilityClient for MacOSAccessibilityClient {
    fn is_trusted(&self) -> bool {
        is_trusted()
    }

    fn request_trust(&self) -> bool {
        request_trust()
    }

    fn get_focused_window(&self) -> Result<UIElement> {
        if !self.is_trusted() {
            return Err(AccessibilityError::PermissionDenied);
        }

        #[cfg(target_os = "macos")]
        unsafe {
            // Try to get the focused application first
            let result = self.try_get_focused_window();

            if result.is_err() {
                warn!("Failed to get focused window, trying fallback to any accessible window");
                return self.try_get_any_window();
            }

            result
        }

        #[cfg(not(target_os = "macos"))]
        Err(AccessibilityError::Platform(
            "macOS accessibility is only available on macOS".to_string(),
        ))
    }

    fn traverse_tree(&self, root: &UIElement, filter: ElementFilter) -> Vec<UIElement> {
        let mut elements = Vec::new();
        self.traverse_recursive(root, &filter, &mut elements);
        elements
    }

    async fn perform_action(&self, element_id: &str, action: &str) -> Result<()> {
        if !self.is_trusted() {
            return Err(AccessibilityError::PermissionDenied);
        }

        warn!(
            "perform_action not fully implemented: element_id={}, action={}",
            element_id, action
        );
        Ok(())
    }
}

impl MacOSAccessibilityClient {
    #[cfg(target_os = "macos")]
    unsafe fn try_get_focused_window(&self) -> Result<UIElement> {
        // Get the system-wide element
        let system_element = AXUIElement::system_wide();

        // Get the focused application
        let app_cf: CFType = system_element
            .attribute(&AXAttribute::new(&CFString::from_static_string(
                kAXFocusedApplicationAttribute,
            )))
            .map_err(|e| {
                AccessibilityError::Platform(format!("Failed to get focused app: {:?}", e))
            })?;

        let app_element = AXUIElement::wrap_under_get_rule(app_cf.as_CFTypeRef() as *mut _);

        // Get the focused window from the focused application
        let window_cf: CFType = app_element
            .attribute(&AXAttribute::new(&CFString::from_static_string(
                kAXFocusedUIElementAttribute,
            )))
            .map_err(|e| {
                AccessibilityError::Platform(format!("Failed to get focused window: {:?}", e))
            })?;

        let window_element = AXUIElement::wrap_under_get_rule(window_cf.as_CFTypeRef() as *mut _);

        // Convert to UIElement
        self.ax_element_to_ui_element(&window_element)
    }

    /// Fallback: Try to get any accessible application window
    #[cfg(target_os = "macos")]
    unsafe fn try_get_any_window(&self) -> Result<UIElement> {
        let system_element = AXUIElement::system_wide();

        // Get all running applications
        let apps_cf: CFType = system_element
            .attribute(&AXAttribute::new(&CFString::from_static_string(
                "AXRunningApplications",
            )))
            .map_err(|e| {
                AccessibilityError::Platform(format!("Failed to get running apps: {:?}", e))
            })?;

        if let Some(apps_array) = apps_cf.downcast::<CFArray>() {
            // Try each application until we find one with windows
            for app_ref in apps_array.iter() {
                if let Some(ptr) = app_ref.as_ref() {
                    let app_element = AXUIElement::wrap_under_get_rule((ptr as *const _) as *mut _);

                    // Try to get windows from this app
                    let attr_windows = AXAttribute::new(&CFString::from_static_string("AXWindows"));
                    if let Ok(windows_cf) = app_element.attribute(&attr_windows) {
                        if let Some(windows_array) = windows_cf.downcast::<CFArray>() {
                            if windows_array.len() > 0 {
                                // Get the first window
                                if let Some(window_ref) = windows_array.iter().next() {
                                    if let Some(window_ptr) = window_ref.as_ref() {
                                        let window_element = AXUIElement::wrap_under_get_rule(
                                            (window_ptr as *const _) as *mut _,
                                        );
                                        return self.ax_element_to_ui_element(&window_element);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(AccessibilityError::NoFocusedWindow)
    }

    fn traverse_recursive(
        &self,
        element: &UIElement,
        filter: &ElementFilter,
        results: &mut Vec<UIElement>,
    ) {
        let matches = match filter {
            ElementFilter::All => true,
            ElementFilter::Roles(roles) => roles.contains(&element.role.as_str()),
            ElementFilter::HasActions => !element.actions.is_empty(),
        };

        if matches {
            results.push(element.clone());
        }

        for child in &element.children {
            self.traverse_recursive(child, filter, results);
        }
    }

    #[cfg(target_os = "macos")]
    unsafe fn ax_element_to_ui_element(&self, element: &AXUIElement) -> Result<UIElement> {
        // Get role
        let role_cf: CFType = element
            .attribute(&AXAttribute::new(&CFString::from_static_string(
                kAXRoleAttribute,
            )))
            .map_err(|e| AccessibilityError::Platform(format!("Failed to get role: {:?}", e)))?;

        let role = role_cf
            .downcast::<CFString>()
            .map(|s| s.to_string())
            .ok_or_else(|| AccessibilityError::Platform("Role is not a string".to_string()))?;

        // Get title/label
        let label = element
            .attribute(&AXAttribute::new(&CFString::from_static_string(
                kAXTitleAttribute,
            )))
            .ok()
            .and_then(|v: CFType| {
                if let Some(s) = v.downcast::<CFString>() {
                    Some(s.to_string())
                } else {
                    None
                }
            });

        // Get position and size from AX attributes
        let rect = match (Self::get_ax_position(element), Self::get_ax_size(element)) {
            (Some(pos), Some(size)) => {
                let y = Self::convert_to_top_left_origin(pos.y, size.height);
                Rect::new(pos.x as i32, y, size.width as u32, size.height as u32)
            }
            _ => Rect::new(0, 0, 0, 0), // fallback for elements without position
        };

        // Get actual actions
        let actions = element
            .action_names()
            .map(|arr| arr.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        let id = format!("{:p}", element);
        let mut ui_element = UIElement::new(id, role, label, rect, actions);

        // Try to get children - use raw CFType approach
        let attr_children_cf = AXAttribute::new(&CFString::from_static_string("AXChildren"));
        if let Ok(children_cf) = element.attribute(&attr_children_cf) {
            if let Some(children_array) = children_cf.downcast::<CFArray>() {
                for child_ref in children_array.iter() {
                    if let Some(ptr) = child_ref.as_ref() {
                        let child_ax =
                            AXUIElement::wrap_under_get_rule((ptr as *const _) as *mut _);
                        if let Ok(child) = self.ax_element_to_ui_element(&child_ax) {
                            ui_element.add_child(child);
                        }
                    }
                }
            }
        }

        Ok(ui_element)
    }

    /// AXPositionからCGPointを取得
    #[cfg(target_os = "macos")]
    unsafe fn get_ax_position(element: &AXUIElement) -> Option<CGPoint> {
        let attr = AXAttribute::new(&CFString::from_static_string(kAXPositionAttribute));
        element.attribute(&attr).ok().and_then(|value_cf| {
            let value_ref = value_cf.as_CFTypeRef() as AXValueRef;
            let mut point = MaybeUninit::<CGPoint>::uninit();
            if AXValueGetValue(value_ref, kAXValueTypeCGPoint, point.as_mut_ptr() as *mut _) {
                Some(point.assume_init())
            } else {
                None
            }
        })
    }

    /// AXSizeからCGSizeを取得
    #[cfg(target_os = "macos")]
    unsafe fn get_ax_size(element: &AXUIElement) -> Option<CGSize> {
        let attr = AXAttribute::new(&CFString::from_static_string(kAXSizeAttribute));
        element.attribute(&attr).ok().and_then(|value_cf| {
            let value_ref = value_cf.as_CFTypeRef() as AXValueRef;
            let mut size = MaybeUninit::<CGSize>::uninit();
            if AXValueGetValue(value_ref, kAXValueTypeCGSize, size.as_mut_ptr() as *mut _) {
                Some(size.assume_init())
            } else {
                None
            }
        })
    }

    /// macOS座標系（左下原点）をtop-left originに変換
    #[cfg(target_os = "macos")]
    fn convert_to_top_left_origin(macos_y: f64, height: f64) -> i32 {
        let screen_height = CGDisplay::main().pixels_high() as f64;
        (screen_height - macos_y - height) as i32
    }
}
