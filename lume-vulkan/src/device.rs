use ash::vk;
use log::{info, error};

pub struct VulkanDevice {
    pub instance: ash::Instance,
    pub device: ash::Device,
    pub physical_device: vk::PhysicalDevice,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue, 
    pub graphics_queue_index: u32,
    pub descriptor_pool: vk::DescriptorPool,
}

impl Drop for VulkanDevice {
    fn drop(&mut self) {
        unsafe {
            info!("Destroying Vulkan Device and Descriptor Pool");
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_device(None);
        }
    }
}



impl lume_core::Device for VulkanDevice {
    type CommandBuffer = crate::VulkanCommandBuffer;
    type CommandPool = crate::VulkanCommandPool;
    type Swapchain = crate::VulkanSwapchain;
    type ShaderModule = crate::VulkanShaderModule;
    type RenderPass = crate::VulkanRenderPass;
    type PipelineLayout = crate::VulkanPipelineLayout;
    type GraphicsPipeline = crate::VulkanGraphicsPipeline;
    type ComputePipeline = crate::VulkanComputePipeline;
    type Semaphore = crate::VulkanSemaphore;
    type Framebuffer = crate::VulkanFramebuffer;
    type TextureView = crate::VulkanTextureView;
    type Texture = crate::VulkanTexture;
    type Sampler = crate::VulkanSampler;
    type Buffer = crate::VulkanBuffer;
    type BindGroupLayout = crate::VulkanBindGroupLayout;
    type BindGroup = crate::VulkanBindGroup;

