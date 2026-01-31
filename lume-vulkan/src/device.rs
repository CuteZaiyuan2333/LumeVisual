use ash::{vk};
use log::{info};
use std::sync::{Arc, Mutex};
use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;
use lume_core::{LumeError, LumeResult};

pub struct VulkanDeviceInner {
    pub allocator: Option<Arc<Mutex<Allocator>>>,
    pub descriptor_pool: vk::DescriptorPool,
    pub graphics_queue_index: u32,
    pub present_queue: vk::Queue, 
    pub graphics_queue: vk::Queue,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub instance: ash::Instance,
}

#[derive(Clone)]
pub struct VulkanDevice {
    pub inner: Arc<VulkanDeviceInner>,
}

impl std::ops::Deref for VulkanDevice {
    type Target = VulkanDeviceInner;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl VulkanDevice {
    pub fn new(
        instance: ash::Instance,
        device: ash::Device,
        graphics_queue: vk::Queue,
        present_queue: vk::Queue,
        graphics_queue_index: u32,
        allocator: Option<Arc<Mutex<Allocator>>>,
        physical_device: vk::PhysicalDevice,
    ) -> Self {
        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLER,
                descriptor_count: 1000,
            },
        ];

        let pool_info = vk::DescriptorPoolCreateInfo {
            pool_size_count: pool_sizes.len() as u32,
            p_pool_sizes: pool_sizes.as_ptr(),
            max_sets: 1000,
            ..Default::default()
        };

        let descriptor_pool = unsafe {
            device.create_descriptor_pool(&pool_info, None).expect("Failed to create descriptor pool")
        };

        Self {
            inner: Arc::new(VulkanDeviceInner {
                instance,
                device,
                physical_device,
                graphics_queue,
                present_queue,
                graphics_queue_index,
                descriptor_pool,
                allocator,
            }),
        }
    }
}

