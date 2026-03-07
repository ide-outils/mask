macro_rules! set {
    ($builder:ident.$method:ident = $value:expr) => {
        *$builder = std::mem::take($builder).$method($value)
    };
}

#[macro_export]
macro_rules! mask_read {
    ($command:ident) => {
        $command
            .get::<::mask_types::Mask>()
            .unwrap()
            .read()
            .unwrap()
    };
}

#[macro_export]
macro_rules! mask_write {
    ($command:ident) => {
        $command
            .get::<::mask_types::Mask>()
            .unwrap()
            .write()
            .unwrap()
    };
}

pub(crate) use mask_read;
pub(crate) use mask_write;
pub(crate) use set;

#[cfg(all(test, not(feature = "debug_test")))]
pub(crate) mod tests {
    pub(crate) use assert_eq as file_assert_eq;
}
#[cfg(all(test, feature = "debug_test"))]
pub(crate) mod tests {
    macro_rules! file_assert_eq {
        ($expected:expr, $result:expr) => {
            debug_assert_eq!($expected, $result, "")
        };
        ($expected:expr, $result:expr, $msg:literal) => {
            let expected = $expected;
            let result = $result;
            let mut success = true;
            if expected != result {
                use std::fs;
                fs::write("./_expected", format!("{expected:#?}")).unwrap();
                fs::write("./_result", format!("{result:#?}")).unwrap();
                success = false;
            }
            // println!("{expected:#?}\n\n\n{result:#?}");
            assert!(success, $msg);
        };
    }
    pub(crate) use file_assert_eq;
}
