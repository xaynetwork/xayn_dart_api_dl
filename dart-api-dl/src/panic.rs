// Copyright 2021 Xayn AG
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::panic::{AssertUnwindSafe, UnwindSafe};

use crate::cobject::{CObject, CObjectMut};

/// If given function panics call the panic handler.
///
/// The panic is converted to a `CObject`  and
/// passed to the panic handler.
///
/// If the panic handler panics it's caught and ignored.
pub(crate) fn catch_unwind_panic_as_cobject<F, P>(mut obj: CObjectMut<'_>, func: F, on_panic: P)
where
    F: UnwindSafe + FnOnce(CObjectMut<'_>),
    P: UnwindSafe + FnOnce(CObjectMut<'_>, CObject),
{
    let a_obj = AssertUnwindSafe(obj.reborrow());
    let err = match std::panic::catch_unwind(|| func(fix(a_obj))) {
        Ok(()) => return,
        Err(err) => err,
    };

    let err = if let Some(err) = err.downcast_ref::<String>() {
        CObject::string_lossy(err)
    } else if let Some(err) = err.downcast_ref::<&'static str>() {
        CObject::string_lossy(err)
    } else {
        CObject::string_lossy("panic of unsupported type")
    };

    let a_obj = AssertUnwindSafe(obj);
    if std::panic::catch_unwind(AssertUnwindSafe(|| on_panic(fix(a_obj), err))).is_err() {
        //TODO log
    }
}

// Rust2021 is too clever
fn fix<T>(v: AssertUnwindSafe<T>) -> T {
    v.0
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
        let mut null = CObject::null();

        let mut res = None;
        let a_res = AssertUnwindSafe(&mut res);
        catch_unwind_panic_as_cobject(
            null.as_ref(),
            |_| panic!("hy there"),
            move |_, mut obj| {
                *fix(a_res) = obj.as_ref().as_string(rt).map(ToOwned::to_owned);
            },
        );
        assert_eq!(res, Some("hy there".to_owned()));

        let mut res = None;
        let res_ref = AssertUnwindSafe(&mut res);
        catch_unwind_panic_as_cobject(
            null.as_ref(),
            |_| panic!("hy {}", "there"),
            move |_, mut obj| {
                *fix(res_ref) = obj.as_ref().as_string(rt).map(ToOwned::to_owned);
            },
        );
        assert_eq!(res, Some("hy there".to_owned()));
    }

    #[test]
    fn test_panic_in_panic_handler_does_not_propagate() {
        let mut null = CObject::null();
        catch_unwind_panic_as_cobject(null.as_ref(), |_| panic!(), |_, _| panic!());
    }

    // Rust 2021 is to clever and want's to only borrow the res.0 by the closure ;=)
    fn fix<T>(res: AssertUnwindSafe<T>) -> T {
        res.0
    }
}
