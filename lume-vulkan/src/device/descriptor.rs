use ash::vk;
use std::collections::HashMap;
use lume_core::{LumeError, LumeResult, device::{BindingType, BindGroupDescriptor, BindGroupLayoutDescriptor}};
use crate::VulkanDevice;

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

impl VulkanDevice {
    pub fn create_bind_group_layout_impl(&self, descriptor: BindGroupLayoutDescriptor) -> LumeResult<VulkanBindGroupLayout> {
        let mut entries = Vec::new();
        let mut type_map = HashMap::new();

        for entry in descriptor.entries {
            let vk_type = match entry.ty {
                BindingType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
                BindingType::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
                BindingType::SampledTexture => vk::DescriptorType::SAMPLED_IMAGE,
                BindingType::Sampler => vk::DescriptorType::SAMPLER,
            };

            let mut stage_flags = vk::ShaderStageFlags::empty();
            if entry.visibility.0 & lume_core::device::ShaderStage::VERTEX.0 != 0 { stage_flags |= vk::ShaderStageFlags::VERTEX; }
            if entry.visibility.0 & lume_core::device::ShaderStage::FRAGMENT.0 != 0 { stage_flags |= vk::ShaderStageFlags::FRAGMENT; }
            if entry.visibility.0 & lume_core::device::ShaderStage::COMPUTE.0 != 0 { stage_flags |= vk::ShaderStageFlags::COMPUTE; }

            entries.push(vk::DescriptorSetLayoutBinding {
                binding: entry.binding,
                descriptor_type: vk_type,
                descriptor_count: 1,
                stage_flags,
                ..Default::default()
            });
            type_map.insert(entry.binding, entry.ty);
        }

        let create_info = vk::DescriptorSetLayoutCreateInfo {
            binding_count: entries.len() as u32,
            p_bindings: entries.as_ptr(),
            ..Default::default()
        };

        let layout = unsafe {
            self.inner.device.create_descriptor_set_layout(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create bind group layout: {}", e)))?
        };

        Ok(VulkanBindGroupLayout {
            layout,
            entries: type_map,
            device: self.inner.device.clone(),
        })
    }

    pub fn create_bind_group_impl(&self, descriptor: BindGroupDescriptor<Self>) -> LumeResult<VulkanBindGroup> {
        let allocate_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool: self.inner.descriptor_pool,
            descriptor_set_count: 1,
            p_set_layouts: &descriptor.layout.layout,
            ..Default::default()
        };

        let sets = unsafe {
            self.inner.device.allocate_descriptor_sets(&allocate_info)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to allocate bind group: {}", e)))?
        };

        let set = sets[0];
        
        let mut final_buffer_infos = Vec::new();
        let mut final_image_infos = Vec::new();
        
        for entry in &descriptor.entries {
            match entry.resource {
                lume_core::device::BindingResource::Buffer(buf) => {
                    final_buffer_infos.push(vk::DescriptorBufferInfo {
                        buffer: buf.buffer,
                        offset: 0,
                        range: buf.size,
                    });
                }
                lume_core::device::BindingResource::TextureView(view) => {
                    final_image_infos.push(vk::DescriptorImageInfo {
                        image_view: view.view,
                        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        ..Default::default()
                    });
                }
                lume_core::device::BindingResource::Sampler(sampler) => {
                    final_image_infos.push(vk::DescriptorImageInfo {
                        sampler: sampler.sampler,
                        ..Default::default()
                    });
                }
            }
        }
        
        let mut buffer_pointer = 0;
        let mut image_pointer = 0;
        let mut writes = Vec::new();
        
        for entry in &descriptor.entries {
            let ty = descriptor.layout.entries.get(&entry.binding).ok_or_else(|| LumeError::Generic("Unknown binding in bind group"))?;
            match entry.resource {
                lume_core::device::BindingResource::Buffer(_) => {
                    let vk_ty = match ty {
                        BindingType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
                        BindingType::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
                        _ => return Err(LumeError::Generic("Mismatched binding type for buffer")),
                    };
                    writes.push(vk::WriteDescriptorSet {
                        dst_set: set,
                        dst_binding: entry.binding,
                        descriptor_count: 1,
                        descriptor_type: vk_ty,
                        p_buffer_info: &final_buffer_infos[buffer_pointer],
                        ..Default::default()
                    });
                    buffer_pointer += 1;
                }
                lume_core::device::BindingResource::TextureView(_) => {
                    writes.push(vk::WriteDescriptorSet {
                        dst_set: set,
                        dst_binding: entry.binding,
                        descriptor_count: 1,
                        descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
                        p_image_info: &final_image_infos[image_pointer],
                        ..Default::default()
                    });
                    image_pointer += 1;
                }
                lume_core::device::BindingResource::Sampler(_) => {
                    writes.push(vk::WriteDescriptorSet {
                        dst_set: set,
                        dst_binding: entry.binding,
                        descriptor_count: 1,
                        descriptor_type: vk::DescriptorType::SAMPLER,
                        p_image_info: &final_image_infos[image_pointer],
                        ..Default::default()
                    });
                    image_pointer += 1;
                }
            }
        }

        unsafe {
            self.inner.device.update_descriptor_sets(&writes, &[]);
        }

        Ok(VulkanBindGroup { set })
    }
}
