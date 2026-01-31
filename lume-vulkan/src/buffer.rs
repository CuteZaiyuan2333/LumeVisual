use ash::vk;

pub struct VulkanBuffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: u64,
    pub device: ash::Device,
}

impl Drop for VulkanBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

impl lume_core::device::Buffer for VulkanBuffer {
    fn write_data(&self, offset: u64, data: &[u8]) -> Result<(), &'static str> {
        unsafe {
            let ptr = self.device.map_memory(
                self.memory,
                offset,
                data.len() as u64,
                vk::MemoryMapFlags::empty(),
            ).map_err(|_| "Failed to map buffer memory")?;

            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());

            self.device.unmap_memory(self.memory);
        }
        Ok(())
    }

    fn read_data(&self, offset: u64, data: &mut [u8]) -> Result<(), &'static str> {
        unsafe {
            let ptr = self.device.map_memory(
                self.memory,
                offset,
                data.len() as u64,
                vk::MemoryMapFlags::empty(),
            ).map_err(|_| "Failed to map buffer memory for reading")?;

            std::ptr::copy_nonoverlapping(ptr as *const u8, data.as_mut_ptr(), data.len());

            self.device.unmap_memory(self.memory);
        }
        Ok(())
    }
}
