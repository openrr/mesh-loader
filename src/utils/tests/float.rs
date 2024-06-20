use super::*;

fn leading_str() -> String {
    (if fastrand::bool() {
        "+".to_owned()
    } else {
        String::new()
    }) + &"0".repeat(fastrand::u8(..) as usize)
}

// Asserts that `$a` and `$b` have performed equivalent operations.
macro_rules! assert_float_op_eq {
    ($a:expr, $b:expr $(, $($tt:tt)*)?) => {{
        // See also:
        // - https://github.com/rust-lang/unsafe-code-guidelines/issues/237.
        // - https://github.com/rust-lang/portable-simd/issues/39.
        let a = $a;
        let b = $b;
        if a.is_nan() && b.is_nan() // don't check sign of NaN: https://github.com/rust-lang/rust/issues/55131
            || a.is_infinite()
                && b.is_infinite()
                && a.is_sign_positive() == b.is_sign_positive()
                && a.is_sign_negative() == b.is_sign_negative()
        {
            // ok
        } else {
            assert_eq!(a, b $(, $($tt)*)?);
        }
    }};
}

macro_rules! quickcheck_float {
    ($name:ident, $ty:ident) => {
        mod $name {
            use super::*;
            ::quickcheck::quickcheck! {
                fn parse_str(x: String) -> bool {
                    match ($ty::parse(x.as_bytes()), x.parse::<$ty>().ok()) {
                        (Some(a), Some(b)) => assert_float_op_eq!(a, b, "{x}"),
                        (a, b) => assert_eq!(a, b, "{x}"),
                    }
                    true
                }
                fn parse_valid(x: $ty) -> bool {
                    assert_float_op_eq!(
                        $ty::parse(x.to_string().as_bytes()).unwrap(),
                        x
                    );
                    if !x.is_nan() && !x.is_infinite() && !x.is_sign_negative() {
                        assert_float_op_eq!(
                            $ty::parse((leading_str() + &x.to_string()).as_bytes()).unwrap(),
                            x
                        );
                    }
                    true
                }
            }
        }
    };
}

quickcheck_float!(test_f64, f64);
quickcheck_float!(test_f32, f32);
