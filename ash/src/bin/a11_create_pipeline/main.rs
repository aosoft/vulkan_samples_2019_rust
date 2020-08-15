//  11_create_pipeline
#[macro_use(defer)]
extern crate scopeguard;
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk::Handle;
use std::io::Read;
use vk_sample_config::config;

#[repr(C, packed)]
struct Vertex {
    pub position: nalgebra_glm::Vec3,
    pub normal: nalgebra_glm::Vec3,
    pub tangent: nalgebra_glm::Vec3,
    pub texcoord: nalgebra_glm::Vec3,
}

impl Default for Vertex {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[allow(unused_variables)]
fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    let config = config::Configs::new("create_pipeline");
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

    let descriptor_set_layout_bindings = [ash::vk::DescriptorSetLayoutBinding::builder()
        .descriptor_type(ash::vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1)
        .binding(0)
        .stage_flags(ash::vk::ShaderStageFlags::VERTEX)
        .build()];
    let descriptor_set_layout = std::cell::RefCell::new(
        Vec::<ash::vk::DescriptorSetLayout>::with_capacity(swapchain_image_count as usize),
    );
    defer! {
        descriptor_set_layout.borrow().iter().for_each(|item|{
            unsafe { device.destroy_descriptor_set_layout(*item, None) };
        });
    }
    for i in 0..swapchain_image_count {
        descriptor_set_layout.borrow_mut().push(unsafe {
            device
                .create_descriptor_set_layout(
                    &ash::vk::DescriptorSetLayoutCreateInfo::builder().build(),
                    None,
                )
                .unwrap()
        });
    }

