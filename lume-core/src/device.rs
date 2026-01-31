pub trait Device: Sized + Clone {
    type Buffer: Buffer;
    type Texture: Texture;
    type TextureView: TextureView;
    type Sampler: Sampler;
    type ShaderModule: ShaderModule;
    type RenderPass: RenderPass;
    type PipelineLayout: PipelineLayout;
    type GraphicsPipeline: GraphicsPipeline;
    type ComputePipeline: ComputePipeline;
    type CommandPool: CommandPool<CommandBuffer = Self::CommandBuffer>;
    type CommandBuffer: CommandBuffer;
    type Framebuffer: Framebuffer;
    type Swapchain: Swapchain<TextureView = Self::TextureView>;
    type BindGroupLayout: BindGroupLayout;
    type BindGroup: BindGroup;
    type Semaphore: Semaphore;

    /// Wait for the device to be idle.
    fn wait_idle(&self) -> crate::LumeResult<()>;

    fn create_command_pool(&self) -> crate::LumeResult<Self::CommandPool>;
    fn create_semaphore(&self) -> crate::LumeResult<Self::Semaphore>;

    fn create_swapchain(
        &self,
        surface: &impl crate::instance::Surface,
        descriptor: SwapchainDescriptor,
    ) -> crate::LumeResult<Self::Swapchain>;

    fn create_shader_module(&self, code: &[u32]) -> crate::LumeResult<Self::ShaderModule>;
    fn create_render_pass(&self, descriptor: RenderPassDescriptor) -> crate::LumeResult<Self::RenderPass>;
    fn create_pipeline_layout(&self, descriptor: PipelineLayoutDescriptor<Self>) -> crate::LumeResult<Self::PipelineLayout>;
    fn create_graphics_pipeline(&self, descriptor: GraphicsPipelineDescriptor<Self>) -> crate::LumeResult<Self::GraphicsPipeline>;
    fn create_compute_pipeline(&self, descriptor: ComputePipelineDescriptor<Self>) -> crate::LumeResult<Self::ComputePipeline>;
    fn create_framebuffer(&self, descriptor: FramebufferDescriptor<Self>) -> crate::LumeResult<Self::Framebuffer>;
    fn create_buffer(&self, descriptor: BufferDescriptor) -> crate::LumeResult<Self::Buffer>;
    fn create_texture(&self, descriptor: TextureDescriptor) -> crate::LumeResult<Self::Texture>;
    fn create_texture_view(&self, texture: &Self::Texture, descriptor: TextureViewDescriptor) -> crate::LumeResult<Self::TextureView>;
    fn create_sampler(&self, descriptor: SamplerDescriptor) -> crate::LumeResult<Self::Sampler>;
    fn create_bind_group_layout(&self, descriptor: BindGroupLayoutDescriptor) -> crate::LumeResult<Self::BindGroupLayout>;
    fn create_bind_group(&self, descriptor: BindGroupDescriptor<Self>) -> crate::LumeResult<Self::BindGroup>;

    /// Submit command buffers to the graphics queue.
    fn submit(
        &self,
        command_buffers: &[&Self::CommandBuffer],
        wait_semaphores: &[&Self::Semaphore],
        signal_semaphores: &[&Self::Semaphore],
    ) -> crate::LumeResult<()>;
}

pub trait CommandPool {
    type Device: Device;
    type CommandBuffer: CommandBuffer<Device = Self::Device>;
    fn allocate_command_buffer(&self) -> crate::LumeResult<Self::CommandBuffer>;
}

pub trait CommandBuffer {
    type Device: Device;
    fn reset(&mut self) -> crate::LumeResult<()>;
    fn begin(&mut self) -> crate::LumeResult<()>;
    fn end(&mut self) -> crate::LumeResult<()>;

    fn begin_render_pass(&mut self, render_pass: &<Self::Device as Device>::RenderPass, framebuffer: &<Self::Device as Device>::Framebuffer, clear_color: [f32; 4]);
    fn end_render_pass(&mut self);

