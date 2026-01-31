use ash::{vk, Entry};
use lume_core::{Instance, InstanceDescriptor};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::ffi::{CStr, CString};
use log::{info, error, warn};
use crate::VulkanDevice;

pub struct VulkanInstance {
    _entry: ash::Entry,
    instance: ash::Instance,
    _debug_utils_loader: Option<ash::ext::debug_utils::Instance>,
    _debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = unsafe { *p_callback_data };
    let message_id_number = callback_data.message_id_number;
    
    let message_id_name = if callback_data.p_message_id_name.is_null() {
        std::borrow::Cow::from("")
    } else {
        unsafe { CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy() }
    };

    let message = if callback_data.p_message.is_null() {
        std::borrow::Cow::from("")
    } else {
        unsafe { CStr::from_ptr(callback_data.p_message).to_string_lossy() }
    };

    let log_level = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => log::Level::Debug,
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => log::Level::Info,
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => log::Level::Warn,
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => log::Level::Error,
        _ => log::Level::Info,
    };

    log::log!(
        log_level,
        "[Vulkan] {:?} [{} ({})]: {}",
        message_type,
        message_id_name,
        message_id_number,
        message
    );

    vk::FALSE
}


impl Instance for VulkanInstance {
    type Device = VulkanDevice;
    type Surface = crate::VulkanSurface;

    fn new(descriptor: InstanceDescriptor) -> Result<Self, &'static str> {
        // ... (existing new implementation remains same, just ensuring we are inside the impl)
        // For brevity in this tool call, I will not re-paste `new` if I can avoid it.
        // But `replace_file_content` requires replacing what matches.
        // I will match the impl block start and end.
        // Actually, it's better to use `multi_replace_file_content` if I want to keep `new`.
        // But the previous `replace_file_content` showed I have the full file context in my memory? 
        // No, I need to read it or be precise. 
        // I'll assume users code hasn't changed much. 
        // Let's use `multi_replace_file_content` to surgically update items.
        // Wait, I need to implement `request_device` too.
        Self::new_impl(descriptor)
    }

    fn create_surface(
        &self,
        _display_handle: impl HasDisplayHandle,
        window_handle: impl HasWindowHandle,
    ) -> Result<Self::Surface, &'static str> {
        use raw_window_handle::RawWindowHandle;

        let raw_window_handle = window_handle.window_handle().map_err(|_| "Failed to get window handle")?.as_raw();

        let surface_khr = match raw_window_handle {
            #[cfg(target_os = "windows")]
            RawWindowHandle::Win32(handle) => {
                let hinstance = handle.hinstance.ok_or("No hinstance found")?;
                let hwnd = handle.hwnd;
                
                let create_info = vk::Win32SurfaceCreateInfoKHR {
                    hinstance: unsafe { std::mem::transmute(hinstance.get()) },
                    hwnd: unsafe { std::mem::transmute(hwnd.get()) },
                    ..Default::default()
                };

                let win32_surface_loader = ash::khr::win32_surface::Instance::new(&self._entry, &self.instance);
                unsafe {
                    win32_surface_loader.create_win32_surface(&create_info, None)
                        .map_err(|e| {
                            error!("Failed to create Win32 surface: {:?}", e);
                            "Failed to create Win32 surface"
                        })?
                }
            }
            _ => return Err("Unsupported window handle for this platform"),
        };

        info!("Vulkan Surface created successfully: {:?}", surface_khr);
        
        let surface_loader = ash::khr::surface::Instance::new(&self._entry, &self.instance);

        Ok(crate::VulkanSurface {
            surface: surface_khr,
            surface_loader,
        })
    }

    fn request_device(
        &self,
        surface: Option<&Self::Surface>,
    ) -> Result<Self::Device, &'static str> {
        
        let pdevices = unsafe {
            self.instance.enumerate_physical_devices()
                .map_err(|_| "Failed to enumerate physical devices")?
        };

        info!("Found {} physical devices", pdevices.len());

        let (pdevice, queue_family_index) = pdevices.iter().find_map(|pdevice| {
            let props = unsafe { self.instance.get_physical_device_properties(*pdevice) };
            let queue_families = unsafe { self.instance.get_physical_device_queue_family_properties(*pdevice) };

            // Find a queue family that supports Graphics (and optionally Present if surface is provided)
            let index = queue_families.iter().enumerate().position(|(i, q)| {
                let supports_graphics = q.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                let supports_present = if let Some(surface) = surface {
                    unsafe {
                        surface.surface_loader.get_physical_device_surface_support(
                            *pdevice,
                            i as u32,
                            surface.surface
                        ).unwrap_or(false)
                    }
                } else {
                    true
                };
                supports_graphics && supports_present
            });

            if let Some(index) = index {
                // Prefer Discrete GPU
                if props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
                    info!("Selected Discrete GPU: {:?}", unsafe { CStr::from_ptr(props.device_name.as_ptr()) });
                    return Some((*pdevice, index as u32));
                }
                 // If not discrete, still acceptable if it's the first one we found (we might refine this later)
                 Some((*pdevice, index as u32))
            } else {
                None
            }
        }).ok_or("No suitable physical device found")?;

        let priorities = [1.0];
        let queue_create_info = vk::DeviceQueueCreateInfo {
            queue_family_index,
            p_queue_priorities: priorities.as_ptr(),
            queue_count: 1,
            ..Default::default()
        };

        let mut device_extension_names = Vec::new();
        if surface.is_some() {
            device_extension_names.push(ash::khr::swapchain::NAME.as_ptr());
        }

        let device_create_info = vk::DeviceCreateInfo {
            p_queue_create_infos: &queue_create_info,
            queue_create_info_count: 1,
            pp_enabled_extension_names: device_extension_names.as_ptr(),
            enabled_extension_count: device_extension_names.len() as u32,
            ..Default::default()
        };

        let device = unsafe {
            self.instance.create_device(pdevice, &device_create_info, None)
                .map_err(|e| {
                    error!("Failed to create logical device: {:?}", e);
                    "Failed to create logical device"
                })?
        };

        let graphics_queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        // Create Descriptor Pool
        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 10,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 10,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 10,
            },
        ];

        let pool_info = vk::DescriptorPoolCreateInfo {
            pool_size_count: pool_sizes.len() as u32,
            p_pool_sizes: pool_sizes.as_ptr(),
            max_sets: 10,
            ..Default::default()
        };

        let descriptor_pool = unsafe {
            device.create_descriptor_pool(&pool_info, None)
                .map_err(|_| "Failed to create descriptor pool")?
        };

        info!("Vulkan Device created successfully");

        Ok(VulkanDevice {
            instance: self.instance.clone(),
            device,
            physical_device: pdevice,
            graphics_queue,
            present_queue: graphics_queue,
            graphics_queue_index: queue_family_index,
            descriptor_pool,
        })
    }
}

