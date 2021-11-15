use std::panic::{AssertUnwindSafe, UnwindSafe};

use crate::cobject::{ExternCObject, OwnedCObject};

/// If given function panics call the panic handler.
///
/// The panic is converted to a `OwnedCObject`  and
/// passed to the panic handler.
///
/// If the panic handler panics it's caught and ignored.
///
//FIXME find a better name
pub fn catch_unwind_panic_as_cobject<F, P>(obj: &mut ExternCObject, func: F, on_panic: P)
where
    F: UnwindSafe + FnOnce(&mut ExternCObject),
    P: UnwindSafe + FnOnce(&mut ExternCObject, &mut OwnedCObject),
{
    let a_obj = AssertUnwindSafe(&mut *obj);
    let err = match std::panic::catch_unwind(|| func(fix(a_obj))) {
        Ok(val) => return val,
        Err(err) => err,
    };

    let mut err = if let Some(err) = err.downcast_ref::<String>() {
        OwnedCObject::string_lossy(err)
    } else if let Some(err) = err.downcast_ref::<&'static str>() {
        OwnedCObject::string_lossy(err)
    } else {
        OwnedCObject::string_lossy("panic of unsupported type")
    };

    let a_obj = AssertUnwindSafe(obj);
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| on_panic(fix(a_obj), &mut err)));

    //TODO log
}

// Rust2021 is too clever
fn fix<T>(a: AssertUnwindSafe<T>) -> T {
    a.0
}

#[cfg(test)]
mod tests {
    use crate::DartRuntime;

    use super::*;

    #[test]
    fn test_catch_panic_to_cobject() {
        //Safe: Only because we do not call any dart dl functions, but
        //      we do create abstractions which make it "safe" to call
        //      them, even through it here isn't.
        let rt = unsafe { DartRuntime::instance_unchecked() };
        let mut null = OwnedCObject::null();

        let mut res = None;
        let mres = AssertUnwindSafe(&mut res);
        catch_unwind_panic_as_cobject(
            &mut null,
            |_| panic!("hy there"),
            move |_, obj| {
                *fix(mres) = obj.as_string(rt).map(ToOwned::to_owned);
            },
        );
        assert_eq!(res, Some("hy there".to_owned()));

        let mut res = None;
        let res_ref = AssertUnwindSafe(&mut res);
        catch_unwind_panic_as_cobject(
            &mut null,
            |_| panic!("hy {}", "there"),
            move |_, obj| {
                *fix(res_ref) = obj.as_string(rt).map(ToOwned::to_owned);
            },
        );
        assert_eq!(res, Some("hy there".to_owned()));
    }

    #[test]
    fn test_panic_in_panic_handler_does_not_propagate() {
        let mut null = OwnedCObject::null();
        catch_unwind_panic_as_cobject(&mut null, |_| panic!(), |_, _| panic!());
    }

    // Rust 2021 is to clever and want's to only borrow the res.0 by the closure ;=)
    fn fix<T>(res: AssertUnwindSafe<T>) -> T {
        res.0
    }
}