    fn wait_idle(&self) -> Result<(), &'static str> {
        unsafe {
            self.device.device_wait_idle()
                .map_err(|_| "Failed to wait for device idle")
        }
    }

    fn create_semaphore(&self) -> Result<Self::Semaphore, &'static str> {
        let create_info = vk::SemaphoreCreateInfo::default();
        let semaphore = unsafe {
            self.device.create_semaphore(&create_info, None)
                .map_err(|_| "Failed to create semaphore")?
        };
        Ok(crate::VulkanSemaphore {
            semaphore,
            device: self.device.clone(),
        })
    }

    fn create_command_pool(&self) -> Result<Self::CommandPool, &'static str> {
        let create_info = vk::CommandPoolCreateInfo {
            queue_family_index: self.graphics_queue_index,
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            ..Default::default()
        };

        let pool = unsafe {
            self.device.create_command_pool(&create_info, None)
                .map_err(|_| "Failed to create command pool")?
        };

        Ok(crate::VulkanCommandPool {
            pool,
            device: self.device.clone(),
        })
    }

    fn submit(
        &self,
        command_buffers: &[&Self::CommandBuffer],
        wait_semaphores: &[&Self::Semaphore],
        signal_semaphores: &[&Self::Semaphore],
    ) -> Result<(), &'static str> {
        let vk_command_buffers: Vec<vk::CommandBuffer> = command_buffers.iter().map(|cb| cb.buffer).collect();
        let vk_wait_semaphores: Vec<vk::Semaphore> = wait_semaphores.iter().map(|s| s.semaphore).collect();
        let vk_signal_semaphores: Vec<vk::Semaphore> = signal_semaphores.iter().map(|s| s.semaphore).collect();

        let wait_dst_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

        let submit_info = vk::SubmitInfo {
            wait_semaphore_count: vk_wait_semaphores.len() as u32,
            p_wait_semaphores: vk_wait_semaphores.as_ptr(),
            p_wait_dst_stage_mask: wait_dst_stage_mask.as_ptr(),
            command_buffer_count: vk_command_buffers.len() as u32,
            p_command_buffers: vk_command_buffers.as_ptr(),
            signal_semaphore_count: vk_signal_semaphores.len() as u32,
            p_signal_semaphores: vk_signal_semaphores.as_ptr(),
            ..Default::default()
        };

        unsafe {
            self.device.queue_submit(self.graphics_queue, &[submit_info], vk::Fence::null())
                .map_err(|_| "Failed to submit queue")
        }
    }

    fn create_shader_module(&self, code: &[u32]) -> Result<Self::ShaderModule, &'static str> {
        let create_info = vk::ShaderModuleCreateInfo {
            code_size: code.len() * 4,
            p_code: code.as_ptr(),
            ..Default::default()
        };

        let module = unsafe {
            self.device.create_shader_module(&create_info, None)
                .map_err(|_| "Failed to create shader module")?
        };

        Ok(crate::VulkanShaderModule {
            module,
            device: self.device.clone(),
        })
    }

    fn create_render_pass(&self, descriptor: lume_core::device::RenderPassDescriptor) -> Result<Self::RenderPass, &'static str> {
        let mut attachments = Vec::new();
        let mut has_depth = false;

        // Color attachment
        let color_format = match descriptor.color_format {
            lume_core::device::TextureFormat::Bgra8UnormSrgb => vk::Format::B8G8R8A8_SRGB,
            lume_core::device::TextureFormat::Rgba8UnormSrgb => vk::Format::R8G8B8A8_SRGB,
            lume_core::device::TextureFormat::Rgba8Unorm => vk::Format::R8G8B8A8_UNORM,
            lume_core::device::TextureFormat::Depth32Float => return Err("Cannot use Depth32Float as color format"),
        };

        attachments.push(vk::AttachmentDescription {
            format: color_format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        });

        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };

        // Depth attachment
        let mut depth_attachment_ref = vk::AttachmentReference::default();
        if let Some(df) = descriptor.depth_stencil_format {
            let depth_format = match df {
                lume_core::device::TextureFormat::Depth32Float => vk::Format::D32_SFLOAT,
                _ => return Err("Only Depth32Float is supported for depth stencil format currently"),
            };

            attachments.push(vk::AttachmentDescription {
                format: depth_format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::DONT_CARE,
                stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                ..Default::default()
            });

            depth_attachment_ref = vk::AttachmentReference {
                attachment: 1,
                layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            };
            has_depth = true;
        }

        let subpass = vk::SubpassDescription {
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_ref,
            p_depth_stencil_attachment: if has_depth { &depth_attachment_ref } else { std::ptr::null() },
            ..Default::default()
        };

        let dependency = vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            ..Default::default()
        };

        let create_info = vk::RenderPassCreateInfo {
            attachment_count: attachments.len() as u32,
            p_attachments: attachments.as_ptr(),
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 1,
            p_dependencies: &dependency,
            ..Default::default()
        };

        let render_pass = unsafe {
            self.device.create_render_pass(&create_info, None)
                .map_err(|_| "Failed to create render pass")?
        };

        Ok(crate::VulkanRenderPass {
            render_pass,
            device: self.device.clone(),
        })
    }

    fn create_pipeline_layout(&self, descriptor: lume_core::device::PipelineLayoutDescriptor<Self>) -> Result<Self::PipelineLayout, &'static str> {
        let set_layouts: Vec<vk::DescriptorSetLayout> = descriptor.bind_group_layouts.iter().map(|l| l.layout).collect();
        
        let create_info = vk::PipelineLayoutCreateInfo {
            set_layout_count: set_layouts.len() as u32,
            p_set_layouts: set_layouts.as_ptr(),
            ..Default::default()
        };

        let layout = unsafe {
            self.device.create_pipeline_layout(&create_info, None)
                .map_err(|_| "Failed to create pipeline layout")?
        };

        Ok(crate::VulkanPipelineLayout {
            layout,
            set_layouts,
            device: self.device.clone(),
        })
    }

    fn create_compute_pipeline(&self, descriptor: lume_core::device::ComputePipelineDescriptor<Self>) -> Result<Self::ComputePipeline, &'static str> {
        let entry_name = std::ffi::CString::new("main").unwrap();

        let stage_info = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::COMPUTE,
            module: descriptor.shader.module,
            p_name: entry_name.as_ptr(),
            ..Default::default()
        };

        let create_info = vk::ComputePipelineCreateInfo {
            stage: stage_info,
            layout: descriptor.layout.layout,
            ..Default::default()
        };

        let pipelines = unsafe {
            self.device.create_compute_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .map_err(|_| "Failed to create compute pipeline")?
        };

        Ok(crate::VulkanComputePipeline {
            pipeline: pipelines[0],
            layout: descriptor.layout.layout,
            device: self.device.clone(),
        })
    }

    fn create_graphics_pipeline(&self, descriptor: lume_core::device::GraphicsPipelineDescriptor<Self>) -> Result<Self::GraphicsPipeline, &'static str> {
        let entry_name = std::ffi::CString::new("main").unwrap();

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::VERTEX,
                module: descriptor.vertex_shader.module,
                p_name: entry_name.as_ptr(),
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::FRAGMENT,
                module: descriptor.fragment_shader.module,
                p_name: entry_name.as_ptr(),
                ..Default::default()
            },
        ];

        let mut vertex_binding_descriptions = Vec::new();
        let mut vertex_attribute_descriptions = Vec::new();

        if let Some(layout) = &descriptor.vertex_layout {
            vertex_binding_descriptions.push(vk::VertexInputBindingDescription {
                binding: 0,
                stride: layout.array_stride,
                input_rate: vk::VertexInputRate::VERTEX,
            });

            for attr in &layout.attributes {
                vertex_attribute_descriptions.push(vk::VertexInputAttributeDescription {
                    location: attr.location,
                    binding: 0,
                    format: match attr.format {
                        lume_core::device::VertexFormat::Float32x2 => vk::Format::R32G32_SFLOAT,
                        lume_core::device::VertexFormat::Float32x3 => vk::Format::R32G32B32_SFLOAT,
                        lume_core::device::VertexFormat::Float32x4 => vk::Format::R32G32B32A32_SFLOAT,
                    },
                    offset: attr.offset,
                });
            }
        }

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            vertex_binding_description_count: vertex_binding_descriptions.len() as u32,
            p_vertex_binding_descriptions: vertex_binding_descriptions.as_ptr(),
            vertex_attribute_description_count: vertex_attribute_descriptions.len() as u32,
            p_vertex_attribute_descriptions: vertex_attribute_descriptions.as_ptr(),
            ..Default::default()
        };

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
            topology: match descriptor.primitive.topology {
                lume_core::device::PrimitiveTopology::TriangleList => vk::PrimitiveTopology::TRIANGLE_LIST,
            },
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        };

        let rasterizer = vk::PipelineRasterizationStateCreateInfo {
            depth_clamp_enable: vk::FALSE,
            rasterizer_discard_enable: vk::FALSE,
            polygon_mode: vk::PolygonMode::FILL,
            line_width: 1.0,
            cull_mode: vk::CullModeFlags::NONE,
            front_face: vk::FrontFace::CLOCKWISE,
            depth_bias_enable: vk::FALSE,
            ..Default::default()
        };

        let multisampling = vk::PipelineMultisampleStateCreateInfo {
            sample_shading_enable: vk::FALSE,
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            color_write_mask: vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A,
            blend_enable: vk::FALSE,
            ..Default::default()
        };

        let color_blending = vk::PipelineColorBlendStateCreateInfo {
            logic_op_enable: vk::FALSE,
            attachment_count: 1,
            p_attachments: &color_blend_attachment,
            ..Default::default()
        };

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo {
            dynamic_state_count: dynamic_states.len() as u32,
            p_dynamic_states: dynamic_states.as_ptr(),
            ..Default::default()
        };

        let viewport_state = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            scissor_count: 1,
            ..Default::default()
        };

        let depth_stencil_info = if let Some(ds) = &descriptor.depth_stencil {
            vk::PipelineDepthStencilStateCreateInfo {
                depth_test_enable: vk::TRUE,
                depth_write_enable: if ds.depth_write_enabled { vk::TRUE } else { vk::FALSE },
                depth_compare_op: match ds.depth_compare {
                    lume_core::device::CompareFunction::Never => vk::CompareOp::NEVER,
                    lume_core::device::CompareFunction::Less => vk::CompareOp::LESS,
                    lume_core::device::CompareFunction::Equal => vk::CompareOp::EQUAL,
                    lume_core::device::CompareFunction::LessEqual => vk::CompareOp::LESS_OR_EQUAL,
                    lume_core::device::CompareFunction::Greater => vk::CompareOp::GREATER,
                    lume_core::device::CompareFunction::NotEqual => vk::CompareOp::NOT_EQUAL,
                    lume_core::device::CompareFunction::GreaterEqual => vk::CompareOp::GREATER_OR_EQUAL,
                    lume_core::device::CompareFunction::Always => vk::CompareOp::ALWAYS,
                },
                depth_bounds_test_enable: vk::FALSE,
                stencil_test_enable: vk::FALSE,
                ..Default::default()
            }
        } else {
            vk::PipelineDepthStencilStateCreateInfo::default()
        };

        let create_info = vk::GraphicsPipelineCreateInfo {
            stage_count: 2,
            p_stages: shader_stages.as_ptr(),
            p_vertex_input_state: &vertex_input_info,
            p_input_assembly_state: &input_assembly,
            p_viewport_state: &viewport_state,
            p_rasterization_state: &rasterizer,
            p_multisample_state: &multisampling,
            p_color_blend_state: &color_blending,
            p_depth_stencil_state: &depth_stencil_info,
            p_dynamic_state: &dynamic_state_info,
            layout: descriptor.layout.layout,
            render_pass: descriptor.render_pass.render_pass,
            subpass: 0,
            ..Default::default()
        };

        let pipelines = unsafe {
            self.device.create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .map_err(|_| "Failed to create graphics pipeline")?
        };

        Ok(crate::VulkanGraphicsPipeline {
            pipeline: pipelines[0],
            layout: descriptor.layout.layout,
            device: self.device.clone(),
        })
    }

    fn create_framebuffer(&self, descriptor: lume_core::device::FramebufferDescriptor<Self>) -> Result<Self::Framebuffer, &'static str> {
        let vk_attachments: Vec<vk::ImageView> = descriptor.attachments.iter().map(|&a| a.view).collect();

        let create_info = vk::FramebufferCreateInfo {
            render_pass: descriptor.render_pass.render_pass,
            attachment_count: vk_attachments.len() as u32,
            p_attachments: vk_attachments.as_ptr(),
            width: descriptor.width,
            height: descriptor.height,
            layers: 1,
            ..Default::default()
        };

        let framebuffer = unsafe {
            self.device.create_framebuffer(&create_info, None)
                .map_err(|_| "Failed to create framebuffer")?
        };

        Ok(crate::VulkanFramebuffer {
            framebuffer,
            width: descriptor.width,
            height: descriptor.height,
            device: self.device.clone(),
        })
    }

    fn create_buffer(&self, descriptor: lume_core::device::BufferDescriptor) -> Result<Self::Buffer, &'static str> {
        let mut usage = vk::BufferUsageFlags::empty();
        if (descriptor.usage.0 & lume_core::device::BufferUsage::VERTEX.0) != 0 { usage |= vk::BufferUsageFlags::VERTEX_BUFFER; }
        if (descriptor.usage.0 & lume_core::device::BufferUsage::INDEX.0) != 0 { usage |= vk::BufferUsageFlags::INDEX_BUFFER; }
        if (descriptor.usage.0 & lume_core::device::BufferUsage::UNIFORM.0) != 0 { usage |= vk::BufferUsageFlags::UNIFORM_BUFFER; }
        if (descriptor.usage.0 & lume_core::device::BufferUsage::STORAGE.0) != 0 { usage |= vk::BufferUsageFlags::STORAGE_BUFFER; }
        if (descriptor.usage.0 & lume_core::device::BufferUsage::COPY_SRC.0) != 0 { usage |= vk::BufferUsageFlags::TRANSFER_SRC; }
        if (descriptor.usage.0 & lume_core::device::BufferUsage::COPY_DST.0) != 0 { usage |= vk::BufferUsageFlags::TRANSFER_DST; }

        let create_info = vk::BufferCreateInfo {
            size: descriptor.size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe {
            self.device.create_buffer(&create_info, None)
                .map_err(|_| "Failed to create buffer")?
        };

        let mem_requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };
        let mem_properties = unsafe { self.instance.get_physical_device_memory_properties(self.physical_device) };

        // Find memory type
        let memory_type_index = (0..mem_properties.memory_type_count)
            .find(|&i| {
                let suitable = (mem_requirements.memory_type_bits & (1 << i)) != 0;
                let properties = mem_properties.memory_types[i as usize].property_flags;
                // For now, assume we want host visible and coherent for all buffers
                let required = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
                suitable && (properties & required) == required
            })
            .ok_or("Failed to find suitable memory type")?;

        let alloc_info = vk::MemoryAllocateInfo {
            allocation_size: mem_requirements.size,
            memory_type_index,
            ..Default::default()
        };

        let memory = unsafe {
            self.device.allocate_memory(&alloc_info, None)
                .map_err(|_| "Failed to allocate buffer memory")?
        };

        unsafe {
            self.device.bind_buffer_memory(buffer, memory, 0)
                .map_err(|_| "Failed to bind buffer memory")?;
        }

        Ok(crate::VulkanBuffer {
            buffer,
            memory,
            size: descriptor.size,
            device: self.device.clone(),
        })
    }

    fn create_bind_group_layout(&self, descriptor: lume_core::device::BindGroupLayoutDescriptor) -> Result<Self::BindGroupLayout, &'static str> {
        let bindings: Vec<vk::DescriptorSetLayoutBinding> = descriptor.entries.iter().map(|entry| {
            vk::DescriptorSetLayoutBinding {
                binding: entry.binding,
                descriptor_type: match entry.ty {
                    lume_core::device::BindingType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
                    lume_core::device::BindingType::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
                    lume_core::device::BindingType::SampledTexture => vk::DescriptorType::SAMPLED_IMAGE,
                    lume_core::device::BindingType::Sampler => vk::DescriptorType::SAMPLER,
                },
                descriptor_count: 1,
                stage_flags: {
                    let mut flags = vk::ShaderStageFlags::empty();
                    if (entry.visibility.0 & lume_core::device::ShaderStage::VERTEX.0) != 0 { flags |= vk::ShaderStageFlags::VERTEX; }
                    if (entry.visibility.0 & lume_core::device::ShaderStage::FRAGMENT.0) != 0 { flags |= vk::ShaderStageFlags::FRAGMENT; }
                    if (entry.visibility.0 & lume_core::device::ShaderStage::COMPUTE.0) != 0 { flags |= vk::ShaderStageFlags::COMPUTE; }
                    flags
                },
                ..Default::default()
            }
        }).collect();

        let create_info = vk::DescriptorSetLayoutCreateInfo {
            binding_count: bindings.len() as u32,
            p_bindings: bindings.as_ptr(),
            ..Default::default()
        };

        let layout = unsafe {
            self.device.create_descriptor_set_layout(&create_info, None)
                .map_err(|_| "Failed to create descriptor set layout")?
        };

        let mut entries_map = std::collections::HashMap::new();
        for entry in &descriptor.entries {
            entries_map.insert(entry.binding, entry.ty);
        }

        Ok(crate::VulkanBindGroupLayout {
            layout,
            entries: entries_map,
            device: self.device.clone(),
        })
    }

    fn create_bind_group(&self, descriptor: lume_core::device::BindGroupDescriptor<Self>) -> Result<Self::BindGroup, &'static str> {
        let set_layouts = [descriptor.layout.layout];
        let alloc_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool: self.descriptor_pool,
            descriptor_set_count: 1,
            p_set_layouts: set_layouts.as_ptr(),
            ..Default::default()
        };

        let sets = unsafe {
            self.device.allocate_descriptor_sets(&alloc_info)
                .map_err(|_| "Failed to allocate descriptor set")?
        };
        let set = sets[0];

        let mut writes = Vec::new();
        // Keep resources alive during update
        let mut buffer_infos = Vec::new();
        let mut image_infos = Vec::new();

        for entry in &descriptor.entries {
            match entry.resource {
                lume_core::device::BindingResource::Buffer(buffer) => {
                    buffer_infos.push(vk::DescriptorBufferInfo {
                        buffer: buffer.buffer,
                        offset: 0,
                        range: buffer.size,
                    });
                    
                    let descriptor_type = match descriptor.layout.entries.get(&entry.binding) {
                        Some(lume_core::device::BindingType::UniformBuffer) => vk::DescriptorType::UNIFORM_BUFFER,
                        Some(lume_core::device::BindingType::StorageBuffer) => vk::DescriptorType::STORAGE_BUFFER,
                        _ => vk::DescriptorType::UNIFORM_BUFFER,
                    };

                    writes.push(vk::WriteDescriptorSet {
                        dst_set: set,
                        dst_binding: entry.binding,
                        dst_array_element: 0,
                        descriptor_count: 1,
                        descriptor_type,
                        p_buffer_info: &buffer_infos[buffer_infos.len() - 1],
                        ..Default::default()
                    });
                }
                lume_core::device::BindingResource::TextureView(view) => {
                    image_infos.push(vk::DescriptorImageInfo {
                        image_view: view.view,
                        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        ..Default::default()
                    });
                    writes.push(vk::WriteDescriptorSet {
                        dst_set: set,
                        dst_binding: entry.binding,
                        dst_array_element: 0,
                        descriptor_count: 1,
                        descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
                        p_image_info: &image_infos[image_infos.len() - 1],
                        ..Default::default()
                    });
                }
                lume_core::device::BindingResource::Sampler(sampler) => {
                    image_infos.push(vk::DescriptorImageInfo {
                        sampler: sampler.sampler,
                        ..Default::default()
                    });
                    writes.push(vk::WriteDescriptorSet {
                        dst_set: set,
                        dst_binding: entry.binding,
                        dst_array_element: 0,
                        descriptor_count: 1,
                        descriptor_type: vk::DescriptorType::SAMPLER,
                        p_image_info: &image_infos[image_infos.len() - 1],
                        ..Default::default()
                    });
                }
            }
        }

        unsafe {
            self.device.update_descriptor_sets(&writes, &[]);
        }

        Ok(crate::VulkanBindGroup {
            set,
        })
    }

    fn create_texture(&self, descriptor: lume_core::device::TextureDescriptor) -> Result<Self::Texture, &'static str> {
        let create_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            format: match descriptor.format {
                lume_core::device::TextureFormat::Bgra8UnormSrgb => vk::Format::B8G8R8A8_SRGB,
                lume_core::device::TextureFormat::Rgba8UnormSrgb => vk::Format::R8G8B8A8_SRGB,
                lume_core::device::TextureFormat::Rgba8Unorm => vk::Format::R8G8B8A8_UNORM,
                lume_core::device::TextureFormat::Depth32Float => vk::Format::D32_SFLOAT,
            },
            extent: vk::Extent3D {
                width: descriptor.width,
                height: descriptor.height,
                depth: descriptor.depth,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: {
                let mut usage = vk::ImageUsageFlags::empty();
                if (descriptor.usage.0 & lume_core::device::TextureUsage::TEXTURE_BINDING.0) != 0 { usage |= vk::ImageUsageFlags::SAMPLED; }
                if (descriptor.usage.0 & lume_core::device::TextureUsage::STORAGE_BINDING.0) != 0 { usage |= vk::ImageUsageFlags::STORAGE; }
                if (descriptor.usage.0 & lume_core::device::TextureUsage::RENDER_ATTACHMENT.0) != 0 { usage |= vk::ImageUsageFlags::COLOR_ATTACHMENT; }
                if (descriptor.usage.0 & lume_core::device::TextureUsage::DEPTH_STENCIL_ATTACHMENT.0) != 0 { usage |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT; }
                if (descriptor.usage.0 & lume_core::device::TextureUsage::COPY_SRC.0) != 0 { usage |= vk::ImageUsageFlags::TRANSFER_SRC; }
                if (descriptor.usage.0 & lume_core::device::TextureUsage::COPY_DST.0) != 0 { usage |= vk::ImageUsageFlags::TRANSFER_DST; }
                usage
            },
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            ..Default::default()
        };

        let image = unsafe {
            self.device.create_image(&create_info, None)
                .map_err(|_| "Failed to create image")?
        };

        // Memory allocation (Simplified, same logic as buffer)
        let mem_requirements = unsafe { self.device.get_image_memory_requirements(image) };
        let mem_properties = unsafe { self.instance.get_physical_device_memory_properties(self.physical_device) };
        let memory_type_index = (0..mem_properties.memory_type_count)
            .find(|&i| {
                let suitable = (mem_requirements.memory_type_bits & (1 << i)) != 0;
                let properties = mem_properties.memory_types[i as usize].property_flags;
                let required = vk::MemoryPropertyFlags::DEVICE_LOCAL;
                suitable && (properties & required) == required
            })
            .ok_or("Failed to find suitable memory type for image")?;

        let alloc_info = vk::MemoryAllocateInfo {
            allocation_size: mem_requirements.size,
            memory_type_index,
            ..Default::default()
        };

        let memory = unsafe {
            self.device.allocate_memory(&alloc_info, None)
                .map_err(|_| "Failed to allocate image memory")?
        };

        unsafe {
            self.device.bind_image_memory(image, memory, 0)
                .map_err(|_| "Failed to bind image memory")?;
        }

        Ok(crate::VulkanTexture {
            image,
            memory,
            device: self.device.clone(),
        })
    }

    fn create_texture_view(&self, texture: &Self::Texture, descriptor: lume_core::device::TextureViewDescriptor) -> Result<Self::TextureView, &'static str> {
        let format = match descriptor.format {
            Some(lume_core::device::TextureFormat::Bgra8UnormSrgb) => vk::Format::B8G8R8A8_SRGB,
            Some(lume_core::device::TextureFormat::Rgba8UnormSrgb) => vk::Format::R8G8B8A8_SRGB,
            Some(lume_core::device::TextureFormat::Rgba8Unorm) => vk::Format::R8G8B8A8_UNORM,
            Some(lume_core::device::TextureFormat::Depth32Float) => vk::Format::D32_SFLOAT,
            None => {
                vk::Format::R8G8B8A8_UNORM 
            }
        };

        let aspect_mask = if format == vk::Format::D32_SFLOAT {
            vk::ImageAspectFlags::DEPTH
        } else {
            vk::ImageAspectFlags::COLOR
        };

        let create_info = vk::ImageViewCreateInfo {
            image: texture.image,
            view_type: vk::ImageViewType::TYPE_2D,
            format,
            components: vk::ComponentMapping::default(),
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };

        let view = unsafe {
            self.device.create_image_view(&create_info, None)
                .map_err(|_| "Failed to create image view")?
        };

        Ok(crate::VulkanTextureView {
            view,
            device: self.device.clone(),
        })
    }

    fn create_sampler(&self, descriptor: lume_core::device::SamplerDescriptor) -> Result<Self::Sampler, &'static str> {
        let create_info = vk::SamplerCreateInfo {
            mag_filter: match descriptor.mag_filter {
                lume_core::device::FilterMode::Nearest => vk::Filter::NEAREST,
                lume_core::device::FilterMode::Linear => vk::Filter::LINEAR,
            },
            min_filter: match descriptor.min_filter {
                lume_core::device::FilterMode::Nearest => vk::Filter::NEAREST,
                lume_core::device::FilterMode::Linear => vk::Filter::LINEAR,
            },
            address_mode_u: match descriptor.address_mode_u {
                lume_core::device::AddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
                lume_core::device::AddressMode::MirrorRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
                lume_core::device::AddressMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            },
            address_mode_v: match descriptor.address_mode_v {
                lume_core::device::AddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
                lume_core::device::AddressMode::MirrorRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
                lume_core::device::AddressMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            },
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            ..Default::default()
        };

        let sampler = unsafe {
            self.device.create_sampler(&create_info, None)
                .map_err(|_| "Failed to create sampler")?
        };

        Ok(crate::VulkanSampler {
            sampler,
            device: self.device.clone(),
        })
    }

    fn create_swapchain(
        &self,
        surface: &impl lume_core::instance::Surface,
        descriptor: lume_core::device::SwapchainDescriptor,
    ) -> Result<Self::Swapchain, &'static str> {
        // Cast opaque surface to VulkanSurface. 
        // In a real generic system, we might need Any or downcast, 
        // but here we know the concrete types match because the Instance created them.
        // However, generic `impl Surface` prevents direct access unless we assume memory layout or use Any.
        // For this phase, let's assume usage of correct concrete types and "unsafe" cast if needed,
        // OR better, we can't easily downcast strictly without Any.
        // But since `lume-vulkan` knows `lume-core` traits, and we are in `lume-vulkan`,
        // we can try to unsafe pointer cast if we are confident, or change trait to exposing `as_any`.
        // Let's rely on the fact that we passed `impl Surface` but in `main` we passed `&surface` which is `VulkanSurface`.
        // Actually, the trait signature in `Device` is `surface: &impl crate::instance::Surface`.
        // `VulkanDevice` needs `VulkanSurface` to get `vk::SurfaceKHR`.
        
        // HACK: For now, unsafe transmute or pointer access. 
        // Real solution: Add `sys_handle()` to Surface trait returning Raw or a backend-specific unique ID/Handle.
        // Let's unsafe cast for now to unblock.
        let vk_surface = unsafe {
             &*(surface as *const _ as *const crate::VulkanSurface)
        };
        
        // 1. Query Surface Capabilities
        let surface_loader = &vk_surface.surface_loader;
        let surface_khr = vk_surface.surface;

        let capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(self.physical_device, surface_khr)
                .map_err(|_| "Failed to query surface capabilities")?
        };

        // 2. Select Format
        let formats = unsafe {
            surface_loader.get_physical_device_surface_formats(self.physical_device, surface_khr)
                .map_err(|_| "Failed to query surface formats")?
        };
        let format = formats.iter().find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        }).unwrap_or(&formats[0]);

        // 3. Select Present Mode
        let present_modes = unsafe {
            surface_loader.get_physical_device_surface_present_modes(self.physical_device, surface_khr)
                .map_err(|_| "Failed to query present modes")?
        };
        let present_mode = present_modes.iter().cloned().find(|&m| m == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO); // FIFO is guaranteed

        // 4. Extent
        let extent = if capabilities.current_extent.width != u32::MAX {
            capabilities.current_extent
        } else {
            vk::Extent2D {
                width: descriptor.width.clamp(capabilities.min_image_extent.width, capabilities.max_image_extent.width),
                height: descriptor.height.clamp(capabilities.min_image_extent.height, capabilities.max_image_extent.height),
            }
        };

        let image_count = (capabilities.min_image_count + 1).min(if capabilities.max_image_count > 0 { capabilities.max_image_count } else { u32::MAX });

        let create_info = vk::SwapchainCreateInfoKHR {
            surface: surface_khr,
            min_image_count: image_count,
            image_format: format.format,
            image_color_space: format.color_space,
            image_extent: extent,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            pre_transform: capabilities.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode,
            clipped: vk::TRUE,
            ..Default::default()
        };

        let swapchain_loader = ash::khr::swapchain::Device::new(&self.instance, &self.device);
        let swapchain = unsafe {
            swapchain_loader.create_swapchain(&create_info, None)
                .map_err(|e| {
                    error!("Failed to create swapchain: {:?}", e);
                    "Failed to create swapchain"
                })?
        };

        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain).unwrap() };
        let image_views: Vec<crate::VulkanTextureView> = images.iter().map(|&image| {
            let create_info = vk::ImageViewCreateInfo {
                image,
                view_type: vk::ImageViewType::TYPE_2D,
                format: format.format,
                components: vk::ComponentMapping::default(),
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };
            let view = unsafe { self.device.create_image_view(&create_info, None).unwrap() };
            crate::VulkanTextureView {
                view,
                device: self.device.clone(),
            }
        }).collect();

        // Sync objects
        let semaphore_create_info = vk::SemaphoreCreateInfo::default();
        let image_available_semaphores = (0..1).map(|_| { // just 1 for now
            unsafe { self.device.create_semaphore(&semaphore_create_info, None).unwrap() }
        }).collect();

        info!("Swapchain created ({:?}) with {} images", extent, images.len());

        Ok(crate::VulkanSwapchain {
            swapchain_loader,
            swapchain,
            images,
            image_views,
            extent,
            format: format.format,
            image_available_semaphores,
            current_frame: 0,
            device: self.device.clone(),
            present_queue: self.present_queue,
        })
    }
}
