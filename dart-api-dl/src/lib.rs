use livecycle::InDartRuntime;

//FIXME
pub mod cobject;
pub mod livecycle;
/// All functions in this module can be called from any thread.
pub mod port;
mod slot;

/// All functions in this module can only be called from inside of a dart thread.
mod dart_thread_only {
    // util!(@wrap_slots
    //   F(Dart_IsError, bool, (Dart_Handle handle))
    //   F(Dart_IsApiError, bool, (Dart_Handle handle))
    //   F(Dart_IsUnhandledExceptionError, bool, (Dart_Handle handle))
    //   F(Dart_IsCompilationError, bool, (Dart_Handle handle))
    //   F(Dart_IsFatalError, bool, (Dart_Handle handle))
    //   F(Dart_GetError, const char*, (Dart_Handle handle))
    //   F(Dart_ErrorHasException, bool, (Dart_Handle handle))
    //   F(Dart_ErrorGetException, Dart_Handle, (Dart_Handle handle))
    //   F(Dart_ErrorGetStackTrace, Dart_Handle, (Dart_Handle handle))
    //   F(Dart_NewApiError, Dart_Handle, (const char* error))
    //   F(Dart_NewCompilationError, Dart_Handle, (const char* error))
    //   F(Dart_NewUnhandledExceptionError, Dart_Handle, (Dart_Handle exception))
    //   F(Dart_PropagateError, void, (Dart_Handle handle))
    //
    //   F(Dart_HandleFromPersistent, Dart_Handle, (Dart_PersistentHandle object))
    //   F(Dart_HandleFromWeakPersistent, Dart_Handle, (Dart_WeakPersistentHandle object))
    //   F(Dart_NewPersistentHandle, Dart_PersistentHandle, (Dart_Handle object))
    //   F(Dart_SetPersistentHandle, void, (Dart_PersistentHandle obj1, Dart_Handle obj2))
    //   F(Dart_DeletePersistentHandle, void, (Dart_PersistentHandle object))
    //   F(Dart_NewWeakPersistentHandle, Dart_WeakPersistentHandle, (Dart_Handle object, void* peer, intptr_t external_allocation_size,Dart_HandleFinalizer callback))                                           \
    //   F(Dart_DeleteWeakPersistentHandle, void, (Dart_WeakPersistentHandle object))
    //   F(Dart_UpdateExternalSize, void, (Dart_WeakPersistentHandle object, intptr_t external_allocation_size))
    //   F(Dart_NewFinalizableHandle, Dart_FinalizableHandle, (Dart_Handle object, void* peer, intptr_t external_allocation_size, Dart_HandleFinalizer callback))
    //   F(Dart_DeleteFinalizableHandle, void, (Dart_FinalizableHandle object, Dart_Handle strong_ref_to_object))
    //   F(Dart_UpdateFinalizableExternalSize, void, (Dart_FinalizableHandle object, Dart_Handle strong_ref_to_object, intptr_t external_allocation_size))
    //
    //   F(Dart_Post, bool, (Dart_Port_DL port_id, Dart_Handle object))
    //   F(Dart_NewSendPort, Dart_Handle, (Dart_Port_DL port_id))
    //   F(Dart_SendPortGetId, Dart_Handle, (Dart_Handle port, Dart_Port_DL * port_id))

    //   F(Dart_EnterScope, void, ())
    //   F(Dart_ExitScope, void, ())
    // );
}

impl InDartRuntime {}
