//  02_list_devices
use std::borrow::Cow;
use vk_sample_common::config;

fn main() {
    let config = config::Configs::new("list_devices");
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
        eprintln!("利用可能なデバイスがない");
        return;
    }
    for device in devices {
        println!(
            "{}({})",
            device.name(),
            match device.ty() {
                vulkano::instance::PhysicalDeviceType::Other => "その他のデバイス",
                vulkano::instance::PhysicalDeviceType::IntegratedGpu => "統合GPU",
                vulkano::instance::PhysicalDeviceType::DiscreteGpu => "ディスクリートGPU",
                vulkano::instance::PhysicalDeviceType::VirtualGpu => "仮想GPU",
                vulkano::instance::PhysicalDeviceType::Cpu => "CPU",
            }
        );
        println!("  APIバージョン");
        println!("    {}", device.api_version());
        println!("  ドライババージョン");
        println!(
            "    {}",
            vulkano::instance::Version::from_vulkan_version(device.driver_version())
        );
        println!("  ベンダーID");
        println!("    {}", device.pci_vendor_id());
        println!("  デバイスID");
        println!("    {}", device.pci_device_id());

        let avail_dext = vulkano::device::RawDeviceExtensions::supported_by_device(device);
        println!("  利用可能な拡張");
        for ext in avail_dext.iter() {
            println!("    {}", ext.to_str().unwrap());
        }
    }
}
