use super::*;

#[test]
fn max_digit_count() {
    // assert_eq!(max_digit_count!(u128), 39);
    assert_eq!(max_digit_count!(u64), 20);
    assert_eq!(max_digit_count!(u32), 10);
    assert_eq!(max_digit_count!(u16), 5);
    assert_eq!(max_digit_count!(u8), 3);
    // assert_eq!(max_digit_count!(i128), 39);
    assert_eq!(max_digit_count!(i64), 19);
    assert_eq!(max_digit_count!(i32), 10);
    assert_eq!(max_digit_count!(i16), 5);
    assert_eq!(max_digit_count!(i8), 3);
}

fn leading_str() -> String {
    (if fastrand::bool() {
        "+".to_owned()
    } else {
        String::new()
    }) + &"0".repeat(fastrand::u8(..) as usize)
}

#[test]
fn uint() {
    // assert_eq!(
    //     u128::parse_partial(u128::MAX.to_string().as_bytes())
    //         .unwrap()
    //         .0,
    //     u128::MAX
    // );
    // assert_eq!(
    //     u128::parse_partial((leading_str() + &u128::MAX.to_string()).as_bytes())
    //         .unwrap()
    //         .0,
    //     u128::MAX
    // );
    // assert_eq!(
    //     u128::parse_partial(i128::MAX.to_string().as_bytes())
    //         .unwrap()
    //         .0,
    //     i128::MAX as _
    // );
    assert_eq!(
        u64::parse_partial(u64::MAX.to_string().as_bytes())
            .unwrap()
            .0,
        u64::MAX
    );
    assert_eq!(
        u64::parse_partial(10000000000000000000u64.to_string().as_bytes())
            .unwrap()
            .0,
        10000000000000000000u64
    );
    assert_eq!(
        u64::parse_partial((leading_str() + &10000000000000000000u64.to_string()).as_bytes())
            .unwrap()
            .0,
        10000000000000000000u64
    );
    assert_eq!(
        u64::parse_partial(9999999999999999999u64.to_string().as_bytes())
            .unwrap()
            .0,
        9999999999999999999u64
    );
    assert_eq!(
        u64::parse_partial(i64::MAX.to_string().as_bytes())
            .unwrap()
            .0,
        i64::MAX as u64
    );
    assert_eq!(
        u32::parse_partial(u32::MAX.to_string().as_bytes())
            .unwrap()
            .0,
        u32::MAX
    );
    assert_eq!(
        u32::parse_partial(i32::MAX.to_string().as_bytes())
            .unwrap()
            .0,
        i32::MAX as u32
    );
    assert_eq!(
        u16::parse_partial(u16::MAX.to_string().as_bytes())
            .unwrap()
            .0,
        u16::MAX
    );
    assert_eq!(
        u16::parse_partial(i16::MAX.to_string().as_bytes())
            .unwrap()
            .0,
        i16::MAX as u16
    );
    assert_eq!(
        u8::parse_partial(u8::MAX.to_string().as_bytes()).unwrap().0,
        u8::MAX
    );
    assert_eq!(
        u8::parse_partial((leading_str() + &u8::MAX.to_string()).as_bytes())
            .unwrap()
            .0,
        u8::MAX
    );
    assert_eq!(
        u8::parse_partial(i8::MAX.to_string().as_bytes()).unwrap().0,
        i8::MAX as u8
    );
    assert_eq!(u8::parse_partial(b"0").unwrap().0, 0);
    assert_eq!(
        u8::parse_partial((leading_str() + "0").as_bytes())
            .unwrap()
            .0,
        0
    );
    assert_eq!(u8::parse(b"308"), None);
}

macro_rules! quickcheck_uint {
    ($name:ident, $ty:ident) => {
        mod $name {
            use super::*;
            ::quickcheck::quickcheck! {
                fn parse_str(x: String) -> bool {
                    assert_eq!($ty::parse(x.as_bytes()), x.parse::<$ty>().ok(), "{x}");
                    true
                }
                fn parse_valid(x: $ty) -> bool {
                    assert_eq!(
                        $ty::parse(x.to_string().as_bytes()).unwrap(),
                        x
                    );
                    assert_eq!(
                        $ty::parse((leading_str() + &x.to_string()).as_bytes()).unwrap(),
                        x
                    );
                    true
                }
            }
        }
    };
}
macro_rules! quickcheck_int {
    ($name:ident, $ty:ident) => {
        mod $name {
            use super::*;
            ::quickcheck::quickcheck! {
                fn parse_str(x: String) -> bool {
                    assert_eq!($ty::parse(x.as_bytes()), x.parse::<$ty>().ok(), "{x}");
                    true
                }
                fn parse_valid(x: $ty) -> bool {
                    assert_eq!(
                        $ty::parse(x.to_string().as_bytes()).unwrap(),
                        x
                    );
                    if !x.is_negative() {
                        assert_eq!(
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

// quickcheck_uint!(test_u128, u128);
quickcheck_uint!(test_u64, u64);
quickcheck_uint!(test_u32, u32);
quickcheck_uint!(test_u16, u16);
quickcheck_uint!(test_u8, u8);

// quickcheck_int!(test_i128, i128);
quickcheck_int!(test_i64, i64);
quickcheck_int!(test_i32, i32);
quickcheck_int!(test_i16, i16);
quickcheck_int!(test_i8, i8);
