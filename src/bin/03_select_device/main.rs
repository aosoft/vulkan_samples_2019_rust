//  03_select_device
use std::borrow::Cow;
use vulkan_samples_2019_rust::config;
use vulkano::VulkanObject;

fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    let config = config::Configs::new("select_device");
    let app_info = vulkano::instance::ApplicationInfo {
        application_name: Some(Cow::from(config.prog_name.as_str())),
        application_version: Some(vulkano::instance::Version {
            major: 1,
            minor: 0,
            patch: 0,
        }),
        engine_name: Some(Cow::from("sample_engine")),
        engine_version: Some(vulkano::instance::Version {
            major: 1,
            minor: 0,
            patch: 0,
        }),
    };

    let instance = vulkano::instance::Instance::new(
        Some(&app_info),
        &vulkano::instance::InstanceExtensions::none(),
        if config.validation {
            vec!["VK_LAYER_LUNARG_standard_validation"]
        } else {
            Vec::<&str>::new()
        },
    )
    .unwrap();

    let devices = vulkano::instance::PhysicalDevice::enumerate(&instance);
    if devices.len() == 0 {
        println!("利用可能なデバイスがない");
        return;
    }

    let validated_devices: Vec<vulkano::instance::PhysicalDevice> = devices
        .filter(|device| {
            for family in device.queue_families() {
                if glfw.get_physical_device_presentation_support_raw(
                    instance.internal_object() as usize,
                    device.index(),
                    family.id(),
                ) {
                    return true;
                }
            }
            false
        })
        .collect();
    if validated_devices.len() == 0 {
        println!("必要な拡張とレイヤーを備えたデバイスがない");
        return;
    }
}