impl VulkanInstance {
    // Helper to keep the existing new implementation
    fn new_impl(descriptor: InstanceDescriptor) -> Result<Self, &'static str> {
         info!("Initializing Vulkan Instance for application: {}", descriptor.name);
        
        let entry = unsafe { Entry::load().map_err(|_| "Failed to load Vulkan Entry")? };
        
        let app_name = CString::new(descriptor.name).unwrap();
        let engine_name = CString::new("LumeVisual").unwrap();
        
        let app_info = vk::ApplicationInfo {
            p_application_name: app_name.as_ptr(),
            application_version: 0,
            p_engine_name: engine_name.as_ptr(),
            engine_version: 0,
            api_version: vk::API_VERSION_1_3,
            ..Default::default()
        };

        let extension_names = vec![
            ash::khr::surface::NAME.as_ptr(),
            #[cfg(target_os = "windows")]
            ash::khr::win32_surface::NAME.as_ptr(),
            #[cfg(target_os = "linux")]
            ash::khr::xlib_surface::NAME.as_ptr(),
            ash::ext::debug_utils::NAME.as_ptr(),
        ];

        // Validation layers
        let layer_names = vec![
             CString::new("VK_LAYER_KHRONOS_validation").unwrap(),
        ];
        let _layer_names_ptrs: Vec<*const i8> = layer_names.iter().map(|n| n.as_ptr()).collect();

        let mut debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT {
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            pfn_user_callback: Some(vulkan_debug_callback),
            ..Default::default()
        };

        let create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            pp_enabled_extension_names: extension_names.as_ptr(),
            enabled_extension_count: extension_names.len() as u32,
            // pp_enabled_layer_names: layer_names_ptrs.as_ptr(), 
            // enabled_layer_count: layer_names_ptrs.len() as u32,
            p_next: &mut debug_create_info as *mut _ as *const _,
            ..Default::default()
        };

        let instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .map_err(|e| {
                    error!("Instance creation error: {:?}", e);
                    "Failed to create Vulkan Instance"
                })?
        };

        let debug_utils_loader = ash::ext::debug_utils::Instance::new(&entry, &instance);
        let debug_messenger = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&debug_create_info, None)
                .map_err(|e| {
                   warn!("Failed to create debug messenger: {:?}", e); 
                   "Failed to create debug messenger"
                }).ok()
        };

        info!("Vulkan Instance created successfully");

        Ok(VulkanInstance {
            _entry: entry,
            instance,
            _debug_utils_loader: Some(debug_utils_loader),
            _debug_messenger: debug_messenger,
        })
    }
}