    fn bind_graphics_pipeline(&mut self, pipeline: &<Self::Device as Device>::GraphicsPipeline);
    fn bind_compute_pipeline(&mut self, pipeline: &<Self::Device as Device>::ComputePipeline);
    fn bind_vertex_buffer(&mut self, buffer: &<Self::Device as Device>::Buffer);
    fn bind_bind_group(&mut self, index: u32, bind_group: &<Self::Device as Device>::BindGroup);
    fn set_viewport(&mut self, x: f32, y: f32, width: f32, height: f32);
    fn set_scissor(&mut self, x: i32, y: i32, width: u32, height: u32);
    fn draw(&mut self, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32);
    fn dispatch(&mut self, x: u32, y: u32, z: u32);
    fn copy_buffer_to_buffer(&mut self, source: &<Self::Device as Device>::Buffer, destination: &<Self::Device as Device>::Buffer, size: u64);
    fn copy_buffer_to_texture(&mut self, buffer: &<Self::Device as Device>::Buffer, texture: &<Self::Device as Device>::Texture, width: u32, height: u32);
    fn texture_barrier(&mut self, texture: &<Self::Device as Device>::Texture, old_layout: ImageLayout, new_layout: ImageLayout);
    fn compute_barrier(&mut self);
}

pub trait ShaderModule {}
pub trait RenderPass {}
pub trait PipelineLayout {}
pub trait GraphicsPipeline: Send + Sync {}
pub trait ComputePipeline: Send + Sync {}
pub trait Semaphore: Send + Sync {}
pub trait Framebuffer {}
pub trait TextureView {}
pub trait Texture {
    // For now we might return a raw handle OR have the device create the view from the texture
}
pub trait Sampler {}
pub trait Buffer {
    fn write_data(&self, offset: u64, data: &[u8]) -> crate::LumeResult<()>;
    fn read_data(&self, offset: u64, data: &mut [u8]) -> crate::LumeResult<()>;
}
pub trait BindGroupLayout {}
pub trait BindGroup {}

pub struct FramebufferDescriptor<'a, D: Device> {
    pub render_pass: &'a D::RenderPass,
    pub attachments: &'a [&'a D::TextureView],
    pub width: u32,
    pub height: u32,
}

/// Container for sync objects used during a frame.
pub struct FrameSync<D: Device> {
    pub image_available: D::Semaphore,
    pub render_finished: D::Semaphore,
}

