pub mod cobject;
// pub mod handle;
mod lifecycle;
pub mod port;
mod slot;

pub use lifecycle::*;

//TODO (all with InDartRuntime)
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

//TODO
//   F(Dart_IsApiError, bool, (Dart_Handle handle))
//   F(Dart_IsUnhandledExceptionError, bool, (Dart_Handle handle))
//   F(Dart_IsCompilationError, bool, (Dart_Handle handle))
//   // Do we need to propagate fatal errors? If so it's a problem.
//   F(Dart_IsFatalError, bool, (Dart_Handle handle))
//   F(Dart_ErrorHasException, bool, (Dart_Handle handle))
//   // Uh, we can get a handle to an exception/stack trace but we can't really do
//   // anything with it. Same for the StackTrace.
//   F(Dart_ErrorGetException, Dart_Handle, (Dart_Handle handle))
//   F(Dart_ErrorGetStackTrace, Dart_Handle, (Dart_Handle handle))
//   // That can make sense
//   F(Dart_NewApiError, Dart_Handle, (const char* error))
//   // Why?
//   F(Dart_NewCompilationError, Dart_Handle, (const char* error))
//   // You can't really get a dart exception, except is you already have an
//   // error with an exception.
//   F(Dart_NewUnhandledExceptionError, Dart_Handle, (Dart_Handle exception))
//   //supper problematic as it uses setjmp/longjmp
//   F(Dart_PropagateError, void, (Dart_Handle handle))

//TODO InDartIsolate.with_scope
//   F(Dart_EnterScope, void, ())
//   F(Dart_ExitScope, void, ())
