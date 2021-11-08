macro_rules! __fpslot {
    (@call_slot $slot:ident ( $($pn:expr),* )) => (
        match $slot {
            Some(func) => func($($pn),*),
            None => panic!("Function pointer slot was not initialized: {}", stringify!($slot)),
        }
    );
}

pub(crate) use __fpslot as fpslot;
