//! macOS TCC (Transparency, Consent, and Control) permission checks.
//!
//! Uses the Accessibility API to check and request accessibility permissions.

use accessibility_sys::AXIsProcessTrusted;
use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;

/// Check if the current process has accessibility permissions.
pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Request accessibility permissions from the user.
///
/// This will open the system prompt asking the user to grant accessibility
/// permissions to this application. Returns `true` if permissions are already
/// granted or if the user grants them (detectable on next call).
pub fn request_trust() -> bool {
    // Check if already trusted
    if is_trusted() {
        return true;
    }

    #[cfg(target_os = "macos")]
    unsafe {
        use accessibility_sys::AXIsProcessTrustedWithOptions;
        use core_foundation::string::CFString;

        // Create options dictionary with kAXTrustedCheckOptionPrompt = true
        let prompt_key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
        let prompt_value = CFBoolean::true_value();
        let options =
            CFDictionary::from_CFType_pairs(&[(prompt_key.as_CFType(), prompt_value.as_CFType())]);

        // Request trust (opens system prompt)
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef());

        // The function returns immediately; we can't detect the user's choice
        // right now. The user needs to grant permissions in System Settings,
        // then restart the app.
        false
    }

    #[cfg(not(target_os = "macos"))]
    false
}
