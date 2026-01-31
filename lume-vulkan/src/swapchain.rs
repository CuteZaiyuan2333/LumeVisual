use ash::vk;
use log::{info};
use crate::VulkanTextureView;

pub struct VulkanSwapchain {
    pub swapchain_loader: ash::khr::swapchain::Device,
    pub swapchain: vk::SwapchainKHR,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<VulkanTextureView>,
    pub extent: vk::Extent2D,
    pub format: vk::Format,
    
    // Sync primitives for acquisition
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub current_frame: usize,
    
    pub device: ash::Device,
    pub present_queue: vk::Queue,
}

// View moved to texture.rs

impl Drop for VulkanSwapchain {
    fn drop(&mut self) {
        unsafe {
            info!("Destroying Swapchain");
            self.image_views.clear(); // This will trigger drop on each VulkanTextureView
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            for &sem in &self.image_available_semaphores {
                self.device.destroy_semaphore(sem, None);
            }
        }
    }
}

impl lume_core::device::Swapchain for VulkanSwapchain {
    type TextureView = VulkanTextureView;

    fn acquire_next_image(&mut self) -> lume_core::LumeResult<u32> {
        let semaphore = self.image_available_semaphores[self.current_frame];

        unsafe {
            let (index, _is_suboptimal) = self.swapchain_loader
                .acquire_next_image(
                    self.swapchain,
                    u64::MAX,
                    semaphore,
                    vk::Fence::null(),
                )
                .map_err(|e| lume_core::LumeError::BackendError(format!("Failed to acquire next image: {}", e)))?;
            Ok(index)
        }
    }

    fn present(&mut self, image_index: u32) -> lume_core::LumeResult<()> {
        let swapchains = [self.swapchain];
        let image_indices = [image_index];
        
        let present_info = vk::PresentInfoKHR {
            swapchain_count: 1,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: image_indices.as_ptr(),
            ..Default::default()
        };

        unsafe {
            self.swapchain_loader
                .queue_present(self.present_queue, &present_info)
                .map_err(|e| lume_core::LumeError::BackendError(format!("Queue present failed: {}", e)))?;
        }
        Ok(())
    }

    fn get_view(&self, index: u32) -> &Self::TextureView {
        &self.image_views[index as usize]
    }
}
