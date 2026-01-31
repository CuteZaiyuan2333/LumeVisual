use ash::vk;
use std::collections::HashMap;
use lume_core::{device::BindingType};

pub struct VulkanBindGroupLayout {
    pub layout: vk::DescriptorSetLayout,
    pub entries: HashMap<u32, BindingType>,
    pub device: ash::Device,
}

impl Drop for VulkanBindGroupLayout {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}

impl lume_core::device::BindGroupLayout for VulkanBindGroupLayout {}

pub struct VulkanBindGroup {
    pub set: vk::DescriptorSet,
}

impl lume_core::device::BindGroup for VulkanBindGroup {}
