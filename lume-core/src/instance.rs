use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Vulkan,
    Metal,
}

pub struct InstanceDescriptor<'a> {
    pub name: &'a str,
    pub backend: Backend,
}

pub trait Instance: Sized {
    type Device: crate::Device;
    type Surface: Surface;

    /// Create a new instance of the rendering backend.
    fn new(descriptor: InstanceDescriptor) -> crate::LumeResult<Self>;

    /// Create a surface from a window.
    fn create_surface(
        &self,
        display_handle: impl HasDisplayHandle,
        window_handle: impl HasWindowHandle,
    ) -> crate::LumeResult<Self::Surface>;

    /// Request a suitable graphics device.
    /// This typically involves picking a physical device that supports the created surface.
    fn request_device(
        &self,
        surface: Option<&Self::Surface>,
    ) -> crate::LumeResult<Self::Device>;
}

pub trait Surface {}
