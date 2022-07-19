// macro_rules! trace {
//     ($($tt:tt)*) => {{
//         #[cfg(feature = "tracing")]
//         {
//             tracing::debug!($($tt)*)
//         }
//     }};
// }
#[cfg(feature = "obj")]
macro_rules! debug {
    ($($tt:tt)*) => {{
        #[cfg(feature = "tracing")]
        {
            tracing::debug!($($tt)*)
        }
    }};
}
// macro_rules! info {
//     ($($tt:tt)*) => {{
//         #[cfg(feature = "tracing")]
//         {
//             tracing::info!($($tt)*)
//         }
//     }};
// }
// macro_rules! warn {
//     ($($tt:tt)*) => {{
//         #[cfg(feature = "tracing")]
//         {
//             tracing::warn!($($tt)*)
//         }
//     }};
// }
// macro_rules! error {
//     ($($tt:tt)*) => {{
//         #[cfg(feature = "tracing")]
//         {
//             tracing::error!($($tt)*)
//         }
//     }};
// }
