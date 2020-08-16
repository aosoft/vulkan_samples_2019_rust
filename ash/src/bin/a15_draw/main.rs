//  15_draw
#[macro_use(defer)]
extern crate scopeguard;
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk::Handle;
use std::io::Read;
use vk_sample_common::config;

#[allow(unused_variables)]
fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    let config = config::Configs::new("draw");
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
                    .enabled_features(&features),
                None,
            )
            .unwrap()
    };

    defer! {
        unsafe { device.destroy_device(None); }
    }

    let graphics_queue = unsafe { device.get_device_queue(graphics_queue_index, 0) };
    let present_queue = if eq_queue {
        graphics_queue
    } else {
        unsafe { device.get_device_queue(present_queue_index, 0) }
    };

    let graphics_command_pool = unsafe {
        device
            .create_command_pool(
                &ash::vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(graphics_queue_index)
                    .flags(ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .build(),
                None,
            )
            .unwrap()
    };

    defer! { unsafe { device.destroy_command_pool(graphics_command_pool, None); }}

    //  06_create_swapchain
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

    //  07_create_descriptor_set
    let max_descriptor_set_count = 20_u32;
    let descriptor_pool_size = [ash::vk::DescriptorPoolSize::builder()
        .ty(ash::vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1)
        .build()];
    let descriptor_pool = unsafe {
        device
            .create_descriptor_pool(
                &ash::vk::DescriptorPoolCreateInfo::builder()
                    .pool_sizes(descriptor_pool_size.as_ref())
                    .max_sets(max_descriptor_set_count)
                    .flags(ash::vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                    .build(),
                None,
            )
            .unwrap()
    };
    defer! { unsafe { device.destroy_descriptor_pool(descriptor_pool, None); } }

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

    let descriptor_set = unsafe {
        device
            .allocate_descriptor_sets(
                &ash::vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(descriptor_pool)
                    .set_layouts(descriptor_set_layout.borrow().as_slice())
                    .build(),
            )
            .unwrap()
    };

    defer! {
        unsafe { device.free_descriptor_sets(descriptor_pool, descriptor_set.as_slice()); }
    }

    //  08_create_render_pass
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

    defer! { unsafe { device.destroy_render_pass(render_pass, None); }}

    //  09_create_frame_buffer
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

    //  10_create_shader_module
    let vertex_shader_file_path: std::path::PathBuf =
        [config.shader_dir.as_str(), "simple.vert.spv"]
            .iter()
            .collect();
    let mut vertex_shader_file =
        std::fs::File::open(vertex_shader_file_path).expect("頂点シェーダを読む事ができない");
    let mut vertex_shader_bin = Vec::<u8>::new();
    vertex_shader_file
        .read_to_end(&mut vertex_shader_bin)
        .unwrap();
    //let vertex_shader_bin = vulkan_samples_2019_rust_ash::to_vec_u32(vertex_shader_bin.as_slice());
    let vertex_shader_module = unsafe {
        device
            .create_shader_module(
                &ash::vk::ShaderModuleCreateInfo::builder()
                    .code(vk_sample_common::from_slice(&vertex_shader_bin.as_slice()))
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
    fragment_shader_file
        .read_to_end(&mut fragment_shader_bin)
        .unwrap();
    let fragment_shader_module = unsafe {
        device
            .create_shader_module(
                &ash::vk::ShaderModuleCreateInfo::builder()
                    .code(vk_sample_common::from_slice(
                        &fragment_shader_bin.as_slice(),
                    ))
                    .build(),
                None,
            )
            .unwrap()
    };

    defer! { unsafe { device.destroy_shader_module(fragment_shader_module, None); } }

    //  11_create_pipeline
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
        device
            .create_pipeline_layout(
                &ash::vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(descriptor_set_layout.borrow().as_slice())
                    .push_constant_ranges(push_constant_range.as_ref())
                    .build(),
                None,
            )
            .unwrap()
    };
    defer! { unsafe { device.destroy_pipeline_layout(pipeline_layout, None); } }

    let vertex_input_binding = [ash::vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(std::mem::size_of::<vk_sample_common::Vertex>() as u32)
        .input_rate(ash::vk::VertexInputRate::VERTEX)
        .build()];
    let vertex_input_attribute = [
        ash::vk::VertexInputAttributeDescription::builder()
            .location(0)
            .binding(0)
            .format(ash::vk::Format::R32G32B32_SFLOAT)
            .offset(vk_sample_common::offset_of!(vk_sample_common::Vertex, position) as u32)
            .build(),
        ash::vk::VertexInputAttributeDescription::builder()
            .location(1)
            .binding(0)
            .format(ash::vk::Format::R32G32B32_SFLOAT)
            .offset(vk_sample_common::offset_of!(vk_sample_common::Vertex, normal) as u32)
            .build(),
        ash::vk::VertexInputAttributeDescription::builder()
            .location(2)
            .binding(0)
            .format(ash::vk::Format::R32G32B32_SFLOAT)
            .offset(vk_sample_common::offset_of!(vk_sample_common::Vertex, tangent) as u32)
            .build(),
        ash::vk::VertexInputAttributeDescription::builder()
            .location(3)
            .binding(0)
            .format(ash::vk::Format::R32G32_SFLOAT)
            .offset(vk_sample_common::offset_of!(vk_sample_common::Vertex, texcoord) as u32)
            .build(),
    ];
    let input_assembly_info = ash::vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(ash::vk::PrimitiveTopology::TRIANGLE_LIST)
        .build();
    let viewport_info = ash::vk::PipelineViewportStateCreateInfo::builder()
        .viewport_count(1)
        .scissor_count(1)
        .build();
    let rasterization_info = ash::vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(ash::vk::PolygonMode::FILL)
        .cull_mode(ash::vk::CullModeFlags::NONE)
        .front_face(ash::vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false)
        .line_width(1.0)
        .build();
    let stencil_op = ash::vk::StencilOpState::builder()
        .fail_op(ash::vk::StencilOp::KEEP)
        .pass_op(ash::vk::StencilOp::KEEP)
        .compare_op(ash::vk::CompareOp::ALWAYS)
        .build();
    let depth_stencil_info = ash::vk::PipelineDepthStencilStateCreateInfo::builder()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(ash::vk::CompareOp::LESS_OR_EQUAL)
        .depth_bounds_test_enable(false)
        .stencil_test_enable(false)
        .front(stencil_op)
        .back(stencil_op)
        .build();
    let color_blend_attachments = [ash::vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(
            ash::vk::ColorComponentFlags::R
                | ash::vk::ColorComponentFlags::G
                | ash::vk::ColorComponentFlags::B
                | ash::vk::ColorComponentFlags::A,
        )
        .build()];
    let color_blend_info = ash::vk::PipelineColorBlendStateCreateInfo::builder()
        .attachments(color_blend_attachments.as_ref())
        .build();
    let dynamic_states = [
        ash::vk::DynamicState::VIEWPORT,
        ash::vk::DynamicState::SCISSOR,
    ];
    let dynamic_state_info = ash::vk::PipelineDynamicStateCreateInfo::builder()
        .dynamic_states(dynamic_states.as_ref())
        .build();
    let vertex_input_state = ash::vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_attribute_descriptions(vertex_input_attribute.as_ref())
        .vertex_binding_descriptions(vertex_input_binding.as_ref())
        .build();
    let multisample_info = ash::vk::PipelineMultisampleStateCreateInfo::builder().build();

    let pipeline_create_info = [ash::vk::GraphicsPipelineCreateInfo::builder()
        .stages(pipeline_shader_stages.as_ref())
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_info)
        .viewport_state(&viewport_info)
        .rasterization_state(&rasterization_info)
        .multisample_state(&multisample_info)
        .depth_stencil_state(&depth_stencil_info)
        .color_blend_state(&color_blend_info)
        .dynamic_state(&dynamic_state_info)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .build()];

    let graphics_pipeline = unsafe {
        device
            .create_graphics_pipelines(
                ash::vk::PipelineCache::null(),
                pipeline_create_info.as_ref(),
                None,
            )
            .unwrap()
    };

    defer! {
        unsafe {
            for pipeline in graphics_pipeline.into_iter() {
                device.destroy_pipeline(pipeline, None);
            }
        }
    }

    //  12_create_vertex_buffer
    let vertices = [
        vk_sample_common::Vertex {
            position: nalgebra_glm::Vec3::new(0.0, 0.0, 0.0),
            normal: nalgebra_glm::Vec3::new(0.0, 0.0, 1.0),
            tangent: nalgebra_glm::Vec3::new(1.0, 0.0, 0.0),
            texcoord: nalgebra_glm::Vec2::new(0.0, 0.0),
        },
        vk_sample_common::Vertex {
            position: nalgebra_glm::Vec3::new(1.0, 0.0, 0.0),
            normal: nalgebra_glm::Vec3::new(0.0, 0.0, 1.0),
            tangent: nalgebra_glm::Vec3::new(1.0, 0.0, 0.0),
            texcoord: nalgebra_glm::Vec2::new(1.0, 0.0),
        },
        vk_sample_common::Vertex {
            position: nalgebra_glm::Vec3::new(0.0, 1.0, 0.0),
            normal: nalgebra_glm::Vec3::new(0.0, 0.0, 1.0),
            tangent: nalgebra_glm::Vec3::new(1.0, 0.0, 0.0),
            texcoord: nalgebra_glm::Vec2::new(0.0, 1.0),
        },
    ];

    let vertex_buffer_size = vertices.len() * std::mem::size_of::<vk_sample_common::Vertex>();
    let temporary_vertex_buffer_create_info = ash::vk::BufferCreateInfo::builder()
        .size(vertex_buffer_size as u64)
        .usage(ash::vk::BufferUsageFlags::TRANSFER_SRC)
        .build();
    let temporary_vertex_buffer_alloc_info = vk_mem::AllocationCreateInfo {
        usage: vk_mem::MemoryUsage::CpuToGpu,
        ..vk_mem::AllocationCreateInfo::default()
    };
    let (
        temporary_vertex_buffer,
        temporary_vertex_buffer_allocation,
        temporary_vertex_buffer_allocation_info,
    ) = allocator
        .create_buffer(
            &temporary_vertex_buffer_create_info,
            &temporary_vertex_buffer_alloc_info,
        )
        .expect("一時頂点バッファを作成できない");
    defer! { allocator.destroy_buffer(temporary_vertex_buffer, &temporary_vertex_buffer_allocation).unwrap(); }

    let vertex_buffer_create_info = ash::vk::BufferCreateInfo::builder()
        .size(vertex_buffer_size as u64)
        .usage(ash::vk::BufferUsageFlags::VERTEX_BUFFER | ash::vk::BufferUsageFlags::TRANSFER_DST)
        .build();
    let vertex_buffer_alloc_info = vk_mem::AllocationCreateInfo {
        usage: vk_mem::MemoryUsage::GpuOnly,
        ..vk_mem::AllocationCreateInfo::default()
    };
    let (vertex_buffer, vertex_buffer_allocation, vertex_buffer_allocation_info) = allocator
        .create_buffer(&vertex_buffer_create_info, &vertex_buffer_alloc_info)
        .expect("頂点バッファを作成できない");
    defer! { allocator.destroy_buffer(vertex_buffer, &vertex_buffer_allocation).unwrap(); }

    let mapped = allocator
        .map_memory(&temporary_vertex_buffer_allocation)
        .expect("バッファをマップできない");
    defer! { allocator.unmap_memory(&temporary_vertex_buffer_allocation).unwrap(); }

    unsafe {
        std::ptr::copy::<u8>(vertices.as_ptr() as *const u8, mapped, vertex_buffer_size);
    }

    //  13_create_semaphore
    let mut semaphores = Vec::<Semaphores>::with_capacity(swapchain_image_count as usize);
    for i in 0..swapchain_image_count {
        let fence = unsafe { device.create_fence(&ash::vk::FenceCreateInfo::builder().flags(ash::vk::FenceCreateFlags::SIGNALED).build(), None).unwrap() };
        let image_acquired_semaphore = unsafe { device.create_semaphore(&ash::vk::SemaphoreCreateInfo::builder().build(), None).unwrap() };
        let draw_complete_semaphore = unsafe { device.create_semaphore(&ash::vk::SemaphoreCreateInfo::builder().build(), None).unwrap() };
        let image_ownership_semaphore = unsafe { device.create_semaphore(&ash::vk::SemaphoreCreateInfo::builder().build(), None).unwrap() };

        semaphores.push(Semaphores {
            device: &device,
            fence: fence,
            image_acquired_semaphore: image_acquired_semaphore,
            draw_complete_semaphore: draw_complete_semaphore,
            image_ownership_semaphore: image_ownership_semaphore
        })
    }

    //  14_create_command_buffer
    let graphics_command_buffers = unsafe {
        device
            .allocate_command_buffers(
                &ash::vk::CommandBufferAllocateInfo::builder()
                    .command_pool(graphics_command_pool)
                    .level(ash::vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(swapchain_image_count * 3)
                    .build(),
            )
            .unwrap()
    };

    //  15_draw
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
struct Semaphores<'a> {
    pub device: &'a ash::Device,
    pub fence: ash::vk::Fence,
    pub image_acquired_semaphore: ash::vk::Semaphore,
    pub draw_complete_semaphore: ash::vk::Semaphore,
    pub image_ownership_semaphore: ash::vk::Semaphore,
}

impl Drop for Semaphores<'_> {
    fn drop(&mut self) {
        if self.fence.as_raw() != 0 {
            unsafe {
                self.device.destroy_fence(self.fence, None);
            }
        }
        if self.image_acquired_semaphore.as_raw() != 0 {
            unsafe {
                self.device
                    .destroy_semaphore(self.image_acquired_semaphore, None);
            }
        }
        if self.draw_complete_semaphore.as_raw() != 0 {
            unsafe {
                self.device
                    .destroy_semaphore(self.draw_complete_semaphore, None);
            }
        }
        if self.image_ownership_semaphore.as_raw() != 0 {
            unsafe {
                self.device
                    .destroy_semaphore(self.image_ownership_semaphore, None);
            }
        }
    }
}
