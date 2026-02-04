use ash::vk;
use lume_core::device::ShaderStage;
use lume_core::{LumeError, LumeResult};

use std::sync::Arc;

fn map_shader_stage(stage: lume_core::device::ShaderStage) -> vk::ShaderStageFlags {
    let mut flags = vk::ShaderStageFlags::empty();
    if stage.0 & lume_core::device::ShaderStage::VERTEX.0 != 0 { flags |= vk::ShaderStageFlags::VERTEX; }
    if stage.0 & lume_core::device::ShaderStage::FRAGMENT.0 != 0 { flags |= vk::ShaderStageFlags::FRAGMENT; }
    if stage.0 & lume_core::device::ShaderStage::COMPUTE.0 != 0 { flags |= vk::ShaderStageFlags::COMPUTE; }
    flags
}

pub struct VulkanShaderModuleInner {
    pub module: vk::ShaderModule,
    pub device: ash::Device,
}

impl Drop for VulkanShaderModuleInner {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_shader_module(self.module, None);
        }
    }
}

#[derive(Clone)]
pub struct VulkanShaderModule(pub Arc<VulkanShaderModuleInner>);

pub struct VulkanRenderPassInner {
    pub render_pass: vk::RenderPass,
    pub device: ash::Device,
}

impl Drop for VulkanRenderPassInner {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_render_pass(self.render_pass, None);
        }
    }
}

#[derive(Clone)]
pub struct VulkanRenderPass(pub Arc<VulkanRenderPassInner>);

pub struct VulkanPipelineLayoutInner {
    pub layout: vk::PipelineLayout,
    pub set_layouts: Vec<vk::DescriptorSetLayout>,
    pub device: ash::Device,
}

impl Drop for VulkanPipelineLayoutInner {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline_layout(self.layout, None);
        }
    }
}

#[derive(Clone)]
pub struct VulkanPipelineLayout(pub Arc<VulkanPipelineLayoutInner>);

pub struct VulkanGraphicsPipelineInner {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub device: ash::Device,
}

impl Drop for VulkanGraphicsPipelineInner {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}

#[derive(Clone)]
pub struct VulkanGraphicsPipeline(pub Arc<VulkanGraphicsPipelineInner>);

pub struct VulkanComputePipelineInner {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub device: ash::Device,
}

impl Drop for VulkanComputePipelineInner {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}

#[derive(Clone)]
pub struct VulkanComputePipeline(pub Arc<VulkanComputePipelineInner>);

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
            current_bind_point: vk::PipelineBindPoint::GRAPHICS,
        })
    }
}

pub struct VulkanCommandBuffer {
    pub buffer: vk::CommandBuffer,
    pub device: ash::Device,
    pub current_pipeline_layout: vk::PipelineLayout,
    pub current_bind_point: vk::PipelineBindPoint,
}

impl VulkanCommandBuffer {
    fn internal_barrier(&self, view: &crate::VulkanTextureView, target_layout: vk::ImageLayout) {
        let mut current_layout = view.current_layout.lock().unwrap();
        if *current_layout == target_layout {
            return;
        }

        let image_barrier = vk::ImageMemoryBarrier2 {
            src_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
            src_access_mask: vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ,
            dst_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
            dst_access_mask: vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ,
            old_layout: *current_layout,
            new_layout: target_layout,
            image: view.image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };

        let mut barrier_cloned = image_barrier;
        if target_layout == vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL {
            barrier_cloned.subresource_range.aspect_mask = vk::ImageAspectFlags::DEPTH;
        }

        let dependency_info = vk::DependencyInfo {
            image_memory_barrier_count: 1,
            p_image_memory_barriers: &barrier_cloned,
            ..Default::default()
        };

        unsafe {
            self.device.cmd_pipeline_barrier2(self.buffer, &dependency_info);
        }

        *current_layout = target_layout;
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
            render_pass: render_pass.0.render_pass,
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

    fn draw_indirect(&mut self, buffer: &crate::VulkanBuffer, offset: u64, draw_count: u32, stride: u32) {
        unsafe {
            self.device.cmd_draw_indirect(self.buffer, buffer.buffer, offset, draw_count, stride);
        }
    }

    fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        unsafe {
            self.device.cmd_dispatch(self.buffer, x, y, z);
        }
    }

    fn dispatch_indirect(&mut self, buffer: &crate::VulkanBuffer, offset: u64) {
        unsafe {
            self.device.cmd_dispatch_indirect(self.buffer, buffer.buffer, offset);
        }
    }

