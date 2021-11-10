use std::{ffi::CString, ops::Deref};

use dart_api_dl_sys::{
    Dart_CObject, Dart_CloseNativePort_DL, Dart_NativeMessageHandler_DL, Dart_NewNativePort_DL,
    Dart_Port_DL, Dart_PostCObject_DL, Dart_PostInteger_DL, ILLEGAL_PORT,
};

use thiserror::Error;

use crate::{
    cobject::{ExternCObject, OwnedCObject},
    lifecycle::DartRuntime,
    slot::fpslot,
};

impl DartRuntime {
    /// Wraps the port.
    ///
    /// Fails is `port == ILLEGAL_PORT`.
    ///
    /// # Safety
    ///
    /// The caller must make sure `port` refers
    /// to a valid port we are allowed to send
    /// messages to.
    ///
    pub unsafe fn send_port_from_raw(
        &self,
        port: Dart_Port_DL,
        origin: Dart_Port_DL,
    ) -> Result<SendPort, PortCreatingFailed> {
        if port == ILLEGAL_PORT {
            Err(PortCreatingFailed)
        } else {
            Ok(SendPort {
                port,
                origin: (origin != ILLEGAL_PORT).then(|| origin),
            })
        }
    }

    /// Wrap a port id as `NativeRecvPort`.
    ///
    /// This closed the port when this wrapper is dropped.
    ///
    /// # Safety
    ///
    /// - the port must be a native port
    /// - we must be allowed to close the native port without
    ///   causing safety issue
    pub unsafe fn native_recv_port_from_raw(
        &self,
        port: Dart_Port_DL
    ) -> Result<NativeRecvPort, PortCreatingFailed> {
        if port == ILLEGAL_PORT {
            Err(PortCreatingFailed)
        } else {
            Ok(NativeRecvPort(SendPort {
                port,
                origin: None,
            }))
        }
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
    ) -> Result<NativeRecvPort, PortCreatingFailed> {
        let c_name = CString::new(name).map_err(|_| PortCreatingFailed)?;

        let port =
            fpslot!(@call Dart_NewNativePort_DL(c_name.as_ptr(), handler, handle_concurrently));

        self.native_recv_port_from_raw(port)
    }

    /// A rust-safe way to creates a new [`NativeRecvPort`].
    pub fn native_recp_port<N>(&self, name: &str) -> Result<NativeRecvPort, PortCreatingFailed>
    where
        N: NativeMessageHandler,
    {
        let c_name = CString::new(name).map_err(|_| PortCreatingFailed)?;

        let port = unsafe {
            fpslot!(@call Dart_NewNativePort_DL(c_name.as_ptr(), Some(handle_message::<N>), N::CONCURRENT_HANDLING))
        };

        return if port == ILLEGAL_PORT {
            Err(PortCreatingFailed)
        } else {
            unsafe { self.native_recv_port_from_raw(port) }
        };

        unsafe extern "C" fn handle_message<N>(ourself: Dart_Port_DL, data_ref: *mut Dart_CObject)
        where
            N: NativeMessageHandler,
        {
            if let Some(port) = DartRuntime::instance().ok().and_then(|rt| rt.native_recv_port_from_raw(ourself).ok()) {
                ExternCObject::with_pointer(data_ref, |data| N::handle_message(&port, data))
            } else {
                // shouldn't happen but we can't panic either
            }
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
#[derive(Debug, Clone, Copy)]
pub struct SendPort {
    port: Dart_Port_DL,
    origin: Option<Dart_Port_DL>,
}

impl SendPort {

    pub fn as_raw(&self) -> Dart_Port_DL {
        self.port
    }

    /// Sends given integer to given port.
    ///
    pub fn post_integer(&self, message: i64) -> Result<(), PortPostMessageFailed> {
        // SAFE: As long as trying to send to a closed port is safe, which should be
        //       safe for darts security model to work.
        if unsafe { fpslot!(@call Dart_PostInteger_DL(self.port, message)) } {
            Ok(())
        } else {
            Err(PortPostMessageFailed)
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
    pub fn post_cobject(&self, cobject: &mut OwnedCObject) -> Result<(), PortPostMessageFailed> {
        // SAFE: As long as `OwnedCObject` was properly constructed and is kept in a sound
        //       sate which is a requirements of it's unsafe interfaces.
        if unsafe { fpslot!(@call Dart_PostCObject_DL(self.port, cobject.as_ptr_mut())) } {
            Ok(())
        } else {
            Err(PortPostMessageFailed)
        }
    }
}

/// Handler for a native receiver port.
///
/// If this handler is dropped the port is closed.
#[derive(Debug)]
pub struct NativeRecvPort(SendPort);

impl Drop for NativeRecvPort {
    fn drop(&mut self) {
        // SAFE:
        // - Is save is calling dart functions is safe
        // - and if calling it a bad port id is safe
        //
        // Both should be the case
        unsafe { fpslot!(@call Dart_CloseNativePort_DL(self.as_raw())) };
    }
}

impl Deref for NativeRecvPort {
    type Target = SendPort;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Error)]
#[error("Posting message failed.")]
pub struct PortPostMessageFailed;

#[derive(Debug, Error)]
#[error("Posting message failed.")]
pub struct PortCreatingFailed;