impl Drop for VulkanDeviceInner {
    fn drop(&mut self) {
        unsafe {
            info!("Destroying Vulkan Device and Descriptor Pool");
            // Explicitly drop allocator BEFORE destroying device
            self.allocator.take();
            
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

    fn wait_idle(&self) -> LumeResult<()> {
        unsafe {
            self.inner.device.device_wait_idle()
                .map_err(|e| LumeError::BackendError(format!("Device wait idle failed: {}", e)))
        }
    }

    fn create_semaphore(&self) -> LumeResult<Self::Semaphore> {
        let create_info = vk::SemaphoreCreateInfo::default();
        let semaphore = unsafe {
            self.inner.device.create_semaphore(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create semaphore: {}", e)))?
        };
        Ok(crate::VulkanSemaphore {
            semaphore,
            device: self.inner.device.clone(),
        })
    }

    fn create_command_pool(&self) -> LumeResult<Self::CommandPool> {
        let create_info = vk::CommandPoolCreateInfo {
            queue_family_index: self.inner.graphics_queue_index,
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            ..Default::default()
        };

        let pool = unsafe {
            self.inner.device.create_command_pool(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create command pool: {}", e)))?
        };

        Ok(crate::VulkanCommandPool {
            pool,
            device: self.inner.device.clone(),
        })
    }

    fn submit(
        &self,
        command_buffers: &[&Self::CommandBuffer],
        wait_semaphores: &[&Self::Semaphore],
        signal_semaphores: &[&Self::Semaphore],
    ) -> LumeResult<()> {
        let vk_command_buffers: Vec<vk::CommandBuffer> = command_buffers.iter().map(|cb| cb.buffer).collect();
        let vk_wait_semaphores: Vec<vk::Semaphore> = wait_semaphores.iter().map(|s| s.semaphore).collect();
        let vk_signal_semaphores: Vec<vk::Semaphore> = signal_semaphores.iter().map(|s| s.semaphore).collect();

        let wait_stages = vec![vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT; vk_wait_semaphores.len()];

        let submit_info = vk::SubmitInfo {
            wait_semaphore_count: vk_wait_semaphores.len() as u32,
            p_wait_semaphores: vk_wait_semaphores.as_ptr(),
            p_wait_dst_stage_mask: wait_stages.as_ptr(),
            command_buffer_count: vk_command_buffers.len() as u32,
            p_command_buffers: vk_command_buffers.as_ptr(),
            signal_semaphore_count: vk_signal_semaphores.len() as u32,
            p_signal_semaphores: vk_signal_semaphores.as_ptr(),
            ..Default::default()
        };

        unsafe {
            self.inner.device.queue_submit(self.inner.graphics_queue, &[submit_info], vk::Fence::null())
                .map_err(|e| LumeError::SubmissionFailed(format!("Failed to submit command buffers: {}", e)))
        }
    }

    fn create_shader_module(&self, code: &[u32]) -> LumeResult<Self::ShaderModule> {
        let create_info = vk::ShaderModuleCreateInfo {
            code_size: code.len() * 4,
            p_code: code.as_ptr(),
            ..Default::default()
        };

        let module = unsafe {
            self.inner.device.create_shader_module(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create shader module: {}", e)))?
        };

        Ok(crate::VulkanShaderModule {
            module,
            device: self.inner.device.clone(),
        })
    }

    fn create_render_pass(&self, descriptor: lume_core::device::RenderPassDescriptor) -> LumeResult<Self::RenderPass> {
        let mut attachments = Vec::new();
        let mut has_depth = false;

        // Color attachment
        let color_format = match descriptor.color_format {
            lume_core::device::TextureFormat::Bgra8UnormSrgb => vk::Format::B8G8R8A8_SRGB,
            lume_core::device::TextureFormat::Rgba8UnormSrgb => vk::Format::R8G8B8A8_SRGB,
            lume_core::device::TextureFormat::Rgba8Unorm => vk::Format::R8G8B8A8_UNORM,
            lume_core::device::TextureFormat::Depth32Float => return Err(LumeError::Generic("Cannot use Depth32Float as color format")),
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
                _ => return Err(LumeError::Generic("Only Depth32Float is supported for depth stencil format currently")),
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
            self.inner.device.create_render_pass(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create render pass: {}", e)))?
        };

        Ok(crate::VulkanRenderPass {
            render_pass,
            device: self.inner.device.clone(),
        })
    }

    fn create_pipeline_layout(&self, descriptor: lume_core::device::PipelineLayoutDescriptor<Self>) -> LumeResult<Self::PipelineLayout> {
        let set_layouts: Vec<vk::DescriptorSetLayout> = descriptor.bind_group_layouts.iter().map(|l| l.layout).collect();
        
        let create_info = vk::PipelineLayoutCreateInfo {
            set_layout_count: set_layouts.len() as u32,
            p_set_layouts: set_layouts.as_ptr(),
            ..Default::default()
        };

        let layout = unsafe {
            self.inner.device.create_pipeline_layout(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create pipeline layout: {}", e)))?
        };

        Ok(crate::VulkanPipelineLayout {
            layout,
            set_layouts,
            device: self.inner.device.clone(),
        })
    }

    fn create_compute_pipeline(&self, descriptor: lume_core::device::ComputePipelineDescriptor<Self>) -> LumeResult<Self::ComputePipeline> {
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
            self.inner.device.create_compute_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .map_err(|(_, e)| LumeError::PipelineCreationFailed(format!("Failed to create compute pipeline: {:?}", e)))?
        };

        Ok(crate::VulkanComputePipeline {
            pipeline: pipelines[0],
            layout: descriptor.layout.layout,
            device: self.inner.device.clone(),
        })
    }

    fn create_graphics_pipeline(&self, descriptor: lume_core::device::GraphicsPipelineDescriptor<Self>) -> LumeResult<Self::GraphicsPipeline> {
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
            self.inner.device.create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .map_err(|(_, e)| LumeError::PipelineCreationFailed(format!("Failed to create graphics pipeline: {:?}", e)))?
        };

        Ok(crate::VulkanGraphicsPipeline {
            pipeline: pipelines[0],
            layout: descriptor.layout.layout,
            device: self.inner.device.clone(),
        })
    }

    fn create_framebuffer(&self, descriptor: lume_core::device::FramebufferDescriptor<Self>) -> LumeResult<Self::Framebuffer> {
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
            self.inner.device.create_framebuffer(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create framebuffer: {}", e)))?
        };

        Ok(crate::VulkanFramebuffer {
            framebuffer,
            width: descriptor.width,
            height: descriptor.height,
            device: self.inner.device.clone(),
        })
    }

    fn create_buffer(&self, descriptor: lume_core::device::BufferDescriptor) -> LumeResult<Self::Buffer> {
        let mut usage = vk::BufferUsageFlags::empty();
        if descriptor.usage.0 & lume_core::device::BufferUsage::VERTEX.0 != 0 { usage |= vk::BufferUsageFlags::VERTEX_BUFFER; }
        if descriptor.usage.0 & lume_core::device::BufferUsage::INDEX.0 != 0 { usage |= vk::BufferUsageFlags::INDEX_BUFFER; }
        if descriptor.usage.0 & lume_core::device::BufferUsage::UNIFORM.0 != 0 { usage |= vk::BufferUsageFlags::UNIFORM_BUFFER; }
        if descriptor.usage.0 & lume_core::device::BufferUsage::STORAGE.0 != 0 { usage |= vk::BufferUsageFlags::STORAGE_BUFFER; }
        if descriptor.usage.0 & lume_core::device::BufferUsage::COPY_SRC.0 != 0 { usage |= vk::BufferUsageFlags::TRANSFER_SRC; }
        if descriptor.usage.0 & lume_core::device::BufferUsage::COPY_DST.0 != 0 { usage |= vk::BufferUsageFlags::TRANSFER_DST; }

        let create_info = vk::BufferCreateInfo {
            size: descriptor.size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe {
            self.inner.device.create_buffer(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create buffer: {}", e)))?
        };
        log::info!("Vulkan buffer handle created: {:?}", buffer);

        let requirements = unsafe { self.inner.device.get_buffer_memory_requirements(buffer) };
        log::info!("Buffer requirements: {:?}", requirements);
        let location = if descriptor.mapped_at_creation {
            MemoryLocation::CpuToGpu
        } else {
            MemoryLocation::GpuOnly
        };

        let allocator = self.inner.allocator.as_ref().ok_or_else(|| LumeError::BackendError("Allocator not initialized".to_string()))?;
        log::info!("Allocating memory from GPU allocator...");
        let allocation = allocator.lock().unwrap().allocate(&AllocationCreateDesc {
            name: "Generic Buffer",
            requirements,
            location,
            linear: true,
            allocation_scheme: AllocationScheme::DedicatedBuffer(buffer),
        }).map_err(|e| LumeError::BackendError(format!("Failed to allocate buffer memory: {}", e)))?;
        log::info!("Memory allocated successfully. Binding...");

        unsafe {
            self.inner.device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .map_err(|e| LumeError::BackendError(format!("Failed to bind buffer memory: {}", e)))?;
        }

        Ok(crate::VulkanBuffer {
            buffer,
            allocation,
            size: descriptor.size,
            allocator: allocator.clone(),
            device: self.clone(),
        })
    }

    fn create_bind_group_layout(&self, descriptor: lume_core::device::BindGroupLayoutDescriptor) -> LumeResult<Self::BindGroupLayout> {
        let mut entries = Vec::new();
        let mut type_map = std::collections::HashMap::new();

        for entry in descriptor.entries {
            let vk_type = match entry.ty {
                lume_core::device::BindingType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
                lume_core::device::BindingType::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
                lume_core::device::BindingType::SampledTexture => vk::DescriptorType::SAMPLED_IMAGE,
                lume_core::device::BindingType::Sampler => vk::DescriptorType::SAMPLER,
            };

            let mut stage_flags = vk::ShaderStageFlags::empty();
            if entry.visibility.0 & lume_core::device::ShaderStage::VERTEX.0 != 0 { stage_flags |= vk::ShaderStageFlags::VERTEX; }
            if entry.visibility.0 & lume_core::device::ShaderStage::FRAGMENT.0 != 0 { stage_flags |= vk::ShaderStageFlags::FRAGMENT; }
            if entry.visibility.0 & lume_core::device::ShaderStage::COMPUTE.0 != 0 { stage_flags |= vk::ShaderStageFlags::COMPUTE; }

            entries.push(vk::DescriptorSetLayoutBinding {
                binding: entry.binding,
                descriptor_type: vk_type,
                descriptor_count: 1,
                stage_flags,
                ..Default::default()
            });
            type_map.insert(entry.binding, entry.ty);
        }

        let create_info = vk::DescriptorSetLayoutCreateInfo {
            binding_count: entries.len() as u32,
            p_bindings: entries.as_ptr(),
            ..Default::default()
        };

        let layout = unsafe {
            self.inner.device.create_descriptor_set_layout(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create bind group layout: {}", e)))?
        };

        Ok(crate::VulkanBindGroupLayout {
            layout,
            entries: type_map,
            device: self.inner.device.clone(),
        })
    }

    fn create_bind_group(&self, descriptor: lume_core::device::BindGroupDescriptor<Self>) -> LumeResult<Self::BindGroup> {
        let allocate_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool: self.inner.descriptor_pool,
            descriptor_set_count: 1,
            p_set_layouts: &descriptor.layout.layout,
            ..Default::default()
        };

        let sets = unsafe {
            self.inner.device.allocate_descriptor_sets(&allocate_info)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to allocate bind group: {}", e)))?
        };

        let set = sets[0];
        
        // 1. Pre-collect all resources to ensure stable addresses
        let mut final_buffer_infos = Vec::new();
        let mut final_image_infos = Vec::new();
        
        // Collect into stable vectors first
        for entry in &descriptor.entries {
            match entry.resource {
                lume_core::device::BindingResource::Buffer(buf) => {
                    final_buffer_infos.push(vk::DescriptorBufferInfo {
                        buffer: buf.buffer,
                        offset: 0,
                        range: buf.size,
                    });
                }
                lume_core::device::BindingResource::TextureView(view) => {
                    final_image_infos.push(vk::DescriptorImageInfo {
                        image_view: view.view,
                        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        ..Default::default()
                    });
                }
                lume_core::device::BindingResource::Sampler(sampler) => {
                    final_image_infos.push(vk::DescriptorImageInfo {
                        sampler: sampler.sampler,
                        ..Default::default()
                    });
                }
            }
        }
        
        let mut buffer_pointer = 0;
        let mut image_pointer = 0;
        let mut writes = Vec::new();
        
        // 2. Re-iterate to build writes using stable references from final_buffer_infos/final_image_infos
        for entry in &descriptor.entries {
            let ty = descriptor.layout.entries.get(&entry.binding).ok_or_else(|| LumeError::Generic("Unknown binding in bind group"))?;
            match entry.resource {
                lume_core::device::BindingResource::Buffer(_) => {
                    let vk_ty = match ty {
                        lume_core::device::BindingType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
                        lume_core::device::BindingType::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
                        _ => return Err(LumeError::Generic("Mismatched binding type for buffer")),
                    };
                    writes.push(vk::WriteDescriptorSet {
                        dst_set: set,
                        dst_binding: entry.binding,
                        descriptor_count: 1,
                        descriptor_type: vk_ty,
                        p_buffer_info: &final_buffer_infos[buffer_pointer],
                        ..Default::default()
                    });
                    buffer_pointer += 1;
                }
                lume_core::device::BindingResource::TextureView(_) => {
                    writes.push(vk::WriteDescriptorSet {
                        dst_set: set,
                        dst_binding: entry.binding,
                        descriptor_count: 1,
                        descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
                        p_image_info: &final_image_infos[image_pointer],
                        ..Default::default()
                    });
                    image_pointer += 1;
                }
                lume_core::device::BindingResource::Sampler(_) => {
                    writes.push(vk::WriteDescriptorSet {
                        dst_set: set,
                        dst_binding: entry.binding,
                        descriptor_count: 1,
                        descriptor_type: vk::DescriptorType::SAMPLER,
                        p_image_info: &final_image_infos[image_pointer],
                        ..Default::default()
                    });
                    image_pointer += 1;
                }
            }
        }

        unsafe {
            self.inner.device.update_descriptor_sets(&writes, &[]);
        }

        Ok(crate::VulkanBindGroup { set })
    }

    fn create_texture(&self, descriptor: lume_core::device::TextureDescriptor) -> LumeResult<Self::Texture> {
        let mut usage = vk::ImageUsageFlags::empty();
        if descriptor.usage.0 & lume_core::device::TextureUsage::TEXTURE_BINDING.0 != 0 { usage |= vk::ImageUsageFlags::SAMPLED; }
        if descriptor.usage.0 & lume_core::device::TextureUsage::STORAGE_BINDING.0 != 0 { usage |= vk::ImageUsageFlags::STORAGE; }
        if descriptor.usage.0 & lume_core::device::TextureUsage::RENDER_ATTACHMENT.0 != 0 { usage |= vk::ImageUsageFlags::COLOR_ATTACHMENT; }
        if descriptor.usage.0 & lume_core::device::TextureUsage::DEPTH_STENCIL_ATTACHMENT.0 != 0 { usage |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT; }
        if descriptor.usage.0 & lume_core::device::TextureUsage::COPY_SRC.0 != 0 { usage |= vk::ImageUsageFlags::TRANSFER_SRC; }
        if descriptor.usage.0 & lume_core::device::TextureUsage::COPY_DST.0 != 0 { usage |= vk::ImageUsageFlags::TRANSFER_DST; }

        let format = map_texture_format(descriptor.format);
        let create_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            format,
            extent: vk::Extent3D { width: descriptor.width, height: descriptor.height, depth: 1 },
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            ..Default::default()
        };

        let image = unsafe {
            self.inner.device.create_image(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create texture ({}x{}): {}", descriptor.width, descriptor.height, e)))?
        };

        let requirements = unsafe { self.inner.device.get_image_memory_requirements(image) };
        let allocator = self.inner.allocator.as_ref().ok_or_else(|| LumeError::BackendError("Allocator not initialized".to_string()))?;
        
        let allocation = allocator.lock().unwrap().allocate(&AllocationCreateDesc {
            name: "Lume_Texture",
            requirements,
            location: MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        }).map_err(|e| LumeError::BackendError(format!("Failed to allocate texture memory: {}", e)))?;

        unsafe {
            self.inner.device.bind_image_memory(image, allocation.memory(), allocation.offset())
                .map_err(|e| LumeError::BackendError(format!("Failed to bind texture memory: {}", e)))?;
        }

        Ok(crate::VulkanTexture {
            image,
            allocation,
            format,
            width: descriptor.width,
            height: descriptor.height,
            allocator: allocator.clone(),
            device: self.inner.device.clone(),
        })
    }

    fn create_texture_view(&self, texture: &Self::Texture, descriptor: lume_core::device::TextureViewDescriptor) -> LumeResult<Self::TextureView> {
        let view_format = descriptor.format.map(map_texture_format).unwrap_or(texture.format);
        
        // Automatically determine aspect mask based on format
        let aspect_mask = if is_depth_format(view_format) {
            vk::ImageAspectFlags::DEPTH
        } else {
            vk::ImageAspectFlags::COLOR
        };

        let create_info = vk::ImageViewCreateInfo {
            image: texture.image,
            view_type: vk::ImageViewType::TYPE_2D,
            format: view_format,
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
            self.inner.device.create_image_view(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create texture view: {}", e)))?
        };

        Ok(crate::VulkanTextureView {
            view,
            device: self.inner.device.clone(),
        })
    }

    fn create_sampler(&self, descriptor: lume_core::device::SamplerDescriptor) -> LumeResult<Self::Sampler> {
        let create_info = vk::SamplerCreateInfo {
            mag_filter: map_filter(descriptor.mag_filter),
            min_filter: map_filter(descriptor.min_filter),
            address_mode_u: map_address_mode(descriptor.address_mode_u),
            address_mode_v: map_address_mode(descriptor.address_mode_v),
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            ..Default::default()
        };

        let sampler = unsafe {
            self.inner.device.create_sampler(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create sampler: {}", e)))?
        };

        Ok(crate::VulkanSampler {
            sampler,
            device: self.inner.device.clone(),
        })
    }

    fn create_swapchain(
        &self,
        surface: &impl lume_core::instance::Surface,
        descriptor: lume_core::device::SwapchainDescriptor,
    ) -> LumeResult<Self::Swapchain> {
        let vk_surface = unsafe {
             &*(surface as *const dyn lume_core::instance::Surface as *const crate::VulkanSurface)
        };
        
        let surface_loader = &vk_surface.surface_loader;
        let surface_khr = vk_surface.surface;

        let capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(self.inner.physical_device, surface_khr)
                .map_err(|e| LumeError::SurfaceCreationFailed(format!("Failed to query surface capabilities: {}", e)))?
        };

        let formats = unsafe {
            surface_loader.get_physical_device_surface_formats(self.inner.physical_device, surface_khr)
                .map_err(|e| LumeError::SurfaceCreationFailed(format!("Failed to query surface formats: {}", e)))?
        };
        let format = formats.iter().find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        }).unwrap_or(&formats[0]);

        let present_modes = unsafe {
            surface_loader.get_physical_device_surface_present_modes(self.inner.physical_device, surface_khr)
                .map_err(|e| LumeError::SurfaceCreationFailed(format!("Failed to query present modes: {}", e)))?
        };
        let present_mode = present_modes.iter().cloned().find(|&m| m == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);

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

        let swapchain_loader = ash::khr::swapchain::Device::new(&self.inner.instance, &self.inner.device);
        let swapchain = unsafe {
            swapchain_loader.create_swapchain(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create swapchain: {}", e)))?
        };

        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain)
            .map_err(|e| LumeError::BackendError(format!("Failed to get swapchain images: {}", e)))? };
            
        let mut image_views = Vec::new();
        for &image in &images {
            let iv_create_info = vk::ImageViewCreateInfo {
                image,
                view_type: vk::ImageViewType::TYPE_2D,
                format: format.format,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };
            let view = unsafe { self.inner.device.create_image_view(&iv_create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create swapchain image view: {}", e)))? };
            image_views.push(crate::VulkanTextureView {
                view,
                device: self.inner.device.clone(),
            });
        }

        let semaphore_create_info = vk::SemaphoreCreateInfo::default();
        let mut image_available_semaphores = Vec::new();
        for _ in 0..1 {
            let sema = unsafe { self.inner.device.create_semaphore(&semaphore_create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create swapchain semaphore: {}", e)))? };
            image_available_semaphores.push(sema);
        }

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
            device: self.inner.device.clone(),
            present_queue: self.inner.present_queue,
        })
    }
}

fn map_texture_format(format: lume_core::device::TextureFormat) -> vk::Format {
    match format {
        lume_core::device::TextureFormat::Bgra8UnormSrgb => vk::Format::B8G8R8A8_SRGB,
        lume_core::device::TextureFormat::Rgba8UnormSrgb => vk::Format::R8G8B8A8_SRGB,
        lume_core::device::TextureFormat::Rgba8Unorm => vk::Format::R8G8B8A8_UNORM,
        lume_core::device::TextureFormat::Depth32Float => vk::Format::D32_SFLOAT,
    }
}

fn map_filter(filter: lume_core::device::FilterMode) -> vk::Filter {
    match filter {
        lume_core::device::FilterMode::Nearest => vk::Filter::NEAREST,
        lume_core::device::FilterMode::Linear => vk::Filter::LINEAR,
    }
}

fn map_address_mode(mode: lume_core::device::AddressMode) -> vk::SamplerAddressMode {
    match mode {
        lume_core::device::AddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
        lume_core::device::AddressMode::MirrorRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
        lume_core::device::AddressMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
    }
}

fn is_depth_format(format: vk::Format) -> bool {
    matches!(
        format,
        vk::Format::D16_UNORM
            | vk::Format::X8_D24_UNORM_PACK32
            | vk::Format::D32_SFLOAT
            | vk::Format::S8_UINT
            | vk::Format::D16_UNORM_S8_UINT
            | vk::Format::D24_UNORM_S8_UINT
            | vk::Format::D32_SFLOAT_S8_UINT
    )
}
