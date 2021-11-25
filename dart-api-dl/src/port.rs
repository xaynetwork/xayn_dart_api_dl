//! This module contains types and implementations for interacting with send/receive ports.
use std::{
    ffi::{CString, NulError},
    mem::forget,
    ops::Deref,
};

use dart_api_dl_sys::{
    Dart_CObject, Dart_CloseNativePort_DL, Dart_NewNativePort_DL, Dart_PostCObject_DL,
    Dart_PostInteger_DL, ILLEGAL_PORT,
};

use thiserror::Error;

use crate::{
    cobject::{CObject, CObjectType, OwnedCObject},
    lifecycle::fpslot,
    lifecycle::DartRuntime,
    panic::catch_unwind_panic_as_cobject,
    UninitializedFunctionSlot,
};

/// Raw Id of a dart Port.
///
/// Same as `Dart_Port_DL`.
///
// We redefine this as using `cbindgen` with a exported
// type generated by `bindgen` doesn't work. At the same
// time as `dart_api_dl.h` isn't cleanly separated from
// other dart api's we don't want to `include` it into
// the file generated by `bindgen`. There is also the
// problem was in same places `Dart_Port` is used due to
// the non-clear separation.
pub type DartPortId = i64;

// like `Dart_NativeMessageHandler_DL` but not wrapped in an `Option`
type DartNativeMessageHandler =
    unsafe extern "C" fn(dest_port_id: DartPortId, message: *mut Dart_CObject);

impl DartRuntime {
    /// Wraps the port.
    ///
    /// Returns `None` if `port == ILLEGAL_PORT`.
    ///
    /// This is done so because `ILLEGAL_PORT` is in generally
    /// used to indicate both "no port" and "somehow bad port".
    ///
    /// This is safe as sending data to a "invalid" (not yet opened/already closed)
    /// port is safe in dart (and in my understanding must be or it would invalidate
    /// the dart security model).
    pub fn send_port_from_raw(&self, port: DartPortId) -> Option<SendPort> {
        self.send_port_from_raw_with_origin(port, ILLEGAL_PORT)
    }

    /// Wraps the port.
    ///
    /// Returns `None` if `port == ILLEGAL_PORT`.
    /// This is done so because `ILLEGAL_PORT` is in generally
    /// used to indicate both "no port" and "somehow bad port".
    ///
    /// The `origin_id` is in practice nearly always unset (i.e.
    /// set to `ILLEGAL_PORT`). I'm not even sure if it's possible
    /// for it to be set when receiving a [`CObject`] send port message
    /// from dart.
    ///
    pub fn send_port_from_raw_with_origin(
        &self,
        port: DartPortId,
        origin: DartPortId,
    ) -> Option<SendPort> {
        (port != ILLEGAL_PORT).then(|| SendPort { port, origin })
    }

    /// Wrap a raw port id as `NativeRecvPort`.
    ///
    /// The returned type will closed the port when it's dropped and can
    /// be used as a guard.
    pub fn native_recv_port_from_raw(&self, port: DartPortId) -> Option<NativeRecvPort> {
        (port != ILLEGAL_PORT).then(|| {
            NativeRecvPort(SendPort {
                port,
                origin: ILLEGAL_PORT,
            })
        })
    }

    /// Creates a new [`NativeRecvPort`].
    ///
    /// If possible use [`DartRuntime::native_recv_port()`] instead.
    ///
    /// By sending the port id of this port to dart you then can send
    /// messages from dart to rust.
    ///
    /// Which thread will call the handler when is message is received is not
    /// specified and might vary over the lifetime of the port.
    ///
    /// If `handle_concurrently` multiple threads might call the handler at the
    /// same time with different messages.
    ///
    /// The `*mut Dart_CObject` passed in to the handler not a owned reference,
    /// dart will free the memory related to the passed in `CObject` after the
    /// handle completes. Mutating the `CObject` will not change this. But that
    /// fact it's not clearly documented either but derived from dart VM
    /// implementation details. As such you should treat it as a `&`-reference.
    ///
    /// Dart should never call the handler with a nullptr.
    ///
    /// # Safety
    ///
    /// - The `handler` must be safe to call with valid parameters.
    /// - The handler must not panic.
    /// - The handler must be safe to use under given `handle_concurrently` option.
    ///
    unsafe fn unsafe_native_recp_port(
        self,
        name: &str,
        handler: DartNativeMessageHandler,
        handle_concurrently: bool,
    ) -> Result<NativeRecvPort, PortCreationFailed> {
        //TODO Result
        let c_name = CString::new(name)?;

        let port = unsafe {
            fpslot!(@call Dart_NewNativePort_DL(c_name.as_ptr(), Some(handler), handle_concurrently))?
        };

        self.native_recv_port_from_raw(port)
            .ok_or(PortCreationFailed::DartFailed)
    }

