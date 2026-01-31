use ash::vk;
use std::sync::{Arc, Mutex};
use gpu_allocator::vulkan::Allocator;

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
            vk::DescriptorPoolSize { ty: vk::DescriptorType::UNIFORM_BUFFER, descriptor_count: 1000 },
            vk::DescriptorPoolSize { ty: vk::DescriptorType::STORAGE_BUFFER, descriptor_count: 1000 },
            vk::DescriptorPoolSize { ty: vk::DescriptorType::SAMPLED_IMAGE, descriptor_count: 1000 },
            vk::DescriptorPoolSize { ty: vk::DescriptorType::SAMPLER, descriptor_count: 1000 },
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
            log::info!("Destroying Vulkan Device and Descriptor Pool");
            self.allocator.take();
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_device(None);
        }
    }
}

// Submodules for implementation
mod resource;
mod pipeline;
mod descriptor;
mod queue;

pub use descriptor::{VulkanBindGroup, VulkanBindGroupLayout};