pub struct RenderPassDescriptor {
    pub color_format: TextureFormat,
    pub depth_stencil_format: Option<TextureFormat>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextureFormat {
    Bgra8UnormSrgb,
    Rgba8UnormSrgb,
    Rgba8Unorm,
    Depth32Float,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImageLayout {
    Undefined,
    General,
    TransferSrc,
    TransferDst,
    ShaderReadOnly,
}

pub struct TextureDescriptor {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub format: TextureFormat,
    pub usage: TextureUsage,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TextureUsage(pub u32);

impl TextureUsage {
    pub const TEXTURE_BINDING: Self = Self(1 << 0);
    pub const STORAGE_BINDING: Self = Self(1 << 1);
    pub const RENDER_ATTACHMENT: Self = Self(1 << 2);
    pub const DEPTH_STENCIL_ATTACHMENT: Self = Self(1 << 3);
    pub const COPY_SRC: Self = Self(1 << 4);
    pub const COPY_DST: Self = Self(1 << 5);
}

impl std::ops::BitOr for TextureUsage {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

pub struct SamplerDescriptor {
    pub min_filter: FilterMode,
    pub mag_filter: FilterMode,
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FilterMode {
    Nearest,
    Linear,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AddressMode {
    Repeat,
    MirrorRepeat,
    ClampToEdge,
}

pub struct TextureViewDescriptor {
    pub format: Option<TextureFormat>,
}

pub struct PipelineLayoutDescriptor<'a, D: Device> {
    pub bind_group_layouts: &'a [&'a D::BindGroupLayout],
}

pub struct GraphicsPipelineDescriptor<'a, D: Device> {
    pub vertex_shader: &'a D::ShaderModule,
    pub fragment_shader: &'a D::ShaderModule,
    pub render_pass: &'a D::RenderPass,
    pub layout: &'a D::PipelineLayout,
    pub primitive: PrimitiveState,
    pub vertex_layout: Option<VertexLayout>,
    pub depth_stencil: Option<DepthStencilState>,
}

#[derive(Clone, Copy, Debug)]
pub struct DepthStencilState {
    pub format: TextureFormat,
    pub depth_write_enabled: bool,
    pub depth_compare: CompareFunction,
}

#[derive(Clone, Copy, Debug)]
pub enum CompareFunction {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

pub struct PrimitiveState {
    pub topology: PrimitiveTopology,
}

pub enum PrimitiveTopology {
    TriangleList,
}

#[derive(Clone, Debug)]
pub struct VertexAttribute {
    pub location: u32,
    pub format: VertexFormat,
    pub offset: u32,
}

#[derive(Clone, Copy, Debug)]
pub enum VertexFormat {
    Float32x2,
    Float32x3,
    Float32x4,
}

#[derive(Clone, Debug)]
pub struct VertexLayout {
    pub array_stride: u32,
    pub attributes: Vec<VertexAttribute>,
}

pub trait Swapchain {
    type TextureView: TextureView;
    fn present(&mut self, image_index: u32, wait_semaphores: &[&impl Semaphore]) -> crate::LumeResult<()>;
    fn acquire_next_image(&mut self, signal_semaphore: &impl Semaphore) -> crate::LumeResult<u32>;
    fn get_view(&self, index: u32) -> &Self::TextureView;
}

pub struct SwapchainDescriptor {
    pub width: u32,
    pub height: u32,
    // Add format/vsync options later
}

pub struct BufferDescriptor {
    pub size: u64,
    pub usage: BufferUsage,
    pub mapped_at_creation: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BufferUsage(pub u32);

impl BufferUsage {
    pub const VERTEX: Self = Self(1 << 0);
    pub const INDEX: Self = Self(1 << 1);
    pub const UNIFORM: Self = Self(1 << 2);
    pub const STORAGE: Self = Self(1 << 3);
    pub const COPY_SRC: Self = Self(1 << 4);
    pub const COPY_DST: Self = Self(1 << 5);
}

impl std::ops::BitOr for BufferUsage {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

pub struct BindGroupLayoutDescriptor {
    pub entries: Vec<BindGroupLayoutEntry>,
}

pub struct BindGroupLayoutEntry {
    pub binding: u32,
    pub visibility: ShaderStage,
    pub ty: BindingType,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ShaderStage(pub u32);

impl ShaderStage {
    pub const VERTEX: Self = Self(1 << 0);
    pub const FRAGMENT: Self = Self(1 << 1);
    pub const COMPUTE: Self = Self(1 << 2);
}

impl std::ops::BitOr for ShaderStage {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingType {
    UniformBuffer,
    StorageBuffer,
    SampledTexture,
    Sampler,
}

pub struct BindGroupDescriptor<'a, D: Device> {
    pub layout: &'a D::BindGroupLayout,
    pub entries: Vec<BindGroupEntry<'a, D>>,
}

pub struct BindGroupEntry<'a, D: Device> {
    pub binding: u32,
    pub resource: BindingResource<'a, D>,
}

pub enum BindingResource<'a, D: Device> {
    Buffer(&'a D::Buffer),
    TextureView(&'a D::TextureView),
    Sampler(&'a D::Sampler),
}

pub struct ComputePipelineDescriptor<'a, D: Device> {
    pub shader: &'a D::ShaderModule,
    pub layout: &'a D::PipelineLayout,
}
