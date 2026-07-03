#[inline]
pub fn debug_log(msg: impl std::fmt::Display) {
    #[cfg(debug_assertions)]
    eprintln!("{msg}");
}
