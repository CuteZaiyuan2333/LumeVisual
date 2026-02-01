use ash::vk;
use std::sync::{Arc, Mutex};
use gpu_allocator::vulkan::Allocator;

pub struct VulkanDeviceInner {
    pub allocator: Option<Arc<Mutex<Allocator>>>,
    pub descriptor_pool: vk::DescriptorPool,
    pub bindless_descriptor_set: vk::DescriptorSet,
    pub bindless_layout: vk::DescriptorSetLayout,
    pub graphics_queue_index: u32,
    pub present_queue: vk::Queue, 
    pub graphics_queue: vk::Queue,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub instance: ash::Instance,
    
    // Frame-in-flight management
    pub frame_sync: Mutex<VulkanFrameSyncManager>,
}

pub struct VulkanFrameSyncManager {
    pub current_frame: usize,
    pub frames_in_flight: usize,
    pub fences: Vec<vk::Fence>,
    pub image_available: Vec<vk::Semaphore>,
    pub render_finished: Vec<vk::Semaphore>,
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
        // Create Bindless Layout
        let bindless_binding = vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
            descriptor_count: 100000, // Large array for bindless
            stage_flags: vk::ShaderStageFlags::ALL,
            ..Default::default()
        };

        let binding_flags = [vk::DescriptorBindingFlags::PARTIALLY_BOUND | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND];
        let mut extended_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo {
            binding_count: 1,
            p_binding_flags: binding_flags.as_ptr(),
            ..Default::default()
        };

        let layout_info = vk::DescriptorSetLayoutCreateInfo {
            p_next: &extended_info as *const _ as *const std::ffi::c_void,
            flags: vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
            binding_count: 1,
            p_bindings: &bindless_binding,
            ..Default::default()
        };

        let bindless_layout = unsafe { device.create_descriptor_set_layout(&layout_info, None).unwrap() };

        let pool_sizes = [
            vk::DescriptorPoolSize { ty: vk::DescriptorType::UNIFORM_BUFFER, descriptor_count: 1000 },
            vk::DescriptorPoolSize { ty: vk::DescriptorType::STORAGE_BUFFER, descriptor_count: 1000 },
            vk::DescriptorPoolSize { ty: vk::DescriptorType::SAMPLED_IMAGE, descriptor_count: 100000 },
            vk::DescriptorPoolSize { ty: vk::DescriptorType::SAMPLER, descriptor_count: 1000 },
        ];

        let pool_info = vk::DescriptorPoolCreateInfo {
            flags: vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND,
            pool_size_count: pool_sizes.len() as u32,
            p_pool_sizes: pool_sizes.as_ptr(),
            max_sets: 2000,
            ..Default::default()
        };

        let descriptor_pool = unsafe {
            device.create_descriptor_pool(&pool_info, None).expect("Failed to create descriptor pool")
        };

        let alloc_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool,
            descriptor_set_count: 1,
            p_set_layouts: &bindless_layout,
            ..Default::default()
        };

        let bindless_descriptor_set = unsafe { device.allocate_descriptor_sets(&alloc_info).unwrap()[0] };

        let frames_in_flight = 2;
        let mut fences = Vec::new();
        let mut image_available = Vec::new();
        let mut render_finished = Vec::new();

        for _ in 0..frames_in_flight {
            let fence_info = vk::FenceCreateInfo {
                flags: vk::FenceCreateFlags::SIGNALED,
                ..Default::default()
            };
            let sem_info = vk::SemaphoreCreateInfo::default();
            unsafe {
                fences.push(device.create_fence(&fence_info, None).unwrap());
                image_available.push(device.create_semaphore(&sem_info, None).unwrap());
                render_finished.push(device.create_semaphore(&sem_info, None).unwrap());
            }
        }

        Self {
            inner: Arc::new(VulkanDeviceInner {
                instance,
                device,
                physical_device,
                graphics_queue,
                present_queue,
                graphics_queue_index,
                descriptor_pool,
                bindless_descriptor_set,
                bindless_layout,
                allocator,
                frame_sync: Mutex::new(VulkanFrameSyncManager {
                    current_frame: 0,
                    frames_in_flight,
                    fences,
                    image_available,
                    render_finished,
                }),
            }),
        }
    }
}

impl Drop for VulkanDeviceInner {
    fn drop(&mut self) {
        unsafe {
            log::info!("Destroying Vulkan Device and Synchronization Objects");
            self.device.destroy_descriptor_set_layout(self.bindless_layout, None);
            let sync = self.frame_sync.lock().unwrap();
            for i in 0..sync.frames_in_flight {
                self.device.destroy_fence(sync.fences[i], None);
                self.device.destroy_semaphore(sync.image_available[i], None);
                self.device.destroy_semaphore(sync.render_finished[i], None);
            }
            self.allocator.take();
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_device(None);
        }
    }
}

// Submodules for implementation
pub(crate) mod resource;
mod pipeline;
mod descriptor;
mod queue;

pub use descriptor::{VulkanBindGroup, VulkanBindGroupLayout};
