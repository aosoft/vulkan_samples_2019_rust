//  06_create_swapchain
#[macro_use(defer)]
extern crate scopeguard;
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk::Handle;
use vk_sample_common::config;

#[allow(unused_variables)]
fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    let config = config::Configs::new("create_swapchain");
    let app_info = ash::vk::ApplicationInfo::builder()
        .application_name(
            std::ffi::CString::new(config.prog_name.as_str())
                .unwrap()
                .as_c_str(),
        )
        .application_version(ash::vk::make_version(1, 0, 0))
        .engine_name(unsafe {
            std::ffi::CStr::from_ptr("sample_engine\0".as_ptr() as *const std::os::raw::c_char)
        })
        .engine_version(ash::vk::make_version(1, 0, 0))
        .api_version(ash::vk::make_version(1, 1, 0))
        .build();

    let entry = ash::Entry::new().unwrap();

    let ext = glfw
        .get_required_instance_extensions()
        .unwrap()
        .iter()
        .map(|item| std::ffi::CString::new(item.as_str()).unwrap())
        .collect::<Vec<_>>();
    let ext_raw = &ext.iter().map(|item| item.as_ptr()).collect::<Vec<_>>();
    let layers = if config.validation {
        vec!["VK_LAYER_LUNARG_standard_validation\0".as_ptr() as *const i8]
    } else {
        vec![]
    };

    let create_info = ash::vk::InstanceCreateInfo::builder()
        .application_info(&app_info)
        .enabled_layer_names(&layers)
        .enabled_extension_names(ext_raw.as_slice());

    let instance = unsafe { entry.create_instance(&create_info, None).unwrap() };

    defer! {
        unsafe { instance.destroy_instance(None); }
    }

    let devices = unsafe { instance.enumerate_physical_devices().unwrap() };
    if devices.len() == 0 {
        eprintln!("利用可能なデバイスがない");
        return;
    }

    let dext = [ash::extensions::khr::Swapchain::name().as_ptr()];
    let validated_devices = devices
        .iter()
        .filter(|device| {
            if dext.len() > 0 {
                let avail_dext = unsafe {
                    instance
                        .enumerate_device_extension_properties(**device)
                        .unwrap()
                };
                if dext
                    .iter()
                    .find(|w| {
                        avail_dext
                            .iter()
                            .find(|v| unsafe {
                                std::ffi::CStr::from_ptr(v.extension_name.as_ptr())
                                    == std::ffi::CStr::from_ptr(**w)
                            })
                            .is_some()
                    })
                    .is_none()
                {
                    return false;
                }
            }

            let queue_props =
                unsafe { instance.get_physical_device_queue_family_properties(**device) };
            for i in 0..queue_props.len() {
                if glfw.get_physical_device_presentation_support_raw(
                    instance.handle().as_raw() as vk_sys::Instance,
                    device.as_raw() as vk_sys::PhysicalDevice,
                    i as u32,
                ) {
                    return true;
                }
            }
            false
        })
        .collect::<Vec<_>>();

    if validated_devices.len() == 0 {
        eprintln!("必要な拡張とレイヤーを備えたデバイスがない");
        return;
    }

    println!("利用可能なデバイス");
    for i in 0..validated_devices.len() {
        println!("{}: {}", i, unsafe {
            std::ffi::CStr::from_ptr(
                instance
                    .get_physical_device_properties(*validated_devices[i])
                    .device_name
                    .as_ptr(),
            )
            .to_str()
            .unwrap()
        })
    }

    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

    let (window, _) = glfw
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
        instance.handle().as_raw() as vk_sys::Instance,
        std::ptr::null(),
        &mut raw_surface,
    ) != 0
    {
        eprintln!("サーフェスを作成できない");
        return;
    }

    let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);
    let surface = ash::vk::SurfaceKHR::from_raw(raw_surface);

    defer! {
         unsafe { surface_loader.destroy_surface(surface, None); }
    }

    if config.device_index as usize >= devices.len() {
        eprintln!("{}番目のデバイスは存在しない", config.device_index);
        return;
    }

    let physical_device = validated_devices[config.device_index as usize];
    let queue_props =
        unsafe { instance.get_physical_device_queue_family_properties(*physical_device) };

    let supported = (0..queue_props.len())
        .map(|i| unsafe {
            surface_loader
                .get_physical_device_surface_support(*physical_device, i as u32, surface)
                .unwrap()
        })
        .collect::<Vec<_>>();
    let graphics_queue_index = (0..queue_props.len()).find(|i| {
        queue_props[*i]
            .queue_flags
            .intersects(ash::vk::QueueFlags::GRAPHICS)
    });
    let present_queue_index = match graphics_queue_index {
        Some(t) => Some(t),
        None => (0..supported.len()).find(|i| supported[*i]),
    };
    if graphics_queue_index.is_none() || present_queue_index.is_none() {
        eprintln!("必要なキューが備わっていない");
        return;
    }
    let graphics_queue_index = graphics_queue_index.unwrap() as u32;
    let present_queue_index = present_queue_index.unwrap() as u32;
    let eq_queue = graphics_queue_index == present_queue_index;

    let builder = |index| {
        let priority = [0.0];
        ash::vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(index as u32)
            .queue_priorities(&priority)
            .build()
    };
    let queues = if eq_queue {
        vec![builder(graphics_queue_index)]
    } else {
        vec![builder(graphics_queue_index), builder(present_queue_index)]
    };

    let features = unsafe { instance.get_physical_device_features(*physical_device) };
    let device = unsafe {
        instance
            .create_device(
                *physical_device,
                &ash::vk::DeviceCreateInfo::builder()
                    .queue_create_infos(&queues)
                    .enabled_extension_names(&dext)
                    .enabled_features(&features)
                    .build(),
                None,
            )
            .unwrap()
    };

    defer! {
        unsafe { device.destroy_device(None); }
    }

    let formats = unsafe {
        surface_loader
            .get_physical_device_surface_formats(*physical_device, surface)
            .unwrap()
    };
    if formats.len() == 0 {
        eprintln!("利用可能なピクセルフォーマットが無い");
        return;
    }
    let format = match formats
        .iter()
        .find(|f| f.format == ash::vk::Format::B8G8R8A8_UNORM)
    {
        Some(t) => Some(t),
        None => formats
            .iter()
            .find(|f| f.format == ash::vk::Format::R8G8B8A8_UNORM),
    }
    .expect("利用可能なピクセルフォーマットが無い");

    let surface_capabilities = unsafe {
        surface_loader
            .get_physical_device_surface_capabilities(*physical_device, surface)
            .unwrap()
    };
    let swapchain_extent = if surface_capabilities.current_extent.width == -1i32 as u32 {
        ash::vk::Extent2D::builder()
            .width(config.width)
            .height(config.height)
            .build()
    } else {
        surface_capabilities.current_extent
    };
    let swapchain_image_count = std::cmp::min(
        surface_capabilities.min_image_count + 1,
        surface_capabilities.max_image_count,
    );

    let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &device);
    let swapchain = unsafe {
        swapchain_loader
            .create_swapchain(
                &ash::vk::SwapchainCreateInfoKHR::builder()
                    .surface(surface)
                    .min_image_count(swapchain_image_count)
                    .image_format(format.format)
                    .image_color_space(format.color_space)
                    .image_extent(swapchain_extent)
                    .image_array_layers(1)
                    .image_usage(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT)
                    .image_sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
                    .pre_transform(
                        if surface_capabilities
                            .supported_transforms
                            .intersects(ash::vk::SurfaceTransformFlagsKHR::IDENTITY)
                        {
                            ash::vk::SurfaceTransformFlagsKHR::IDENTITY
                        } else {
                            surface_capabilities.current_transform
                        },
                    )
                    .composite_alpha(ash::vk::CompositeAlphaFlagsKHR::OPAQUE)
                    .present_mode(ash::vk::PresentModeKHR::FIFO)
                    .clipped(true)
                    .build(),
                None,
            )
            .unwrap()
    };

    defer! { unsafe { swapchain_loader.destroy_swapchain(swapchain, None); } }
}
