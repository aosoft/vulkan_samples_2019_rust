//  a04_create_window

use std::borrow::Cow;
use vk_sample_config::config;
use vulkano::VulkanObject;

#[allow(unused_variables)]
fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    let config = config::Configs::new("create_window");
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

    let mut required_ext = std::collections::HashSet::<std::ffi::CString>::new();
    for x in glfw.get_required_instance_extensions().unwrap() {
        required_ext.insert(std::ffi::CString::new(x).unwrap());
    }
    let ext = vulkano::instance::RawInstanceExtensions::new(required_ext);

    let instance = vulkano::instance::Instance::new(
        Some(&app_info),
        ext,
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

    let validated_devices: Vec<vulkano::instance::PhysicalDevice> = devices
        .filter(|device| {
            for family in device.queue_families() {
                if glfw.get_physical_device_presentation_support_raw(
                    instance.internal_object(),
                    device.internal_object(),
                    family.id(),
                ) {
                    return true;
                }
            }
            false
        })
        .collect();
    if validated_devices.len() == 0 {
        eprintln!("必要な拡張とレイヤーを備えたデバイスがない");
        return;
    }

    println!("利用可能なデバイス");
    for i in 0..validated_devices.len() {
        println!("{}: {}", i, validated_devices[i].name())
    }

    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

    let (window, events) = glfw
        .with_primary_monitor(|glfw, m| {
            glfw.create_window(
                config.width,
                config.height,
                config.prog_name.as_str(),
                if config.fullscreen {
                    m.map_or(glfw::WindowMode::Windowed, |m| {
                        glfw::WindowMode::FullScreen(m)
                    })
                } else {
                    glfw::WindowMode::Windowed
                },
            )
        })
        .expect("ウィンドウを作成できない");

    let mut raw_surface: vk_sys::SurfaceKHR = 0;
    if window.create_window_surface(
        instance.internal_object(),
        std::ptr::null(),
        &mut raw_surface,
    ) != 0
    {
        eprintln!("サーフェスを作成できない");
        return;
    }

    let surface =
        unsafe { vulkano::swapchain::Surface::from_raw_surface(instance, raw_surface, window) };
}
