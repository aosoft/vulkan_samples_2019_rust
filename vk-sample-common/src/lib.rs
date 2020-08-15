pub mod config;

pub unsafe fn from_slice<'a, T, U>(src: &'a [U]) -> &'a [T] {
    std::slice::from_raw_parts::<T>(
        src.as_ptr() as *const T,
        src.len() / std::mem::size_of::<T>(),
    )
}

// Simple offset_of macro akin to C++ offsetof
#[macro_export]
macro_rules! offset_of {
    ($base:path, $field:ident) => {{
        #[allow(unused_unsafe)]
        unsafe {
            let b: $base = std::mem::zeroed();
            (&b.$field as *const _ as isize) - (&b as *const _ as isize)
        }
    }};
}
