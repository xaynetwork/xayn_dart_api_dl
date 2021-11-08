use std::ffi::CString;

use dart_api_dl_sys::{
    Dart_CObject, Dart_CloseNativePort_DL, Dart_NativeMessageHandler_DL, Dart_NewNativePort_DL,
    Dart_Port_DL, Dart_PostCObject_DL, Dart_PostInteger_DL, ILLEGAL_PORT,
};
use thiserror::Error;

use crate::{
    cobject::{ExternCObject, OwnedCObject},
    livecycle::DartRuntime,
    slot::fpslot,
};

impl DartRuntime {
    pub fn native_send_port(&self, id: Dart_Port_DL) -> NativeSendPort {
        // SAFE:
        // - putting in a arbitrary id is safe (but will fail sending)  //TODO: Check
        // - we made sure the dart vm started
        // - we have at a different place a unsafe contract which requires the library
        //   consumer to handle shutdown properly.
        NativeSendPort(id)
    }

    /// Creates a new [`NativeRecvPort`].
    ///
    /// # Safety
    ///
    /// The `handler` muss be sound under given `handle_concurrently` option and
    /// the conditions it's correctly used by dart/this library.
    ///
    pub unsafe fn unsafe_native_recp_port(
        &self,
        name: &str,
        handler: Dart_NativeMessageHandler_DL,
        handle_concurrently: bool,
    ) -> Result<NativeRecvPort, PortManagementFailure> {
        let c_name = CString::new(name).map_err(|_| PortManagementFailure::CreationFailed)?;

        let port = fpslot!(@call_slot Dart_NewNativePort_DL(c_name.as_ptr(), handler, handle_concurrently));

        if port == ILLEGAL_PORT {
            Err(PortManagementFailure::CreationFailed)
        } else {
            Ok(NativeRecvPort(port))
        }
    }

    /// A rust-safe way to creates a new [`NativeRecvPort`].
    pub fn native_recp_port<N>(&self, name: &str) -> Result<NativeRecvPort, PortManagementFailure>
    where
        N: NativeMessageHandler,
    {
        let c_name = CString::new(name).map_err(|_| PortManagementFailure::CreationFailed)?;

        let port = unsafe {
            fpslot!(@call_slot Dart_NewNativePort_DL(c_name.as_ptr(), Some(handle_message::<N>), N::CONCURRENT_HANDLING))
        };

        return if port == ILLEGAL_PORT {
            Err(PortManagementFailure::CreationFailed)
        } else {
            Ok(NativeRecvPort(port))
        };

        unsafe extern "C" fn handle_message<N>(ourself: Dart_Port_DL, data_ref: *mut Dart_CObject)
        where
            N: NativeMessageHandler,
        {
            let recp_port = &NativeRecvPort(ourself);
            ExternCObject::with_pointer(data_ref, |data| N::handle_message(recp_port, data))
        }
    }
}

/// Static rust-safe version of `Dart_NativeMessageHandler_DL`.
pub trait NativeMessageHandler {
    const CONCURRENT_HANDLING: bool;
    fn handle_message(ourself: &NativeRecvPort, data: &ExternCObject);
}

/// Represents a "NativeSendPort" which can be used to send messages to dart.
///
/// # Safety
///
/// Many of the APIs are safe but this relies on following assumptions:
///
/// - The underlying `Dart_CObject` is safe, we make sure it is if only
///   safe code was used.
///
///
#[derive(Debug, Clone)]
pub struct NativeSendPort(Dart_Port_DL);

impl NativeSendPort {
    /// Sends given integer to given port.
    ///
    pub fn post_integer(&self, message: i64) -> Result<(), PortManagementFailure> {
        // SAFE: As long as trying to send to a closed port is safe, which should be
        //       safe for darts security model to work.
        if unsafe { fpslot!(@call_slot Dart_PostInteger_DL(self.0, message)) } {
            Ok(())
        } else {
            Err(PortManagementFailure::PostingFailed(self.clone()))
        }
    }

    /// Sends given [`OwnedCObject`] to given port.
    ///
    /// Like normally for data which is not externally typed a copy of the data is send
    /// over the port and the object stays unchanged (through it might get temp.
    /// modified while being enqueued, which isn't a problem for us to to the
    /// guarantees of `&mut`).
    ///
    /// In case of external typed data it will get send to the client, to avoid
    /// problem and allow auto dropping not send external typed data we set the
    /// type of the [`OwnedCObject`] to `null`.
    ///
    /// If sending fails the external typed data is still be in the [`OwnedCObject`]
    /// and can be reused.
    ///
    ///
    pub fn post_cobject(&self, cobject: &mut OwnedCObject) -> Result<(), PortManagementFailure> {
        // SAFE: As long as `OwnedCObject` was properly constructed and is kept in a sound
        //       sate which is a requirements of it's unsafe interfaces.
        if unsafe { fpslot!(@call_slot Dart_PostCObject_DL(self.0, cobject.as_ptr_mut())) } {
            Ok(())
        } else {
            Err(PortManagementFailure::PostingFailed(self.clone()))
        }
    }
}

#[derive(Debug, Error)]
#[error("Posting message to port (0x{:x}) failed.", _0)]
pub struct PostMessageFailed(Dart_Port_DL);

#[derive(Debug, Clone)]
pub struct NativeRecvPort(Dart_Port_DL);

impl NativeRecvPort {
    /// Closes this [`NativeRecvPort`].
    ///
    pub fn close(self) -> Result<(), PortManagementFailure> {
        // SAFE:
        // - Is save is calling dart functions is safe
        // - and if calling it a bad port id is safe
        //
        // Both should be the case
        if unsafe { fpslot!(@call_slot Dart_CloseNativePort_DL(self.0)) } {
            Ok(())
        } else {
            Err(PortManagementFailure::ClosingFailed(self.clone()))
        }
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PortManagementFailure {
    #[error("Posting message failed on {:?}", _0)]
    PostingFailed(NativeSendPort),
    #[error("Closing port failed on {:?}", _0)]
    ClosingFailed(NativeRecvPort),
    #[error("Creating new recv port failed")]
    CreationFailed,
}
