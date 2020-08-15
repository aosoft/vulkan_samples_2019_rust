pub mod config;

#[repr(C, packed)]
pub struct Vertex {
    pub position: nalgebra_glm::Vec3,
    pub normal: nalgebra_glm::Vec3,
    pub tangent: nalgebra_glm::Vec3,
    pub texcoord: nalgebra_glm::Vec2,
}

impl Default for Vertex {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

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
