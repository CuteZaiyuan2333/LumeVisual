use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};
use lume_core::{Instance, InstanceDescriptor, Backend, Device};
use lume_vulkan::VulkanInstance;
use std::sync::Arc;

struct App {
    window: Option<Arc<Window>>,
    instance: Option<VulkanInstance>,
    surface: Option<lume_vulkan::VulkanSurface>,
    device: Option<lume_vulkan::VulkanDevice>,
    swapchain: Option<lume_vulkan::VulkanSwapchain>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("LumeVisual - Hello Window (Winit 0.30)")
                .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
            
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());

            let instance_desc = InstanceDescriptor {
                name: "Hello Window",
                backend: Backend::Vulkan,
            };
            
            let instance = VulkanInstance::new(instance_desc).expect("Failed to create Lume Instance");
             
            let surface = instance.create_surface(&window, &window).expect("Failed to create surface");
            
            // Request device
            let device = instance.request_device(Some(&surface)).expect("Failed to request device");

            // Create Swapchain
            let size = window.inner_size();
            let swapchain_desc = lume_core::device::SwapchainDescriptor {
                width: size.width,
                height: size.height,
            };
            let swapchain = device.create_swapchain(&surface, swapchain_desc).expect("Failed to create swapchain");

            self.instance = Some(instance);
            self.surface = Some(surface);
            self.device = Some(device);
            self.swapchain = Some(swapchain);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => (),
        }
    }
}

fn main() {
    env_logger::init();
    log::info!("Starting Hello Window Example");

    let event_loop = EventLoop::new().unwrap();
    let mut app = App { window: None, instance: None, surface: None, device: None, swapchain: None };
    event_loop.run_app(&mut app).unwrap();
}
