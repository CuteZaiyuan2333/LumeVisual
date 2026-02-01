use ash::vk;
use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;
use lume_core::{LumeError, LumeResult};
use crate::VulkanDevice;
use std::sync::{Arc, Mutex};

impl VulkanDevice {
    pub fn create_buffer_impl(&self, descriptor: lume_core::device::BufferDescriptor) -> LumeResult<crate::VulkanBuffer> {
        let mut usage = vk::BufferUsageFlags::empty();
        let u = descriptor.usage;
        if u.0 & lume_core::device::BufferUsage::VERTEX.0 != 0 { usage |= vk::BufferUsageFlags::VERTEX_BUFFER; }
        if u.0 & lume_core::device::BufferUsage::INDEX.0 != 0 { usage |= vk::BufferUsageFlags::INDEX_BUFFER; }
        if u.0 & lume_core::device::BufferUsage::UNIFORM.0 != 0 { usage |= vk::BufferUsageFlags::UNIFORM_BUFFER; }
        if u.0 & lume_core::device::BufferUsage::STORAGE.0 != 0 { usage |= vk::BufferUsageFlags::STORAGE_BUFFER; }
        if u.0 & lume_core::device::BufferUsage::COPY_SRC.0 != 0 { usage |= vk::BufferUsageFlags::TRANSFER_SRC; }
        if u.0 & lume_core::device::BufferUsage::COPY_DST.0 != 0 { usage |= vk::BufferUsageFlags::TRANSFER_DST; }

        let create_info = vk::BufferCreateInfo {
            size: descriptor.size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe {
            self.inner.device.create_buffer(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create buffer: {}", e)))?
        };

        let requirements = unsafe { self.inner.device.get_buffer_memory_requirements(buffer) };
        let location = if descriptor.mapped_at_creation { MemoryLocation::CpuToGpu } else { MemoryLocation::GpuOnly };

        let allocator = self.inner.allocator.as_ref().ok_or_else(|| LumeError::BackendError("Allocator not initialized".to_string()))?;
        let allocation = allocator.lock().unwrap().allocate(&AllocationCreateDesc {
            name: "Lume_Buffer",
            requirements,
            location,
            linear: true,
            allocation_scheme: AllocationScheme::DedicatedBuffer(buffer),
        }).map_err(|e| LumeError::BackendError(format!("Failed to allocate buffer memory: {}", e)))?;

        unsafe {
            self.inner.device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .map_err(|e| LumeError::BackendError(format!("Failed to bind buffer memory: {}", e)))?;
        }

        Ok(crate::VulkanBuffer {
            buffer,
            allocation,
            size: descriptor.size,
            allocator: allocator.clone(),
            device: self.clone(),
        })
    }

    pub fn create_texture_impl(&self, descriptor: lume_core::device::TextureDescriptor) -> LumeResult<crate::VulkanTexture> {
        let mut usage = vk::ImageUsageFlags::empty();
        let tu = descriptor.usage;
        if tu.0 & lume_core::device::TextureUsage::TEXTURE_BINDING.0 != 0 { usage |= vk::ImageUsageFlags::SAMPLED; }
        if tu.0 & lume_core::device::TextureUsage::STORAGE_BINDING.0 != 0 { usage |= vk::ImageUsageFlags::STORAGE; }
        if tu.0 & lume_core::device::TextureUsage::RENDER_ATTACHMENT.0 != 0 { usage |= vk::ImageUsageFlags::COLOR_ATTACHMENT; }
        if tu.0 & lume_core::device::TextureUsage::DEPTH_STENCIL_ATTACHMENT.0 != 0 { usage |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT; }
        if tu.0 & lume_core::device::TextureUsage::COPY_SRC.0 != 0 { usage |= vk::ImageUsageFlags::TRANSFER_SRC; }
        if tu.0 & lume_core::device::TextureUsage::COPY_DST.0 != 0 { usage |= vk::ImageUsageFlags::TRANSFER_DST; }

        let format = map_texture_format(descriptor.format);
        let create_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            format,
            extent: vk::Extent3D { width: descriptor.width, height: descriptor.height, depth: 1 },
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            ..Default::default()
        };

        let image = unsafe {
            self.inner.device.create_image(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create texture: {}", e)))?
        };

        let requirements = unsafe { self.inner.device.get_image_memory_requirements(image) };
        let allocator = self.inner.allocator.as_ref().ok_or_else(|| LumeError::BackendError("Allocator not initialized".to_string()))?;
        let allocation = allocator.lock().unwrap().allocate(&AllocationCreateDesc {
            name: "Lume_Texture",
            requirements,
            location: MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        }).map_err(|e| LumeError::BackendError(format!("Failed to allocate texture memory: {}", e)))?;

        unsafe {
            self.inner.device.bind_image_memory(image, allocation.memory(), allocation.offset())
                .map_err(|e| LumeError::BackendError(format!("Failed to bind texture memory: {}", e)))?;
        }

        Ok(crate::VulkanTexture {
            image,
            allocation,
            format,
            width: descriptor.width,
            height: descriptor.height,
            allocator: allocator.clone(),
            device: self.inner.device.clone(),
            current_layout: Arc::new(Mutex::new(vk::ImageLayout::UNDEFINED)),
        })
    }

    pub fn create_sampler_impl(&self, descriptor: lume_core::device::SamplerDescriptor) -> LumeResult<crate::VulkanSampler> {
        let create_info = vk::SamplerCreateInfo {
            mag_filter: match descriptor.mag_filter {
                lume_core::device::FilterMode::Nearest => vk::Filter::NEAREST,
                lume_core::device::FilterMode::Linear => vk::Filter::LINEAR,
            },
            min_filter: match descriptor.min_filter {
                lume_core::device::FilterMode::Nearest => vk::Filter::NEAREST,
                lume_core::device::FilterMode::Linear => vk::Filter::LINEAR,
            },
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            ..Default::default()
        };

        let sampler = unsafe {
            self.inner.device.create_sampler(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create sampler: {}", e)))?
        };

        Ok(crate::VulkanSampler {
            sampler,
            device: self.inner.device.clone(),
        })
    }

    pub fn create_texture_view_impl(&self, texture: &crate::VulkanTexture, descriptor: lume_core::device::TextureViewDescriptor) -> LumeResult<crate::VulkanTextureView> {
        let format = descriptor.format.map(map_texture_format).unwrap_or(texture.format);
        let aspect_mask = if is_depth_format(format) { vk::ImageAspectFlags::DEPTH } else { vk::ImageAspectFlags::COLOR };

        let create_info = vk::ImageViewCreateInfo {
            image: texture.image,
            view_type: vk::ImageViewType::TYPE_2D,
            format,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };

        let view = unsafe {
            self.inner.device.create_image_view(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create texture view: {}", e)))?
        };

        Ok(crate::VulkanTextureView {
            view,
            image: texture.image,
            extent: vk::Extent2D { width: texture.width, height: texture.height },
            current_layout: texture.current_layout.clone(),
            device: self.inner.device.clone(),
        })
    }

    pub fn create_swapchain_impl(
        &self,
        surface: &impl lume_core::instance::Surface,
        descriptor: lume_core::device::SwapchainDescriptor,
    ) -> LumeResult<crate::VulkanSwapchain> {
        let vk_surface = unsafe {
            &*(surface as *const dyn lume_core::instance::Surface as *const crate::VulkanSurface)
        };
        
        let surface_loader = &vk_surface.surface_loader;
        let surface_khr = vk_surface.surface;
        
        let formats = unsafe { surface_loader.get_physical_device_surface_formats(self.inner.physical_device, surface_khr).unwrap() };
        let format = formats[0];
        
        let caps = unsafe { surface_loader.get_physical_device_surface_capabilities(self.inner.physical_device, surface_khr).unwrap() };
        let extent = vk::Extent2D { width: descriptor.width, height: descriptor.height };

        let create_info = vk::SwapchainCreateInfoKHR {
            surface: surface_khr,
            min_image_count: 3,
            image_format: format.format,
            image_color_space: format.color_space,
            image_extent: extent,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            pre_transform: caps.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode: vk::PresentModeKHR::FIFO,
            ..Default::default()
        };

        let loader = ash::khr::swapchain::Device::new(&self.inner.instance, &self.inner.device);
        let swapchain = unsafe { loader.create_swapchain(&create_info, None).unwrap() };
        let images = unsafe { loader.get_swapchain_images(swapchain).unwrap() };
        
        let mut image_views = Vec::new();
        for &image in &images {
            let iv_info = vk::ImageViewCreateInfo {
                image,
                view_type: vk::ImageViewType::TYPE_2D,
                format: format.format,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };
            let view = unsafe { self.inner.device.create_image_view(&iv_info, None).unwrap() };
            image_views.push(crate::VulkanTextureView { 
                view, 
                image,
                extent,
                current_layout: Arc::new(Mutex::new(vk::ImageLayout::UNDEFINED)),
                device: self.inner.device.clone() 
            });
        }

        Ok(crate::VulkanSwapchain {
            swapchain_loader: loader,
            swapchain,
            images,
            image_views,
            extent,
            format: format.format,
            image_available_semaphores: Vec::new(),
            current_frame: 0,
            device: self.inner.device.clone(),
            present_queue: self.inner.present_queue,
        })
    }
}

pub fn map_texture_format(format: lume_core::device::TextureFormat) -> vk::Format {
    match format {
        lume_core::device::TextureFormat::Bgra8UnormSrgb => vk::Format::B8G8R8A8_SRGB,
        lume_core::device::TextureFormat::Rgba8UnormSrgb => vk::Format::R8G8B8A8_SRGB,
        lume_core::device::TextureFormat::Rgba8Unorm => vk::Format::R8G8B8A8_UNORM,
        lume_core::device::TextureFormat::Depth32Float => vk::Format::D32_SFLOAT,
    }
}

pub fn is_depth_format(format: vk::Format) -> bool {
    matches!(
        format,
        vk::Format::D16_UNORM | vk::Format::D32_SFLOAT | vk::Format::S8_UINT | vk::Format::D24_UNORM_S8_UINT
    )
}