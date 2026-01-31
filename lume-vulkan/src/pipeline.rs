use ash::vk;
use lume_core::{LumeError, LumeResult};

pub struct VulkanShaderModule {
    pub module: vk::ShaderModule,
    pub device: ash::Device,
}

impl Drop for VulkanShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_shader_module(self.module, None);
        }
    }
}

pub struct VulkanRenderPass {
    pub render_pass: vk::RenderPass,
    pub device: ash::Device,
}

impl Drop for VulkanRenderPass {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_render_pass(self.render_pass, None);
        }
    }
}

pub struct VulkanPipelineLayout {
    pub layout: vk::PipelineLayout,
    pub set_layouts: Vec<vk::DescriptorSetLayout>,
    pub device: ash::Device,
}

impl Drop for VulkanPipelineLayout {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline_layout(self.layout, None);
        }
    }
}

pub struct VulkanGraphicsPipeline {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub device: ash::Device,
}

impl Drop for VulkanGraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}

pub struct VulkanComputePipeline {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub device: ash::Device,
}

impl Drop for VulkanComputePipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}

pub struct VulkanCommandPool {
    pub pool: vk::CommandPool,
    pub device: ash::Device,
}

impl Drop for VulkanCommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.pool, None);
        }
    }
}

impl lume_core::device::CommandPool for VulkanCommandPool {
    type Device = crate::VulkanDevice;
    type CommandBuffer = VulkanCommandBuffer;

    fn allocate_command_buffer(&self) -> LumeResult<Self::CommandBuffer> {
        let allocate_info = vk::CommandBufferAllocateInfo {
            command_pool: self.pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: 1,
            ..Default::default()
        };

        let command_buffers = unsafe {
            self.device.allocate_command_buffers(&allocate_info)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to allocate command buffer: {}", e)))?
        };

        Ok(VulkanCommandBuffer {
            buffer: command_buffers[0],
            device: self.device.clone(),
            current_pipeline_layout: vk::PipelineLayout::null(),
        })
    }
}

pub struct VulkanCommandBuffer {
    pub buffer: vk::CommandBuffer,
    pub device: ash::Device,
    pub current_pipeline_layout: vk::PipelineLayout,
}

impl VulkanCommandBuffer {
    pub fn set_current_pipeline_layout(&mut self, layout: vk::PipelineLayout) {
        self.current_pipeline_layout = layout;
    }
}

impl lume_core::device::CommandBuffer for VulkanCommandBuffer {
    type Device = crate::VulkanDevice;

    fn reset(&mut self) -> LumeResult<()> {
        unsafe {
            self.device.reset_command_buffer(self.buffer, vk::CommandBufferResetFlags::empty())
                .map_err(|e| LumeError::BackendError(format!("Failed to reset command buffer: {}", e)))
        }
    }

    fn begin(&mut self) -> LumeResult<()> {
        let begin_info = vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };

