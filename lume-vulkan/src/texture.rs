use ash::vk;

pub struct VulkanTexture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub device: ash::Device,
}

impl Drop for VulkanTexture {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self.memory, None);
        }
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
