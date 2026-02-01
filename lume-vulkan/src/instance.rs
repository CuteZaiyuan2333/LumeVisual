use ash::{vk};
use lume_core::{Instance, InstanceDescriptor};
use std::ffi::{CStr};
use log::{info, warn};
use crate::VulkanDevice;
use gpu_allocator::vulkan::*;
use gpu_allocator::AllocationSizes;
use std::sync::{Arc, Mutex};

pub struct VulkanInstance {
    _debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
    _debug_utils_loader: Option<ash::ext::debug_utils::Instance>,
    instance: ash::Instance,
    _entry: ash::Entry,
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

    fn new(descriptor: InstanceDescriptor) -> lume_core::LumeResult<Self> {
        info!("Initializing Vulkan Instance for application: {}", descriptor.name);
        let entry = unsafe { ash::Entry::load().map_err(|e| lume_core::LumeError::BackendError(format!("Failed to load Vulkan entry: {}", e)))? };

        let app_name_cstring = std::ffi::CString::new(descriptor.name).unwrap();
        let app_name = unsafe { CStr::from_bytes_with_nul_unchecked(app_name_cstring.as_bytes_with_nul()) };
        let engine_name = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"LumeEngine\0") };
        
        let app_info = vk::ApplicationInfo {
            p_application_name: app_name.as_ptr(),
            application_version: 0,
            p_engine_name: engine_name.as_ptr(),
            engine_version: 0,
            api_version: vk::API_VERSION_1_3,
            ..Default::default()
        };

        let layer_names: [*const i8; 0] = [];
        let extension_names = [
            ash::ext::debug_utils::NAME.as_ptr(),
            ash::khr::surface::NAME.as_ptr(),
            #[cfg(target_os = "windows")]
            ash::khr::win32_surface::NAME.as_ptr(),
            #[cfg(target_os = "linux")]
            ash::khr::xlib_surface::NAME.as_ptr(),
            #[cfg(target_os = "macos")]
            ash::khr::portability_enumeration::NAME.as_ptr(),
        ];

        let create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            enabled_extension_count: extension_names.len() as u32,
            pp_enabled_extension_names: extension_names.as_ptr(),
            enabled_layer_count: layer_names.len() as u32,
            pp_enabled_layer_names: layer_names.as_ptr(),
            ..Default::default()
        };

        let instance = unsafe {
            entry.create_instance(&create_info, None)
                .map_err(|e| lume_core::LumeError::InstanceCreationFailed(format!("Failed to create Vulkan instance: {}", e)))?
        };

        let debug_utils = ash::ext::debug_utils::Instance::new(&entry, &instance);
        let debug_messenger = setup_debug_utils(&debug_utils)?;

        info!("Vulkan Instance created successfully");

        Ok(VulkanInstance {
            _entry: entry,
            instance,
            _debug_utils_loader: Some(debug_utils),
            _debug_messenger: debug_messenger,
        })
    }

    fn create_surface(
        &self,
        display_handle: impl raw_window_handle::HasDisplayHandle,
        window_handle: impl raw_window_handle::HasWindowHandle,
    ) -> lume_core::LumeResult<Self::Surface> {
        let surface = unsafe {
            let display = display_handle.display_handle().map_err(|e| lume_core::LumeError::SurfaceCreationFailed(e.to_string()))?;
            let window = window_handle.window_handle().map_err(|e| lume_core::LumeError::SurfaceCreationFailed(e.to_string()))?;
            
            ash_window::create_surface(
                &self._entry, 
                &self.instance, 
                display.as_raw(),
                window.as_raw(),
                None
            )
            .map_err(|e| lume_core::LumeError::SurfaceCreationFailed(e.to_string()))?
        };

        info!("Vulkan Surface created successfully: {:?}", surface);
        
        let surface_loader = ash::khr::surface::Instance::new(&self._entry, &self.instance);
        Ok(crate::VulkanSurface { 
            surface,
            surface_loader,
        })
    }

    fn request_device(
        &self,
        surface: Option<&Self::Surface>,
    ) -> lume_core::LumeResult<Self::Device> {
        let surface_loader = ash::khr::surface::Instance::new(&self._entry, &self.instance);
        let pdevices = unsafe {
            self.instance.enumerate_physical_devices()
                .map_err(|e| lume_core::LumeError::BackendError(format!("Failed to enumerate GPUs: {}", e)))?
        };

        info!("Found {} physical devices", pdevices.len());

        let (pdevice, queue_family_index) = pdevices.iter()
            .find_map(|&pdev| {
                let props = unsafe { self.instance.get_physical_device_properties(pdev) };
            let selected_device_name = unsafe { std::ffi::CStr::from_ptr(props.device_name.as_ptr()) }.to_string_lossy();
                info!("Candidate GPU: {}", selected_device_name);
                if props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
                    let families = unsafe { self.instance.get_physical_device_queue_family_properties(pdev) };
                    for (idx, family) in families.iter().enumerate() {
                        if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                            if let Some(surf) = surface {
                                let supported = unsafe {
                                    surface_loader.get_physical_device_surface_support(pdev, idx as u32, surf.surface).unwrap_or(false)
                                };
                                if supported { return Some((pdev, idx as u32)); }
                            } else {
                                return Some((pdev, idx as u32));
                            }
                        }
                    }
                }
                None
            })
            .or_else(|| {
                pdevices.iter().find_map(|&pdev| {
                    let families = unsafe { self.instance.get_physical_device_queue_family_properties(pdev) };
                    for (idx, family) in families.iter().enumerate() {
                        if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                            if let Some(surf) = surface {
                                let supported = unsafe {
                                    surface_loader.get_physical_device_surface_support(pdev, idx as u32, surf.surface).unwrap_or(false)
                                };
                                if supported { return Some((pdev, idx as u32)); }
                            } else {
                                return Some((pdev, idx as u32));
                            }
                        }
                    }
                    None
                })
            })
            .ok_or_else(|| lume_core::LumeError::DeviceCreationFailed("No suitable GPU found".to_string()))?;

        let props = unsafe { self.instance.get_physical_device_properties(pdevice) };
        let selected_device_name = unsafe { std::ffi::CStr::from_ptr(props.device_name.as_ptr()) }.to_string_lossy();
        info!("Selected GPU: {}", selected_device_name);

        let priorities = [1.0];
        let queue_info = vk::DeviceQueueCreateInfo {
            queue_family_index,
            queue_count: 1,
            p_queue_priorities: priorities.as_ptr(),
            ..Default::default()
        };

        let available_extensions = unsafe {
            self.instance.enumerate_device_extension_properties(pdevice)
                .map_err(|e| lume_core::LumeError::BackendError(format!("Failed to enumerate device extensions: {}", e)))?
        };

        let has_mesh_shader = available_extensions.iter().any(|ext| {
            let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
            name == ash::ext::mesh_shader::NAME
        });

        let mut device_extension_names = vec![
            ash::khr::swapchain::NAME.as_ptr(),
            #[cfg(target_os = "macos")]
            ash::vk::KhrPortabilitySubsetFn::name().as_ptr(),
        ];

        if has_mesh_shader {
            info!("Mesh Shader extension supported and enabled.");
            device_extension_names.push(ash::ext::mesh_shader::NAME.as_ptr());
        } else {
            warn!("Mesh Shader extension NOT supported by this GPU.");
        }

        let features_mesh = vk::PhysicalDeviceMeshShaderFeaturesEXT {
            mesh_shader: vk::TRUE,
            task_shader: vk::TRUE,
            ..Default::default()
        };

        let mut features13 = vk::PhysicalDeviceVulkan13Features {
            dynamic_rendering: vk::TRUE,
            synchronization2: vk::TRUE,
            ..Default::default()
        };

        let mut features12 = vk::PhysicalDeviceVulkan12Features {
            descriptor_indexing: vk::TRUE,
            buffer_device_address: vk::TRUE,
            runtime_descriptor_array: vk::TRUE,
            descriptor_binding_variable_descriptor_count: vk::TRUE,
            descriptor_binding_partially_bound: vk::TRUE,
            ..Default::default()
        };

        let features = vk::PhysicalDeviceFeatures::default();
        let create_info = vk::DeviceCreateInfo {
            p_next: &features12 as *const _ as *const std::ffi::c_void,
            p_queue_create_infos: &queue_info,
            queue_create_info_count: 1,
            pp_enabled_extension_names: device_extension_names.as_ptr(),
            enabled_extension_count: device_extension_names.len() as u32,
            p_enabled_features: &features,
            ..Default::default()
        };

        // Chain features: features12 -> features13
        features12.p_next = &features13 as *const _ as *mut std::ffi::c_void;
        
        if has_mesh_shader {
            // Chain features13 -> features_mesh
            features13.p_next = &features_mesh as *const _ as *mut std::ffi::c_void;
        }

        let device = unsafe {
            self.instance.create_device(pdevice, &create_info, None)
                .map_err(|e| lume_core::LumeError::DeviceCreationFailed(e.to_string()))?
        };

        let graphics_queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: self.instance.clone(),
            device: device.clone(),
            physical_device: pdevice,
            debug_settings: Default::default(),
            buffer_device_address: false, 
            allocation_sizes: AllocationSizes::default(),
        }).map_err(|e| lume_core::LumeError::BackendError(format!("Failed to create GPU allocator: {}", e)))?;

        info!("Vulkan Device and Allocator created successfully");

        Ok(VulkanDevice::new(
            self.instance.clone(),
            device,
            graphics_queue,
            graphics_queue,
            queue_family_index,
            Some(Arc::new(Mutex::new(allocator))),
            pdevice,
        ))
    }
}

fn setup_debug_utils(debug_utils: &ash::ext::debug_utils::Instance) -> lume_core::LumeResult<Option<vk::DebugUtilsMessengerEXT>> {
    let debug_info = vk::DebugUtilsMessengerCreateInfoEXT {
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
            | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
            | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        pfn_user_callback: Some(vulkan_debug_callback),
        ..Default::default()
    };

    unsafe {
        debug_utils
            .create_debug_utils_messenger(&debug_info, None)
            .map(Some)
            .map_err(|e| lume_core::LumeError::BackendError(format!("Failed to create debug messenger: {}", e)))
    }
}
