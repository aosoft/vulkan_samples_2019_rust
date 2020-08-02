//  02_list_devices
#[macro_use(defer)]
extern crate scopeguard;
use ash::version::{EntryV1_0, InstanceV1_0};
use vk_sample_config::config;

fn main() {
    let config = config::Configs::new("list_devices");
    let app_info = ash::vk::ApplicationInfo::builder()
        .application_name(std::ffi::CString::new(config.prog_name.as_str()).unwrap().as_c_str())
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

    let devices = unsafe { instance.enumerate_physical_devices().unwrap() };
    if devices.len() == 0 {
        eprintln!("利用可能なデバイスがない");
        return;
    }
    for device in devices {
        let props = unsafe { instance.get_physical_device_properties(device) };
        println!(
            "{}({})",
            unsafe { std::ffi::CStr::from_ptr(props.device_name.as_ptr()) }
                .to_str()
                .unwrap(),
            match props.device_type {
                ash::vk::PhysicalDeviceType::INTEGRATED_GPU => "統合GPU",
                ash::vk::PhysicalDeviceType::DISCRETE_GPU => "ディスクリートGPU",
                ash::vk::PhysicalDeviceType::VIRTUAL_GPU => "仮想GPU",
                ash::vk::PhysicalDeviceType::CPU => "CPU",
                _ => "その他のデバイス",
            }
        );

        let version = |ver: u32| {
            format!(
                "{}.{}.{}",
                ash::vk::version_major(ver),
                ash::vk::version_minor(ver),
                ash::vk::version_patch(ver)
            )
        };

        println!("  APIバージョン");
        println!("    {}", version(props.api_version));
        println!("  ドライババージョン");
        println!("    {}", version(props.driver_version));
        println!("  ベンダーID");
        println!("    {}", props.vendor_id);
        println!("  デバイスID");
        println!("    {}", props.device_id);

        let avail_dext = unsafe {
            instance
                .enumerate_device_extension_properties(device)
                .unwrap()
        };
        println!("  利用可能な拡張");
        for ext in avail_dext.iter() {
            println!(
                "    {}",
                unsafe { std::ffi::CStr::from_ptr(ext.extension_name.as_ptr()) }
                    .to_str()
                    .unwrap()
            );
        }
    }
}
