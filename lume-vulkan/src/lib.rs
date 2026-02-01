mod device;
mod surface;
mod instance;
mod swapchain;
mod pipeline;
mod buffer;
mod texture;

pub use instance::VulkanInstance;
pub use surface::VulkanSurface;
pub use device::VulkanDevice;
pub use swapchain::VulkanSwapchain;
pub use texture::{VulkanTexture, VulkanTextureView, VulkanSampler};
pub use pipeline::*;
pub use buffer::VulkanBuffer;
// BindGroup/Layout are re-exported through device or pipeline
pub use device::{VulkanBindGroup, VulkanBindGroupLayout};

pub struct VulkanSemaphore {
    pub semaphore: ash::vk::Semaphore,
    pub device: ash::Device,
}

impl lume_core::device::Semaphore for VulkanSemaphore {}

impl Drop for VulkanSemaphore {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_semaphore(self.semaphore, None);
        }
    }
}

pub struct VulkanFence {
    pub fence: ash::vk::Fence,
    pub device: ash::Device,
}

impl lume_core::device::Fence for VulkanFence {}

impl Drop for VulkanFence {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_fence(self.fence, None);
        }
    }
}

