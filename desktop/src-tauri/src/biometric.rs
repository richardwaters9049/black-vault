use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BiometricStatus {
    pub available: bool,
    pub kind: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BiometricAuthResult {
    pub authenticated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BiometricError {
    pub code: &'static str,
    pub message: &'static str,
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{BiometricAuthResult, BiometricError, BiometricStatus};
    use block2::RcBlock;
    use objc2::runtime::Bool;
    use objc2_foundation::{NSError, NSString};
    use objc2_local_authentication::{LABiometryType, LAContext, LAPolicy};
    use std::{sync::mpsc, time::Duration};

    fn context() -> objc2::rc::Retained<LAContext> {
        // SAFETY: LAContext::new creates an owned Objective-C LocalAuthentication context.
        unsafe { LAContext::new() }
    }

    pub fn status() -> Result<BiometricStatus, BiometricError> {
        let context = context();
        // SAFETY: The retained context is alive for the complete framework call.
        let available = unsafe {
            context
                .canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthenticationWithBiometrics)
                .is_ok()
        };
        // canEvaluatePolicy populates biometryType for the same live context.
        let kind = if available {
            // SAFETY: Reading biometryType is valid after the preflight call above.
            match unsafe { context.biometryType() } {
                LABiometryType::TouchID => "touch_id",
                LABiometryType::FaceID => "face_id",
                LABiometryType::OpticID => "optic_id",
                _ => "biometric",
            }
        } else {
            "none"
        };
        Ok(BiometricStatus {
            available,
            kind: kind.to_owned(),
            platform: "macos".to_owned(),
        })
    }

    pub fn authenticate() -> Result<BiometricAuthResult, BiometricError> {
        let context = context();
        // SAFETY: The retained context is alive for the complete framework call.
        if unsafe {
            context
                .canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthenticationWithBiometrics)
                .is_err()
        } {
            return Err(BiometricError {
                code: "biometric_unavailable",
                message: "Touch ID is unavailable or is not enrolled on this Mac.",
            });
        }

        let empty = NSString::from_str("");
        let reason = NSString::from_str("Authenticate to Black Vault");
        // Hide the system password fallback because this command promises biometric-only authentication.
        // SAFETY: NSString values and the retained context outlive these property calls.
        unsafe { context.setLocalizedFallbackTitle(Some(&empty)) };

        let (sender, receiver) = mpsc::sync_channel(1);
        let reply = RcBlock::new(move |success: Bool, _error: *mut NSError| {
            let _ = sender.send(success.as_bool());
        });
        // SAFETY: The block is Send, the context/reason stay alive while waiting, and Apple invokes
        // the callback on its private authentication queue.
        unsafe {
            context.evaluatePolicy_localizedReason_reply(
                LAPolicy::DeviceOwnerAuthenticationWithBiometrics,
                &reason,
                &reply,
            )
        };

        match receiver.recv_timeout(Duration::from_secs(60)) {
            Ok(true) => Ok(BiometricAuthResult {
                authenticated: true,
            }),
            Ok(false) => Err(BiometricError {
                code: "biometric_failed",
                message: "Touch ID authentication was cancelled or unsuccessful.",
            }),
            Err(_) => {
                // SAFETY: The context is still retained and may be invalidated before returning.
                unsafe { context.invalidate() };
                Err(BiometricError {
                    code: "biometric_timeout",
                    message: "Touch ID authentication timed out.",
                })
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::{BiometricAuthResult, BiometricError, BiometricStatus};

    pub fn status() -> Result<BiometricStatus, BiometricError> {
        Ok(BiometricStatus {
            available: false,
            kind: "none".to_owned(),
            platform: std::env::consts::OS.to_owned(),
        })
    }

    pub fn authenticate() -> Result<BiometricAuthResult, BiometricError> {
        Err(BiometricError {
            code: "biometric_unsupported",
            message: "Biometric vault authentication is not available on this platform yet.",
        })
    }
}

#[tauri::command]
pub fn get_biometric_status() -> Result<BiometricStatus, BiometricError> {
    platform::status()
}

#[tauri::command]
pub fn authenticate_biometric() -> Result<BiometricAuthResult, BiometricError> {
    platform::authenticate()
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(not(target_os = "macos"))]
    fn unsupported_platform_reports_unavailable_without_authenticating() {
        let status = super::get_biometric_status().expect("status should be queryable");
        assert!(!status.available);
        assert_eq!(status.kind, "none");
        assert!(super::authenticate_biometric().is_err());
    }
}
