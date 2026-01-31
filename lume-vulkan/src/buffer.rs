use ash::vk;
use gpu_allocator::vulkan::*;
use lume_core::{LumeError, LumeResult};
use std::sync::{Arc, Mutex};

pub struct VulkanBuffer {
    pub buffer: vk::Buffer,
    pub allocation: Allocation,
    pub size: u64,
    pub allocator: Arc<Mutex<Allocator>>,
    pub device: ash::Device,
}

impl Drop for VulkanBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
        }
        let allocation = std::mem::replace(&mut self.allocation, Allocation::default());
        self.allocator.lock().unwrap().free(allocation).expect("Failed to free buffer memory");
    }
}

impl lume_core::device::Buffer for VulkanBuffer {
    fn write_data(&self, offset: u64, data: &[u8]) -> LumeResult<()> {
        let ptr = self.allocation.mapped_ptr()
            .ok_or_else(|| LumeError::BackendError("Buffer is not CPU-mappable or not mapped".to_string()))?
            .as_ptr();
        
        unsafe {
            let dst = (ptr as *mut u8).add(offset as usize);
            std::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
        }
        Ok(())
    }

    fn read_data(&self, offset: u64, data: &mut [u8]) -> LumeResult<()> {
        let ptr = self.allocation.mapped_ptr()
            .ok_or_else(|| LumeError::BackendError("Buffer is not CPU-mappable or not mapped".to_string()))?
            .as_ptr();
        
        unsafe {
            let src = (ptr as *const u8).add(offset as usize);
            std::ptr::copy_nonoverlapping(src, data.as_mut_ptr(), data.len());
        }
        Ok(())
    }
}