        unsafe {
            self.device.begin_command_buffer(self.buffer, &begin_info)
                .map_err(|e| LumeError::BackendError(format!("Failed to begin command buffer: {}", e)))
        }
    }

    fn end(&mut self) -> LumeResult<()> {
        unsafe {
            self.device.end_command_buffer(self.buffer)
                .map_err(|e| LumeError::BackendError(format!("Failed to end command buffer: {}", e)))
        }
    }

    fn begin_render_pass(&mut self, render_pass: &crate::VulkanRenderPass, framebuffer: &crate::VulkanFramebuffer, clear_color: [f32; 4]) {
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: clear_color,
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];

        let render_pass_begin_info = vk::RenderPassBeginInfo {
            render_pass: render_pass.render_pass,
            framebuffer: framebuffer.framebuffer,
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D { 
                    width: framebuffer.width, 
                    height: framebuffer.height 
                },
            },
            clear_value_count: clear_values.len() as u32,
            p_clear_values: clear_values.as_ptr(),
            ..Default::default()
        };

        unsafe {
            self.device.cmd_begin_render_pass(self.buffer, &render_pass_begin_info, vk::SubpassContents::INLINE);
        }
    }

    fn draw(&mut self, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32) {
        unsafe {
            self.device.cmd_draw(self.buffer, vertex_count, instance_count, first_vertex, first_instance);
        }
    }

    fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        unsafe {
            self.device.cmd_dispatch(self.buffer, x, y, z);
        }
    }

    fn end_render_pass(&mut self) {
        unsafe {
            self.device.cmd_end_render_pass(self.buffer);
        }
    }

    fn bind_graphics_pipeline(&mut self, pipeline: &crate::VulkanGraphicsPipeline) {
        unsafe {
            self.device.cmd_bind_pipeline(self.buffer, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);
            self.current_pipeline_layout = pipeline.layout;
        }
    }

    fn bind_compute_pipeline(&mut self, pipeline: &crate::VulkanComputePipeline) {
        unsafe {
            self.device.cmd_bind_pipeline(self.buffer, vk::PipelineBindPoint::COMPUTE, pipeline.pipeline);
            self.current_pipeline_layout = pipeline.layout;
        }
    }

    fn bind_vertex_buffer(&mut self, buffer: &crate::VulkanBuffer) {
        unsafe {
            self.device.cmd_bind_vertex_buffers(self.buffer, 0, &[buffer.buffer], &[0]);
        }
    }

    fn bind_bind_group(&mut self, index: u32, bind_group: &crate::VulkanBindGroup) {
        // Try to detect if we are in graphics or compute.
        // For simplicity, we can bind to BOTH or use a flag. 
        // WebGPU-like and Vulkan allow binding to different bind points.
        // Given our abstraction, we might need a way to know the current bind point.
        // For now, let's bind to GRAPHICS if a render pass is active, else COMPUTE?
        // Actually, let's just bind to both if we don't know, or use the last bound pipeline's type.
        // Better: Bind to GRAPHICS for now, and implement separate if needed.
        // WAIT: cmd_bind_descriptor_sets REQUIRES a pipeline_bind_point.
        // Let's use a simple heuristic: if we have a current_pipeline_layout and we bound a compute pipeline last, use COMPUTE.
        // We'll need to track the bind point in VulkanCommandBuffer.
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                self.buffer,
                vk::PipelineBindPoint::GRAPHICS, // Default to GRAPHICS
                self.current_pipeline_layout,
                index,
                &[bind_group.set],
                &[],
            );
            
            // Also bind to COMPUTE just in case, if layout is compatible.
            // In Vulkan, layouts are often the same.
            self.device.cmd_bind_descriptor_sets(
                self.buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.current_pipeline_layout,
                index,
                &[bind_group.set],
                &[],
            );
        }
    }

    fn set_viewport(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let viewport = vk::Viewport {
            x, y, width, height,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        unsafe {
            self.device.cmd_set_viewport(self.buffer, 0, &[viewport]);
        }
    }

    fn set_scissor(&mut self, x: i32, y: i32, width: u32, height: u32) {
        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x, y },
            extent: vk::Extent2D { width, height },
        };
        unsafe {
            self.device.cmd_set_scissor(self.buffer, 0, &[scissor]);
        }
    }

    fn copy_buffer_to_buffer(&mut self, source: &crate::VulkanBuffer, destination: &crate::VulkanBuffer, size: u64) {
        let region = vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size,
        };
        unsafe {
            self.device.cmd_copy_buffer(self.buffer, source.buffer, destination.buffer, &[region]);
        }
    }

    fn copy_buffer_to_texture(&mut self, source: &crate::VulkanBuffer, destination: &crate::VulkanTexture, width: u32, height: u32) {
        let region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: vk::Extent3D { width, height, depth: 1 },
        };

        unsafe {
            self.device.cmd_copy_buffer_to_image(
                self.buffer,
                source.buffer,
                destination.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            );
        }
    }

    fn texture_barrier(&mut self, texture: &crate::VulkanTexture, old_layout: lume_core::device::ImageLayout, new_layout: lume_core::device::ImageLayout) {
        let barrier = vk::ImageMemoryBarrier {
            old_layout: map_layout(old_layout),
            new_layout: map_layout(new_layout),
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: texture.image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_access_mask: match old_layout {
                lume_core::device::ImageLayout::Undefined => vk::AccessFlags::empty(),
                lume_core::device::ImageLayout::TransferDst => vk::AccessFlags::TRANSFER_WRITE,
                _ => vk::AccessFlags::MEMORY_READ, // Simplified
            },
            dst_access_mask: match new_layout {
                lume_core::device::ImageLayout::TransferDst => vk::AccessFlags::TRANSFER_WRITE,
                lume_core::device::ImageLayout::ShaderReadOnly => vk::AccessFlags::SHADER_READ,
                _ => vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            },
            ..Default::default()
        };

        let src_stage = match old_layout {
            lume_core::device::ImageLayout::Undefined => vk::PipelineStageFlags::TOP_OF_PIPE,
            lume_core::device::ImageLayout::TransferDst => vk::PipelineStageFlags::TRANSFER,
            _ => vk::PipelineStageFlags::ALL_COMMANDS,
        };

        let dst_stage = match new_layout {
            lume_core::device::ImageLayout::TransferDst => vk::PipelineStageFlags::TRANSFER,
            lume_core::device::ImageLayout::ShaderReadOnly => vk::PipelineStageFlags::FRAGMENT_SHADER,
            _ => vk::PipelineStageFlags::ALL_COMMANDS,
        };

        unsafe {
            self.device.cmd_pipeline_barrier(
                self.buffer,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }
    }

    fn compute_barrier(&mut self) {
        let barrier = vk::MemoryBarrier {
            src_access_mask: vk::AccessFlags::SHADER_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ | vk::AccessFlags::UNIFORM_READ,
            ..Default::default()
        };

        unsafe {
            self.device.cmd_pipeline_barrier(
                self.buffer,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::COMPUTE_SHADER | vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[barrier],
                &[],
                &[],
            );
        }
    }
}