    let attachments = [
        ash::vk::AttachmentDescription::builder()
            .format(format.format)
            .samples(ash::vk::SampleCountFlags::TYPE_1)
            .load_op(ash::vk::AttachmentLoadOp::CLEAR)
            .store_op(ash::vk::AttachmentStoreOp::STORE)
            .stencil_load_op(ash::vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(ash::vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(ash::vk::ImageLayout::UNDEFINED)
            .final_layout(ash::vk::ImageLayout::PRESENT_SRC_KHR)
            .build(),
        ash::vk::AttachmentDescription::builder()
            .format(ash::vk::Format::D16_UNORM)
            .samples(ash::vk::SampleCountFlags::TYPE_1)
            .load_op(ash::vk::AttachmentLoadOp::CLEAR)
            .store_op(ash::vk::AttachmentStoreOp::STORE)
            .stencil_load_op(ash::vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(ash::vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(ash::vk::ImageLayout::UNDEFINED)
            .final_layout(ash::vk::ImageLayout::PRESENT_SRC_KHR)
            .build(),
    ];
    let color_reference = [ash::vk::AttachmentReference::builder()
        .attachment(1)
        .layout(ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .build()];
    let depth_reference = ash::vk::AttachmentReference::builder()
        .attachment(1)
        .layout(ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        .build();
    let subpass = [ash::vk::SubpassDescription::builder()
        .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_reference)
        .depth_stencil_attachment(&depth_reference)
        .build()];

    let render_pass = unsafe {
        device
            .create_render_pass(
                &ash::vk::RenderPassCreateInfo::builder()
                    .attachments(&attachments)
                    .subpasses(&subpass)
                    .build(),
                None,
            )
            .unwrap()
    };

    let allocator_info = vk_mem::AllocatorCreateInfo {
        physical_device: *physical_device,
        device: device.clone(),
        instance: instance.clone(),
        ..vk_mem::AllocatorCreateInfo::default()
    };
    let allocator = vk_mem::Allocator::new(&allocator_info).expect("アロケータを作成できない");

    let mut framebuffers = Vec::<FrameBuffer>::new();
    for swapchain_image in unsafe { swapchain_loader.get_swapchain_images(swapchain).unwrap() } {
        let mut attachments_raw = Vec::<ash::vk::ImageView>::new();
        let mut framebuffer = FrameBuffer::new(&device, &allocator);
        framebuffer.color_image_attachment = unsafe {
            device
                .create_image_view(
                    &ash::vk::ImageViewCreateInfo::builder()
                        .image(swapchain_image)
                        .view_type(ash::vk::ImageViewType::TYPE_2D)
                        .format(format.format)
                        .build(),
                    None,
                )
                .unwrap()
        };
        attachments_raw.push(framebuffer.color_image_attachment);

        let depth_image_create_info = ash::vk::ImageCreateInfo::builder()
            .format(ash::vk::Format::D16_UNORM)
            .mip_levels(1)
            .array_layers(1)
            .samples(ash::vk::SampleCountFlags::TYPE_1)
            .usage(ash::vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .initial_layout(ash::vk::ImageLayout::UNDEFINED)
            .image_type(ash::vk::ImageType::TYPE_2D)
            .extent(
                ash::vk::Extent3D::builder()
                    .width(swapchain_extent.width)
                    .height(swapchain_extent.height)
                    .depth(1)
                    .build(),
            )
            .build();
        let depth_image_alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::GpuOnly,
            ..vk_mem::AllocationCreateInfo::default()
        };
        let (depth_image, depth_image_allocation, depth_image_alloc_info) = allocator
            .create_image(&depth_image_create_info, &depth_image_alloc_info)
            .expect("イメージを作成できない");
        framebuffer.depth_image = depth_image;
        framebuffer.depth_image_allocation = depth_image_allocation;
        framebuffer.depth_image_attachment = unsafe {
            device
                .create_image_view(
                    &ash::vk::ImageViewCreateInfo::builder()
                        .image(framebuffer.depth_image)
                        .view_type(ash::vk::ImageViewType::TYPE_2D)
                        .format(ash::vk::Format::D16_UNORM)
                        .build(),
                    None,
                )
                .unwrap()
        };
        attachments_raw.push(framebuffer.depth_image_attachment);

        framebuffer.framebuffer = unsafe {
            device
                .create_framebuffer(
                    &ash::vk::FramebufferCreateInfo::builder()
                        .render_pass(render_pass)
                        .attachments(attachments_raw.as_slice())
                        .width(swapchain_extent.width)
                        .height(swapchain_extent.height)
                        .layers(1)
                        .build(),
                    None,
                )
                .unwrap()
        };

        framebuffers.push(framebuffer);
    }

    let vertex_shader_file_path: std::path::PathBuf =
        [config.shader_dir.as_str(), "simple.vert.spv"]
            .iter()
            .collect();
    let mut vertex_shader_file =
        std::fs::File::open(vertex_shader_file_path).expect("頂点シェーダを読む事ができない");
    let mut vertex_shader_bin = Vec::<u8>::new();
    vertex_shader_file.read_to_end(&mut vertex_shader_bin);
    //let vertex_shader_bin = vulkan_samples_2019_rust_ash::to_vec_u32(vertex_shader_bin.as_slice());
    let vertex_shader_module = unsafe {
        device
            .create_shader_module(
                &ash::vk::ShaderModuleCreateInfo::builder()
                    .code(unsafe { from_slice(&vertex_shader_bin.as_slice()) })
                    .build(),
                None,
            )
            .unwrap()
    };

    defer! { unsafe { device.destroy_shader_module(vertex_shader_module, None); } }

    let fragment_shader_file_path: std::path::PathBuf =
        [config.shader_dir.as_str(), "simple.frag.spv"]
            .iter()
            .collect();
    let mut fragment_shader_file = std::fs::File::open(fragment_shader_file_path)
        .expect("フラグメントシェーダを読む事ができない");
    let mut fragment_shader_bin = Vec::<u8>::new();
    fragment_shader_file.read_to_end(&mut fragment_shader_bin);
    let fragment_shader_module = unsafe {
        device
            .create_shader_module(
                &ash::vk::ShaderModuleCreateInfo::builder()
                    .code(unsafe { from_slice(&fragment_shader_bin.as_slice()) })
                    .build(),
                None,
            )
            .unwrap()
    };

    defer! { unsafe { device.destroy_shader_module(fragment_shader_module, None); } }

    let pipeline_shader_stages = [
        ash::vk::PipelineShaderStageCreateInfo::builder()
            .stage(ash::vk::ShaderStageFlags::VERTEX)
            .module(vertex_shader_module)
            .name(unsafe {
                std::ffi::CStr::from_ptr("main\0".as_ptr() as *const std::os::raw::c_char)
            })
            .build(),
        ash::vk::PipelineShaderStageCreateInfo::builder()
            .stage(ash::vk::ShaderStageFlags::FRAGMENT)
            .module(fragment_shader_module)
            .name(unsafe {
                std::ffi::CStr::from_ptr("main\0".as_ptr() as *const std::os::raw::c_char)
            })
            .build(),
    ];

    let push_constant_range = [ash::vk::PushConstantRange::builder()
        .stage_flags(ash::vk::ShaderStageFlags::VERTEX | ash::vk::ShaderStageFlags::FRAGMENT)
        .offset(0)
        .size(
            (std::mem::size_of::<nalgebra_glm::Mat4>() * 2
                + std::mem::size_of::<nalgebra_glm::Vec3>() * 2) as u32,
        )
        .build()];
    let pipeline_layout = unsafe {
        device.create_pipeline_layout(
            &ash::vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(descriptor_set_layout.borrow().as_slice())
                .push_constant_ranges(push_constant_range.as_ref())
                .build(),
                None)
        .unwrap()
    };
    defer! { unsafe { device.destroy_pipeline_layout(pipeline_layout, None); } }

    let vertex_input_binding = ash::vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(std::mem::size_of::<Vertex>() as u32)
        .input_rate(ash::vk::VertexInputRate::VERTEX)
        .build();
    let vertex_input_attribute = [
        ash::vk::VertexInputAttributeDescription::builder()
            .location(0)
            .binding(0)
            .format(ash::vk::Format::R32G32B32_SFLOAT)
            //.offset()
            .build()
    ];
}

struct FrameBuffer<'a> {
    pub device: &'a ash::Device,
    pub allocator: &'a vk_mem::Allocator,
    pub color_image_attachment: ash::vk::ImageView,
    pub depth_image: ash::vk::Image,
    pub depth_image_allocation: vk_mem::Allocation,
    pub depth_image_attachment: ash::vk::ImageView,
    pub framebuffer: ash::vk::Framebuffer,
}

impl FrameBuffer<'_> {
    pub fn new<'a>(device: &'a ash::Device, allocator: &'a vk_mem::Allocator) -> FrameBuffer<'a> {
        FrameBuffer {
            device: device,
            allocator: allocator,
            color_image_attachment: Default::default(),
            depth_image: Default::default(),
            depth_image_allocation: unsafe { std::mem::zeroed() },
            depth_image_attachment: Default::default(),
            framebuffer: Default::default(),
        }
    }
}

impl Drop for FrameBuffer<'_> {
    fn drop(&mut self) {
        if self.framebuffer.as_raw() != 0 {
            unsafe { self.device.destroy_framebuffer(self.framebuffer, None) }
        }

        if self.color_image_attachment.as_raw() != 0 {
            unsafe {
                self.device
                    .destroy_image_view(self.color_image_attachment, None);
            }
        }

        if self.depth_image_attachment.as_raw() != 0 {
            unsafe {
                self.device
                    .destroy_image_view(self.depth_image_attachment, None);
            }
        }

        if self.depth_image.as_raw() != 0 {
            self.allocator
                .destroy_image(self.depth_image, &self.depth_image_allocation)
                .unwrap();
        }
    }
}

unsafe fn from_slice<'a, T, U>(src: &'a [U]) -> &'a [T] {
    std::slice::from_raw_parts::<T>(
        src.as_ptr() as *const T,
        src.len() / std::mem::size_of::<T>(),
    )
}
