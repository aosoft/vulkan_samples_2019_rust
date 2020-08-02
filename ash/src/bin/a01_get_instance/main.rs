//  01_get_instance
#[macro_use(defer)]
extern crate scopeguard;
use ash::version::{EntryV1_0, InstanceV1_0};
use vk_sample_config::config;

fn main() {
    let config = config::Configs::new("get_instance");
    let app_info = ash::vk::ApplicationInfo::builder()
        .application_name(std::ffi::CString::new(config.prog_name).unwrap().as_c_str())
        .application_version(ash::vk::make_version(1, 0, 0))
        .engine_name(unsafe {
            std::ffi::CStr::from_ptr("sample_engine\0".as_ptr() as *const std::os::raw::c_char)
        })
        .engine_version(ash::vk::make_version(1, 0, 0))
        .api_version(ash::vk::make_version(1, 1, 0))
        .build();

    let entry = ash::Entry::new().unwrap();

    let ext: [*const i8; 0] = [];
    let layers = if config.validation {
        vec!["VK_LAYER_LUNARG_standard_validation\0".as_ptr() as *const i8]
    } else {
        vec![]
    };

    let create_info = ash::vk::InstanceCreateInfo::builder()
        .application_info(&app_info)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&ext);

    let instance = unsafe { entry.create_instance(&create_info, None).unwrap() };

    defer! {
        unsafe { instance.destroy_instance(None); }
    }
}