    /// A rust-safe way to creates a new [`NativeRecvPort`].
    ///
    /// Take a look at the [`NativeMessageHandler`] trait for details.
    ///
    /// # Errors
    ///
    /// - If the name contained a nul byte.
    /// - If the port returned by dart is the `ILLEGAL_PORT`.
    /// - (If the api is not initialized, but you can only reach that
    ///   case with unsound code.)
    pub fn native_recv_port<N>(&self) -> Result<NativeRecvPort, PortCreationFailed>
    //TODO Result
    where
        N: NativeMessageHandler,
    {
        //SAFE: The handle_message wrapper provides a safe abstraction
        return unsafe {
            self.unsafe_native_recp_port(N::NAME, handle_message::<N>, N::CONCURRENT_HANDLING)
        };

        unsafe extern "C" fn handle_message<N>(ourself: DartPortId, data_ref: *mut Dart_CObject)
        where
            N: NativeMessageHandler,
        {
            if let Ok(rt) = DartRuntime::instance() {
                if let Some(port) = rt.native_recv_port_from_raw(ourself) {
                    unsafe {
                        CObject::with_pointer(data_ref, |data| {
                            catch_unwind_panic_as_cobject(
                                data,
                                |data| N::handle_message(rt, &port, data),
                                |data, panic_obj| N::handle_panic(rt, &port, data, panic_obj),
                            );
                        });
                    };
                    forget(port);
                }
            }
        }
    }
}

/// The creating of a native receiver port failed.
#[derive(Debug, Error)]
pub enum PortCreationFailed {
    /// The name of the port contained a null byte.
    #[error("The name of the port contained a null byte.")]
    NulInName,
    /// Creating the port failed through dart.
    #[error("Calling Dart_NewNativePort_DL failed")]
    DartFailed,
    /// A supposedly unreachable invariant was reached.
    ///
    /// This likely implies the violation of a unsafe contract
    /// or a unsound assumptions in a unsafe function/block.
    ///
    /// Normally we would prefer to panic, but panics in FFI
    /// are a problem so we have this error variant instead.
    #[error("invariant broken: {}", _0)]
    Unreachable(#[from] UninitializedFunctionSlot),
}

impl From<NulError> for PortCreationFailed {
    fn from(_: NulError) -> Self {
        PortCreationFailed::NulInName
    }
}

/// Static rust-safe version of `Dart_NativeMessageHandler_DL`.
pub trait NativeMessageHandler {
    /// If `false` dart will only call the handler from one thread at a time.
    ///
    /// It still will call it from different threads over time, just not at the same time.
    const CONCURRENT_HANDLING: bool;

    /// A Name used to setup the port.
    ///
    /// This must not contain a `0` byte.
    ///
    /// The name is mainly used for debugging purpose.
    const NAME: &'static str;

    /// Called when handling a message.
    ///
    /// `ourself` can be used to close the port (through you should not rely on
    /// closing happening immediately, dart might/or might not still call it with
    /// already enqueued messages, closing might not be instantly either, do not
    /// rely on "currently" observed behavior/Dart VM code).
    fn handle_message(rt: DartRuntime, ourself: &NativeRecvPort, data: &mut CObject);

    /// Called if [`NativeMessageHandler::handle_message()`] failed.
    ///
    /// It's called with the same object as `handle_message`, as well as a panic
    /// converted to a [`OwnedCObject`] this allows sending back the panic through
    /// a port in the original message.
    ///
    /// This is not calling while `panicing`, as such this will not trigger a
    /// "double-panic" induced abort. Through it also can't do anything with the
    /// panic so it will simply do nothing.
    fn handle_panic(
        rt: DartRuntime,
        ourself: &NativeRecvPort,
        data: &mut CObject,
        panic: &mut OwnedCObject,
    );
}

/// Represents the send port of a [`NativeSendPort`] which can be used to send messages to dart.
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
    port: DartPortId,
    // Not sure what it is used for. In nearly all
    // cases this is equal to `ILLEGAL_PORT` and you
    // always can set it to `ILLEGAL_PORT`.
    origin: DartPortId,
}

