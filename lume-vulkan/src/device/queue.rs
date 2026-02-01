use ash::vk;
use lume_core::{LumeError, LumeResult};
use crate::VulkanDevice;

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
    type Fence = crate::VulkanFence;

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

    fn create_fence(&self, signaled: bool) -> LumeResult<Self::Fence> {
        let flags = if signaled { vk::FenceCreateFlags::SIGNALED } else { vk::FenceCreateFlags::empty() };
        let create_info = vk::FenceCreateInfo {
            flags,
            ..Default::default()
        };
        let fence = unsafe {
            self.inner.device.create_fence(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create fence: {}", e)))?
        };
        Ok(crate::VulkanFence {
            fence,
            device: self.inner.device.clone(),
        })
    }

    fn wait_for_fences(&self, fences: &[&Self::Fence], wait_all: bool, timeout: u64) -> LumeResult<()> {
        let vk_fences: Vec<vk::Fence> = fences.iter().map(|f| f.fence).collect();
        unsafe {
            self.inner.device.wait_for_fences(&vk_fences, wait_all, timeout)
                .map_err(|e| LumeError::BackendError(format!("Wait for fences failed: {}", e)))
        }
    }

    fn reset_fences(&self, fences: &[&Self::Fence]) -> LumeResult<()> {
        let vk_fences: Vec<vk::Fence> = fences.iter().map(|f| f.fence).collect();
        unsafe {
            self.inner.device.reset_fences(&vk_fences)
                .map_err(|e| LumeError::BackendError(format!("Reset fences failed: {}", e)))
        }
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
        fence: Option<&Self::Fence>,
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

        let vk_fence = fence.map(|f| f.fence).unwrap_or(vk::Fence::null());

        unsafe {
            self.inner.device.queue_submit(self.inner.graphics_queue, &[submit_info], vk_fence)
                .map_err(|e| LumeError::SubmissionFailed(format!("Failed to submit command buffers: {}", e)))
        }
    }

    fn create_buffer(&self, descriptor: lume_core::device::BufferDescriptor) -> LumeResult<Self::Buffer> {
        self.create_buffer_impl(descriptor)
    }

    fn create_texture(&self, descriptor: lume_core::device::TextureDescriptor) -> LumeResult<Self::Texture> {
        self.create_texture_impl(descriptor)
    }

    fn create_texture_view(&self, texture: &Self::Texture, descriptor: lume_core::device::TextureViewDescriptor) -> LumeResult<Self::TextureView> {
        self.create_texture_view_impl(texture, descriptor)
    }

    fn create_sampler(&self, descriptor: lume_core::device::SamplerDescriptor) -> LumeResult<Self::Sampler> {
        self.create_sampler_impl(descriptor)
    }

    fn create_shader_module(&self, code: &[u32]) -> LumeResult<Self::ShaderModule> {
        self.create_shader_module_impl(code)
    }

    fn create_render_pass(&self, descriptor: lume_core::device::RenderPassDescriptor) -> LumeResult<Self::RenderPass> {
        self.create_render_pass_impl(descriptor)
    }

    fn create_pipeline_layout(&self, descriptor: lume_core::device::PipelineLayoutDescriptor<Self>) -> LumeResult<Self::PipelineLayout> {
        self.create_pipeline_layout_impl(descriptor)
    }

    fn create_graphics_pipeline(&self, descriptor: lume_core::device::GraphicsPipelineDescriptor<Self>) -> LumeResult<Self::GraphicsPipeline> {
        self.create_graphics_pipeline_impl(descriptor)
    }

    fn create_compute_pipeline(&self, descriptor: lume_core::device::ComputePipelineDescriptor<Self>) -> LumeResult<Self::ComputePipeline> {
        self.create_compute_pipeline_impl(descriptor)
    }

    fn create_framebuffer(&self, descriptor: lume_core::device::FramebufferDescriptor<Self>) -> LumeResult<Self::Framebuffer> {
        self.create_framebuffer_impl(descriptor)
    }

    fn create_bind_group_layout(&self, descriptor: lume_core::device::BindGroupLayoutDescriptor) -> LumeResult<Self::BindGroupLayout> {
        self.create_bind_group_layout_impl(descriptor)
    }

    fn create_bind_group(&self, descriptor: lume_core::device::BindGroupDescriptor<Self>) -> LumeResult<Self::BindGroup> {
        self.create_bind_group_impl(descriptor)
    }

    fn create_swapchain(&self, surface: &impl lume_core::instance::Surface, descriptor: lume_core::device::SwapchainDescriptor) -> LumeResult<Self::Swapchain> {
        self.create_swapchain_impl(surface, descriptor)
    }
}