    fn end_render_pass(&mut self) {
        unsafe {
            self.device.cmd_end_render_pass(self.buffer);
        }
    }

    fn begin_rendering(&mut self, descriptor: lume_core::device::RenderingDescriptor<Self::Device>) {
        for at in descriptor.color_attachments {
            let target_layout = match at.layout {
                lume_core::device::ImageLayout::General => vk::ImageLayout::GENERAL,
                lume_core::device::ImageLayout::ShaderReadOnly => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                _ => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            };
            self.internal_barrier(at.view, target_layout);
        }

        if let Some(ref at) = descriptor.depth_attachment {
            self.internal_barrier(at.view, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
        }

        let color_attachments: Vec<vk::RenderingAttachmentInfo> = descriptor.color_attachments.iter().map(|at| {
            let clear_value = match at.clear_value {
                lume_core::device::ClearValue::Color(c) => vk::ClearValue {
                    color: vk::ClearColorValue { float32: c },
                },
                _ => vk::ClearValue::default(),
            };

            vk::RenderingAttachmentInfo {
                image_view: at.view.view,
                image_layout: match at.layout {
                    lume_core::device::ImageLayout::General => vk::ImageLayout::GENERAL,
                    lume_core::device::ImageLayout::ShaderReadOnly => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    _ => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                },
                load_op: match at.load_op {
                    lume_core::device::AttachmentLoadOp::Load => vk::AttachmentLoadOp::LOAD,
                    lume_core::device::AttachmentLoadOp::Clear => vk::AttachmentLoadOp::CLEAR,
                    lume_core::device::AttachmentLoadOp::DontCare => vk::AttachmentLoadOp::DONT_CARE,
                },
                store_op: match at.store_op {
                    lume_core::device::AttachmentStoreOp::Store => vk::AttachmentStoreOp::STORE,
                    lume_core::device::AttachmentStoreOp::DontCare => vk::AttachmentStoreOp::DONT_CARE,
                },
                clear_value,
                ..Default::default()
            }
        }).collect();

        let depth_attachment = descriptor.depth_attachment.as_ref().map(|at| {
            let clear_value = match at.clear_value {
                lume_core::device::ClearValue::DepthStencil(d, s) => vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue { depth: d, stencil: s },
                },
                _ => vk::ClearValue::default(),
            };

            vk::RenderingAttachmentInfo {
                image_view: at.view.view,
                image_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                load_op: match at.load_op {
                    lume_core::device::AttachmentLoadOp::Load => vk::AttachmentLoadOp::LOAD,
                    lume_core::device::AttachmentLoadOp::Clear => vk::AttachmentLoadOp::CLEAR,
                    lume_core::device::AttachmentLoadOp::DontCare => vk::AttachmentLoadOp::DONT_CARE,
                },
                store_op: match at.store_op {
                    lume_core::device::AttachmentStoreOp::Store => vk::AttachmentStoreOp::STORE,
                    lume_core::device::AttachmentStoreOp::DontCare => vk::AttachmentStoreOp::DONT_CARE,
                },
                clear_value,
                ..Default::default()
            }
        });

        let render_area = if let Some(at) = descriptor.color_attachments.first() {
            vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: at.view.extent,
            }
        } else if let Some(at) = &descriptor.depth_attachment {
            vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: at.view.extent,
            }
        } else {
            vk::Rect2D::default()
        };

        let rendering_info = vk::RenderingInfo {
            render_area,
            layer_count: 1,
            color_attachment_count: color_attachments.len() as u32,
            p_color_attachments: color_attachments.as_ptr(),
            p_depth_attachment: depth_attachment.as_ref().map(|at| at as *const _).unwrap_or(std::ptr::null()),
            ..Default::default()
        };

        unsafe {
            self.device.cmd_begin_rendering(self.buffer, &rendering_info);
        }
    }

    fn end_rendering(&mut self) {
        unsafe {
            self.device.cmd_end_rendering(self.buffer);
        }
    }

    fn bind_graphics_pipeline(&mut self, pipeline: &<Self::Device as lume_core::Device>::GraphicsPipeline) {
        unsafe {
            self.device.cmd_bind_pipeline(self.buffer, vk::PipelineBindPoint::GRAPHICS, pipeline.0.pipeline);
            self.current_pipeline_layout = pipeline.0.layout;
            self.current_bind_point = vk::PipelineBindPoint::GRAPHICS;
        }
    }

    fn bind_compute_pipeline(&mut self, pipeline: &crate::VulkanComputePipeline) {
        unsafe {
            self.device.cmd_bind_pipeline(self.buffer, vk::PipelineBindPoint::COMPUTE, pipeline.0.pipeline);
            self.current_pipeline_layout = pipeline.0.layout;
            self.current_bind_point = vk::PipelineBindPoint::COMPUTE;
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
                self.current_bind_point,
                self.current_pipeline_layout,
                index,
                &[bind_group.set],
                &[],
            );
        }
    }

    fn set_push_constants(&mut self, _layout: &crate::VulkanPipelineLayout, stages: ShaderStage, offset: u32, data: &[u8]) {
        unsafe {
            self.device.cmd_push_constants(
                self.buffer,
                self.current_pipeline_layout,
                map_shader_stage(stages),
                offset,
                data,
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

    fn copy_buffer_to_buffer_offset(&mut self, source: &crate::VulkanBuffer, src_offset: u64, destination: &crate::VulkanBuffer, dst_offset: u64, size: u64) {
        let region = vk::BufferCopy {
            src_offset,
            dst_offset,
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

    fn texture_barrier(&mut self, texture_view: &crate::VulkanTextureView, old_layout: lume_core::device::ImageLayout, new_layout: lume_core::device::ImageLayout) {
        let barrier = vk::ImageMemoryBarrier {
            old_layout: map_layout(old_layout),
            new_layout: map_layout(new_layout),
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: texture_view.image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: if new_layout == lume_core::device::ImageLayout::DepthStencilAttachment {
                    vk::ImageAspectFlags::DEPTH
                } else {
                    vk::ImageAspectFlags::COLOR
                },
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_access_mask: match old_layout {
                lume_core::device::ImageLayout::Undefined => vk::AccessFlags::empty(),
                lume_core::device::ImageLayout::TransferDst => vk::AccessFlags::TRANSFER_WRITE,
                lume_core::device::ImageLayout::ColorAttachment => vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                lume_core::device::ImageLayout::DepthStencilAttachment => vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                _ => vk::AccessFlags::MEMORY_READ, 
            },
            dst_access_mask: match new_layout {
                lume_core::device::ImageLayout::TransferDst => vk::AccessFlags::TRANSFER_WRITE,
                lume_core::device::ImageLayout::ShaderReadOnly => vk::AccessFlags::SHADER_READ,
                lume_core::device::ImageLayout::ColorAttachment => vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                lume_core::device::ImageLayout::DepthStencilAttachment => vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                lume_core::device::ImageLayout::Present => vk::AccessFlags::empty(),
                _ => vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            },
            ..Default::default()
        };

        let src_stage = match old_layout {
            lume_core::device::ImageLayout::Undefined => vk::PipelineStageFlags::TOP_OF_PIPE,
            lume_core::device::ImageLayout::TransferDst => vk::PipelineStageFlags::TRANSFER,
            lume_core::device::ImageLayout::ColorAttachment => vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            lume_core::device::ImageLayout::DepthStencilAttachment => vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            _ => vk::PipelineStageFlags::ALL_COMMANDS,
        };

        let dst_stage = match new_layout {
            lume_core::device::ImageLayout::TransferDst => vk::PipelineStageFlags::TRANSFER,
            lume_core::device::ImageLayout::ShaderReadOnly => vk::PipelineStageFlags::FRAGMENT_SHADER,
            lume_core::device::ImageLayout::ColorAttachment => vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            lume_core::device::ImageLayout::DepthStencilAttachment => vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            lume_core::device::ImageLayout::Present => vk::PipelineStageFlags::BOTTOM_OF_PIPE,
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
        // 升级为全局强力屏障
        let memory_barrier = vk::MemoryBarrier2 {
            src_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
            src_access_mask: vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ,
            dst_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
            dst_access_mask: vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ,
            ..Default::default()
        };

        let dependency_info = vk::DependencyInfo {
            memory_barrier_count: 1,
            p_memory_barriers: &memory_barrier,
            ..Default::default()
        };

        unsafe {
            self.device.cmd_pipeline_barrier2(self.buffer, &dependency_info);
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
        lume_core::device::ImageLayout::ColorAttachment => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        lume_core::device::ImageLayout::DepthStencilAttachment => vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        lume_core::device::ImageLayout::Present => vk::ImageLayout::PRESENT_SRC_KHR,
    }
}

#[derive(Clone)]
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
impl lume_core::device::Framebuffer for VulkanFramebuffer {}