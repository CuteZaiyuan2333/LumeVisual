use ash::vk;

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

impl lume_core::device::CommandPool<crate::VulkanDevice> for VulkanCommandPool {
    fn allocate_command_buffer(&self) -> Result<VulkanCommandBuffer, &'static str> {
        let allocate_info = vk::CommandBufferAllocateInfo {
            command_pool: self.pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: 1,
            ..Default::default()
        };

        let command_buffers = unsafe {
            self.device.allocate_command_buffers(&allocate_info)
                .map_err(|_| "Failed to allocate command buffer")?
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

impl lume_core::device::CommandBuffer<crate::VulkanDevice> for VulkanCommandBuffer {
    fn reset(&mut self) -> Result<(), &'static str> {
        unsafe {
            self.device.reset_command_buffer(self.buffer, vk::CommandBufferResetFlags::empty())
                .map_err(|_| "Failed to reset command buffer")
        }
    }

    fn begin(&mut self) -> Result<(), &'static str> {
        let begin_info = vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };

        unsafe {
            self.device.begin_command_buffer(self.buffer, &begin_info)
                .map_err(|_| "Failed to begin command buffer")
        }
    }

    fn end(&mut self) -> Result<(), &'static str> {
        unsafe {
            self.device.end_command_buffer(self.buffer)
                .map_err(|_| "Failed to end command buffer")
        }
    }

    fn begin_render_pass(&mut self, render_pass: &crate::VulkanRenderPass, framebuffer: &crate::VulkanFramebuffer, clear_color: [f32; 4]) {
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: clear_color,
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

    fn bind_vertex_buffer(&mut self, buffer: &crate::VulkanBuffer) {
        unsafe {
            self.device.cmd_bind_vertex_buffers(self.buffer, 0, &[buffer.buffer], &[0]);
        }
    }

    fn bind_bind_group(&mut self, index: u32, bind_group: &crate::VulkanBindGroup) {
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                self.buffer,
                vk::PipelineBindPoint::GRAPHICS,
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
impl lume_core::device::Semaphore for VulkanSemaphore {}
impl lume_core::device::Framebuffer for VulkanFramebuffer {}

pub struct VulkanBindGroupLayout {
    pub layout: vk::DescriptorSetLayout,
    pub device: ash::Device,
}

impl Drop for VulkanBindGroupLayout {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}

pub struct VulkanBindGroup {
    pub set: vk::DescriptorSet,
}

impl lume_core::device::BindGroupLayout for VulkanBindGroupLayout {}
impl lume_core::device::BindGroup for VulkanBindGroup {}

// Sampler moved to texture.rs
