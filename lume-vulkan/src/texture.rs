use ash::vk;
use gpu_allocator::vulkan::*;
use std::sync::{Arc, Mutex};

pub struct VulkanTexture {
    pub image: vk::Image,
    pub allocation: Allocation,
    pub format: vk::Format,
    pub width: u32,
    pub height: u32,
    pub allocator: Arc<Mutex<Allocator>>,
    pub device: ash::Device,
}

impl Drop for VulkanTexture {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image(self.image, None);
        }
        let allocation = std::mem::replace(&mut self.allocation, Allocation::default());
        self.allocator.lock().unwrap().free(allocation).expect("Failed to free image memory");
    }
}

impl lume_core::device::Texture for VulkanTexture {}

pub struct VulkanTextureView {
    pub view: vk::ImageView,
    pub device: ash::Device,
}

impl Drop for VulkanTextureView {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.view, None);
        }
    }
}

impl lume_core::device::TextureView for VulkanTextureView {}

pub struct VulkanSampler {
    pub sampler: vk::Sampler,
    pub device: ash::Device,
}

impl Drop for VulkanSampler {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_sampler(self.sampler, None);
        }
    }
}

impl lume_core::device::Sampler for VulkanSampler {}
