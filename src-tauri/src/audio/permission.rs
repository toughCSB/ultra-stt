use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicrophoneAuthorization {
    NotDetermined,
    Restricted,
    Denied,
    Authorized,
    Unknown,
}

#[cfg(any(target_os = "macos", test))]
fn from_macos_status(status: isize) -> MicrophoneAuthorization {
    match status {
        0 => MicrophoneAuthorization::NotDetermined,
        1 => MicrophoneAuthorization::Restricted,
        2 => MicrophoneAuthorization::Denied,
        3 => MicrophoneAuthorization::Authorized,
        _ => MicrophoneAuthorization::Unknown,
    }
}

pub fn authorization_status() -> Result<MicrophoneAuthorization> {
    #[cfg(target_os = "macos")]
    {
        platform::authorization_status()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(MicrophoneAuthorization::Authorized)
    }
}

fn ensure_authorized_with(
    status: MicrophoneAuthorization,
    request: impl FnOnce() -> Result<bool>,
) -> Result<()> {
    match status {
        MicrophoneAuthorization::Authorized => Ok(()),
        MicrophoneAuthorization::NotDetermined if request()? => Ok(()),
        MicrophoneAuthorization::Restricted => {
            bail!("Microphone access is restricted on this Mac")
        }
        MicrophoneAuthorization::Unknown => {
            bail!("Could not determine microphone permission on this Mac")
        }
        _ => bail!(
            "Microphone permission is required. Enable Ultra STT in System Settings > Privacy & Security > Microphone, then try recording again."
        ),
    }
}

pub fn ensure_authorized() -> Result<()> {
    let status = authorization_status()?;
    tracing::info!("Microphone authorization: {:?}", status);
    ensure_authorized_with(status, || {
        #[cfg(target_os = "macos")]
        {
            platform::request_access()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(true)
        }
    })
}

#[cfg(target_os = "macos")]
mod platform {
    use anyhow::{anyhow, Result};
    use block2::RcBlock;
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject, Bool};
    use std::sync::mpsc;
    use std::time::Duration;

    use super::{from_macos_status, MicrophoneAuthorization};

    #[link(name = "AVFoundation", kind = "framework")]
    extern "C" {
        static AVMediaTypeAudio: *const AnyObject;
    }

    fn capture_device_class() -> Result<&'static AnyClass> {
        AnyClass::get("AVCaptureDevice").ok_or_else(|| anyhow!("AVCaptureDevice is unavailable"))
    }

    pub(super) fn authorization_status() -> Result<MicrophoneAuthorization> {
        let capture_device = capture_device_class()?;
        let status: isize =
            unsafe { msg_send![capture_device, authorizationStatusForMediaType: AVMediaTypeAudio] };
        Ok(from_macos_status(status))
    }

    pub(super) fn request_access() -> Result<bool> {
        let capture_device = capture_device_class()?;
        let (sender, receiver) = mpsc::channel();
        let completion = RcBlock::new(move |granted: Bool| {
            let _ = sender.send(granted.as_bool());
        });
        let _: () = unsafe {
            msg_send![capture_device, requestAccessForMediaType: AVMediaTypeAudio completionHandler: &*completion]
        };
        receiver.recv_timeout(Duration::from_secs(25)).map_err(|_| {
            anyhow!("Microphone permission request timed out. Allow Ultra STT in the macOS prompt, then try recording again.")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn microphone_status_uses_avfoundation_order_not_speech_recognition_order() {
        assert_eq!(from_macos_status(1), MicrophoneAuthorization::Restricted);
        assert_eq!(from_macos_status(2), MicrophoneAuthorization::Denied);
        assert_eq!(from_macos_status(3), MicrophoneAuthorization::Authorized);
        assert_eq!(from_macos_status(99), MicrophoneAuthorization::Unknown);
    }

    #[test]
    fn existing_grant_never_prompts_again() {
        ensure_authorized_with(MicrophoneAuthorization::Authorized, || {
            panic!("an authorized microphone must not prompt")
        })
        .unwrap();
    }

    #[test]
    fn undetermined_permission_waits_for_the_users_answer() {
        assert!(
            ensure_authorized_with(MicrophoneAuthorization::NotDetermined, || Ok(true)).is_ok()
        );
        assert!(
            ensure_authorized_with(MicrophoneAuthorization::NotDetermined, || Ok(false)).is_err()
        );
    }

    #[test]
    fn refused_permission_never_opens_capture_or_prompts_again() {
        for status in [
            MicrophoneAuthorization::Denied,
            MicrophoneAuthorization::Restricted,
            MicrophoneAuthorization::Unknown,
        ] {
            assert!(ensure_authorized_with(status, || panic!("must not prompt")).is_err());
        }
    }

    #[test]
    fn permission_request_failure_does_not_start_capture() {
        let error = ensure_authorized_with(MicrophoneAuthorization::NotDetermined, || {
            bail!("request timed out")
        })
        .unwrap_err();
        assert_eq!(error.to_string(), "request timed out");
    }
}
