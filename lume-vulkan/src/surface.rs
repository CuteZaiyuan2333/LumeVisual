use ash::vk;

pub struct VulkanSurface {
    pub surface: vk::SurfaceKHR,
    pub surface_loader: ash::khr::surface::Instance,
}

impl Drop for VulkanSurface {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}

impl lume_core::instance::Surface for VulkanSurface {}
