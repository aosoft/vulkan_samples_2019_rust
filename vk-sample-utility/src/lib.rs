pub mod config;

pub unsafe fn from_slice<'a, T, U>(src: &'a [U]) -> &'a [T] {
    std::slice::from_raw_parts::<T>(
        src.as_ptr() as *const T,
        src.len() / std::mem::size_of::<T>(),
    )
}