impl SendPort {
    /// Return the underlying port ids of this `SendPort`.
    ///
    /// The first id is the port id and the second one the
    /// origin id and as such most times equals to `ILLEGAL_PORT`.
    pub fn as_raw(&self) -> (DartPortId, DartPortId) {
        (self.port, self.origin)
    }

    /// Sends given integer to given port.
    ///
    /// This will use `Dart_PostInteger_DL` instead of creating
    /// a integer `CObject`.
    ///
    /// # Errors
    ///
    /// If posting the message failed.
    pub fn post_integer(&self, message: i64) -> Result<(), PortPostMessageFailed> {
        // SAFE: As long as trying to send to a closed port is safe, which should be
        //       safe for darts security model to work.
        if unsafe { fpslot!(@call Dart_PostInteger_DL(self.port, message))? } {
            Ok(())
        } else {
            Err(PortPostMessageFailed)
        }
    }

    /// This will call [`SendPort.post_cobject_mut()`] and then drop the `cobject`.
    ///
    /// See [`SendPort.post_cobject_mut()`] for more details.
    ///
    /// # Errors
    ///
    /// If posting the message failed.
    pub fn post_cobject(&self, mut cobject: OwnedCObject) -> Result<(), PortPostMessageFailed> {
        self.post_cobject_mut(&mut cobject)
    }

    /// Sends given [`ExternalCObject`] to given port.
    ///
    /// Like in dart for data which is not externally typed a copy of the data is send
    /// over the port and the object stays unchanged (through it might get temp.
    /// modified while being enqueued, which isn't a problem for us to to the
    /// guarantees of `&mut`).
    ///
    /// In case of external typed data it will get send (moved) to the client,
    /// to avoid accidentally dropping it when [`ExternalCObject`] is dropped
    /// the [`ExternalCObject`] is set to represent null, iff the sending
    /// succeeded.
    ///
    /// If sending fails the cobject will stay unchanged.
    ///
    /// # Errors
    ///
    /// If posting the message failed this will error.
    ///
    pub fn post_cobject_mut(&self, cobject: &mut CObject) -> Result<(), PortPostMessageFailed> {
        let need_nulling = cobject.r#type() == Ok(CObjectType::ExternalTypedData);
        // SAFE: As long as `OwnedCObject` was properly constructed and is kept in a sound
        //       state (which is a requirement of it's unsafe interfaces).
        if unsafe { fpslot!(@call Dart_PostCObject_DL(self.port, cobject.as_ptr_mut()))? } {
            if need_nulling {
                cobject.set_to_null();
            }
            Ok(())
        } else {
            Err(PortPostMessageFailed)
        }
    }

    //TODO post_slice(&mut [&mut ExternalCObject]) which doesn't allocate a vec or box the objects
}

/// Handler for a native receiver port.
///
/// If this handler is dropped the port is closed.
#[derive(Debug)]
pub struct NativeRecvPort(SendPort);

impl NativeRecvPort {
    /// Prevent drop form closing this type.
    pub fn leak(self) -> SendPort {
        let port = *self;
        forget(self);
        port
    }
}

impl Drop for NativeRecvPort {
    fn drop(&mut self) {
        // SAFE:
        // - Is save is calling dart functions is safe
        // - and if calling it a bad port id is safe
        //
        // Both should be the case
        let _ = unsafe { fpslot!(@call Dart_CloseNativePort_DL(self.as_raw().0)) };
    }
}

impl Deref for NativeRecvPort {
    type Target = SendPort;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Posting a message on a port failed.
#[derive(Debug, Error)]
#[error("Posting message failed.")]
pub struct PortPostMessageFailed;

impl From<UninitializedFunctionSlot> for PortPostMessageFailed {
    fn from(_: UninitializedFunctionSlot) -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use dart_api_dl_sys::{Dart_NativeMessageHandler_DL, Dart_Port_DL};
    use static_assertions::{assert_impl_all, assert_type_eq_all};

    use super::*;

    #[test]
    fn test_static_assertions() {
        assert_impl_all!(SendPort: Send, Sync, Copy, Clone);
        assert_impl_all!(NativeRecvPort: Send, Sync);

        assert_type_eq_all!(Dart_Port_DL, DartPortId, i64);
        assert_type_eq_all!(
            Option<DartNativeMessageHandler>,
            Dart_NativeMessageHandler_DL
        );
    }
}
