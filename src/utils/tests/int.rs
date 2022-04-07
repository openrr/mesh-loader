use super::*;

#[test]
fn max_digit_count() {
    macro_rules! max_digit_count {
        ($ty:ident) => {{
            let mut max = $ty::MAX;
            let mut count = 0;
            while max > 0 {
                count += 1;
                max /= 10;
            }
            assert_eq!(<$ty as Int>::MAX_DIGIT_COUNT, count);
            count
        }};
    }

    assert_eq!(max_digit_count!(u128), 39);
    assert_eq!(max_digit_count!(u64), 20);
    assert_eq!(max_digit_count!(u32), 10);
    assert_eq!(max_digit_count!(u16), 5);
    assert_eq!(max_digit_count!(u8), 3);
    // assert_eq!(max_digit_count!(i128), 39);
    // assert_eq!(max_digit_count!(i64), 19);
    // assert_eq!(max_digit_count!(i32), 10);
    // assert_eq!(max_digit_count!(i16), 5);
    // assert_eq!(max_digit_count!(i8), 3);
}

#[test]
fn uint() {
    assert_eq!(
        u128::parse_partial(u128::MAX.to_string().as_bytes())
            .unwrap()
            .0,
        u128::MAX
    );
    assert_eq!(
        u128::parse_partial(i128::MAX.to_string().as_bytes())
            .unwrap()
            .0,
        i128::MAX as _
    );
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
        u8::parse_partial(i8::MAX.to_string().as_bytes()).unwrap().0,
        i8::MAX as u8
    );
}
