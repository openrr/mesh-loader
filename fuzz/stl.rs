/*
Run with libFuzzer:

```sh
cargo fuzz run --release --features libfuzzer stl
```

Run with AFL++:

```sh
cd fuzz
cargo afl build --release --features afl
cargo afl fuzz -i seeds/stl -o out target/release/stl
```
*/

#![cfg_attr(feature = "libfuzzer", no_main)]

use mesh_loader::stl::from_slice;

#[cfg(any(
    not(any(feature = "libfuzzer", feature = "afl")),
    all(feature = "libfuzzer", feature = "afl"),
))]
compile_error!("exactly one of 'libfuzzer' or 'afl' feature must be enabled");

#[cfg(feature = "libfuzzer")]
libfuzzer_sys::fuzz_target!(|bytes: &[u8]| {
    run(bytes);
});

#[cfg(feature = "afl")]
fn main() {
    afl::fuzz!(|bytes: &[u8]| {
        run(bytes);
    });
}

fn run(bytes: &[u8]) {
    let _result = from_slice(bytes);
}
