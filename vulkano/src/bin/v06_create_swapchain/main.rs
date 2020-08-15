//  06_create_swapchain
use std::borrow::Cow;
use vk_sample_utility::config;
use vulkano::VulkanObject;

#[allow(unused_variables)]
fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    let config = config::Configs::new("create_swapchain");
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

    let dext = vulkano::device::DeviceExtensions {
        khr_swapchain: true,
        ..vulkano::device::DeviceExtensions::none()
    };

    let validated_devices: Vec<vulkano::instance::PhysicalDevice> = devices
        .filter(|device| {
            let ext = vulkano::device::DeviceExtensions::supported_by_device(*device);
            if ext.khr_swapchain {
                for family in device.queue_families() {
                    if glfw.get_physical_device_presentation_support_raw(
                        instance.internal_object(),
                        device.internal_object(),
                        family.id(),
                    ) {
                        return true;
                    }
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

    let surface = std::sync::Arc::new(unsafe {
        vulkano::swapchain::Surface::from_raw_surface(instance.clone(), raw_surface, window)
    });

    if config.device_index as usize >= validated_devices.len() {
        eprintln!("{} 番目のデバイスは存在しない", config.device_index);
        return;
    }

    let physical_device = validated_devices[config.device_index as usize];
    let mut queue_props = physical_device
        .queue_families()
        .filter(|family| surface.is_supported(*family).unwrap());

    let graphics_queue = queue_props.find(|queue_prop| queue_prop.supports_graphics());

    let present_queue = if graphics_queue.is_some() {
        graphics_queue.clone()
    } else {
        queue_props.into_iter().next()
    };

    if graphics_queue.is_none() || present_queue.is_none() {
        eprintln!("必要なキューが備わっていない");
        return;
    }

    let graphics_queue = graphics_queue.unwrap();
    let present_queue = present_queue.unwrap();
    let eq_queue = graphics_queue == present_queue;

    let (device, queues) = vulkano::device::Device::new(
        physical_device,
        physical_device.supported_features(),
        &dext,
        if eq_queue {
            vec![(graphics_queue, 0.0)]
        } else {
            vec![(graphics_queue, 0.0), (present_queue, 0.0)]
        },
    )
    .unwrap();

    let formats = surface
        .capabilities(physical_device)
        .unwrap()
        .supported_formats;
    if formats.len() == 0 {
        eprintln!("利用可能なピクセルフォーマットが無い");
        return;
    }
    let format = match formats
        .iter()
        .find(|f| f.0 == vulkano::format::Format::B8G8R8A8Unorm)
    {
        Some(t) => Some(t),
        None => formats
            .iter()
            .find(|f| f.0 == vulkano::format::Format::R8G8B8A8Unorm),
    }
    .expect("利用可能なピクセルフォーマットが無い");

    let surface_capabilities = surface.capabilities(physical_device).unwrap();
    let swapchain_extent = surface_capabilities
        .current_extent
        .unwrap_or([config.width as u32, config.height as u32]);
    let swapchain_image_count = std::cmp::min(
        surface_capabilities.min_image_count + 1,
        surface_capabilities.max_image_count.unwrap_or(0),
    );

    let swapchain = vulkano::swapchain::Swapchain::new(
        device.clone(),
        surface,
        swapchain_image_count,
        format.0,
        swapchain_extent,
        1,
        vulkano::image::ImageUsage {
            color_attachment: true,
            ..vulkano::image::ImageUsage::none()
        },
        vulkano::sync::SharingMode::Exclusive,
        if surface_capabilities.supported_transforms.identity {
            vulkano::swapchain::SurfaceTransform::Identity
        } else {
            surface_capabilities.current_transform
        },
        vulkano::swapchain::CompositeAlpha::Opaque,
        vulkano::swapchain::PresentMode::Fifo,
        vulkano::swapchain::FullscreenExclusive::Default,
        true,
        format.1,
    );
}
