use lume_core::{Instance, InstanceDescriptor, Backend, Device, device::{BufferDescriptor, BufferUsage, ShaderStage, BindingType, BindGroupLayoutDescriptor, BindGroupLayoutEntry, PipelineLayoutDescriptor, ComputePipelineDescriptor, CommandPool, CommandBuffer, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer}};
use lume_vulkan::{VulkanInstance, VulkanDevice};

fn main() {
    env_logger::init();
    
    let instance_desc = InstanceDescriptor {
        name: "Lume Compute Example",
        backend: Backend::Vulkan,
    };
    
    let instance = VulkanInstance::new(instance_desc).expect("Failed to create Lume Instance");
    let device = instance.request_device(None).expect("Failed to request device");

    // 1. Create Data
    let data_size = 64;
    let mut initial_data = vec![1.0f32; data_size];
    let data_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(initial_data.as_ptr() as *const u8, initial_data.len() * 4)
    };

    // 2. Create Buffer
    let buffer = device.create_buffer(BufferDescriptor {
        size: (data_size * 4) as u64,
        usage: BufferUsage::STORAGE | BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
        mapped_at_creation: true,
    }).expect("Failed to create buffer");

    buffer.write_data(0, data_bytes).expect("Failed to write data");

    // 3. Setup Pipeline
    let shader_spv = include_bytes!("../../shaders/test.comp.spv");
    let shader_code = unsafe {
        std::slice::from_raw_parts(shader_spv.as_ptr() as *const u32, shader_spv.len() / 4)
    };
    let shader_module = device.create_shader_module(shader_code).expect("Failed to create shader module");

    let bind_group_layout = device.create_bind_group_layout(BindGroupLayoutDescriptor {
        entries: vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStage::COMPUTE,
                ty: BindingType::StorageBuffer,
            },
        ],
    }).expect("Failed to create bind group layout");

    let layout = device.create_pipeline_layout(PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
    }).expect("Failed to create layout");

    let pipeline = device.create_compute_pipeline(ComputePipelineDescriptor {
        shader: &shader_module,
        layout: &layout,
    }).expect("Failed to create compute pipeline");

    let bind_group = device.create_bind_group(BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: vec![
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(&buffer),
            },
        ],
    }).expect("Failed to create bind group");

    // 4. Dispatch
    let command_pool = device.create_command_pool().expect("Failed to create command pool");
    let mut cmd = command_pool.allocate_command_buffer().expect("Failed to allocate command buffer");

    cmd.begin().expect("Failed to begin cmd");
    cmd.bind_compute_pipeline(&pipeline);
    cmd.bind_bind_group(0, &bind_group);
    cmd.dispatch(1, 1, 1);
    cmd.compute_barrier();
    cmd.end().expect("Failed to end cmd");

    device.submit(&[&cmd], &[], &[]).expect("Failed to submit compute cmd");
    device.wait_idle().expect("Wait idle failed");

    // 5. Read back
    let mut result_data = vec![0.0f32; data_size];
    let result_bytes: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(result_data.as_mut_ptr() as *mut u8, result_data.len() * 4)
    };

    buffer.read_data(0, result_bytes).expect("Failed to read back data");

    println!("Compute Result (first 10):");
    for i in 0..10 {
        println!("  [{}] {} -> {}", i, initial_data[i], result_data[i]);
    }

    if result_data[0] == 2.0 {
        println!("SUCCESS: Compute working correctly!");
    } else {
        println!("FAILURE: Expected 2.0, got {}", result_data[0]);
    }
}