fn map_layout(layout: lume_core::device::ImageLayout) -> vk::ImageLayout {
    match layout {
        lume_core::device::ImageLayout::Undefined => vk::ImageLayout::UNDEFINED,
        lume_core::device::ImageLayout::General => vk::ImageLayout::GENERAL,
        lume_core::device::ImageLayout::TransferSrc => vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        lume_core::device::ImageLayout::TransferDst => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        lume_core::device::ImageLayout::ShaderReadOnly => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    }
}

#[derive(Clone)]
pub struct VulkanSemaphore {
    pub semaphore: vk::Semaphore,
    pub device: ash::Device,
}

impl Drop for VulkanSemaphore {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_semaphore(self.semaphore, None);
        }
    }
}

pub struct VulkanFramebuffer {
    pub framebuffer: vk::Framebuffer,
    pub width: u32,
    pub height: u32,
    pub device: ash::Device,
}

impl Drop for VulkanFramebuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_framebuffer(self.framebuffer, None);
        }
    }
}

impl lume_core::device::ShaderModule for VulkanShaderModule {}
impl lume_core::device::RenderPass for VulkanRenderPass {}
impl lume_core::device::PipelineLayout for VulkanPipelineLayout {}
impl lume_core::device::GraphicsPipeline for VulkanGraphicsPipeline {}
impl lume_core::device::ComputePipeline for VulkanComputePipeline {}
impl lume_core::device::Semaphore for VulkanSemaphore {}
impl lume_core::device::Framebuffer for VulkanFramebuffer {}

// BindGroup and BindGroupLayout moved to descriptor.rs


// Sampler moved to texture.rs
