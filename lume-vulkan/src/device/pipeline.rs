use ash::vk;
use std::ffi::CString;
use lume_core::{LumeError, LumeResult, device::*};
use crate::VulkanDevice;

use std::sync::Arc;
use crate::{VulkanShaderModuleInner, VulkanRenderPassInner, VulkanPipelineLayoutInner, VulkanGraphicsPipelineInner, VulkanComputePipelineInner};

impl VulkanDevice {
    pub fn create_shader_module_impl(&self, code: &[u32]) -> LumeResult<crate::VulkanShaderModule> {
        let create_info = vk::ShaderModuleCreateInfo {
            code_size: code.len() * 4,
            p_code: code.as_ptr(),
            ..Default::default()
        };

        let module = unsafe {
            self.inner.device.create_shader_module(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create shader module: {}", e)))?
        };

        Ok(crate::VulkanShaderModule(Arc::new(VulkanShaderModuleInner {
            module,
            device: self.inner.device.clone(),
        })))
    }

    pub fn create_render_pass_impl(&self, descriptor: RenderPassDescriptor) -> LumeResult<crate::VulkanRenderPass> {
        let mut attachments = Vec::new();
        let mut has_depth = false;

        let color_format = match descriptor.color_format {
            TextureFormat::Bgra8UnormSrgb => vk::Format::B8G8R8A8_SRGB,
            TextureFormat::Rgba8UnormSrgb => vk::Format::R8G8B8A8_SRGB,
            TextureFormat::Rgba8Unorm => vk::Format::R8G8B8A8_UNORM,
            TextureFormat::Rg32Uint => vk::Format::R32G32_UINT,
            TextureFormat::Depth32Float => return Err(LumeError::Generic("Cannot use Depth32Float as color format")),
        };

        attachments.push(vk::AttachmentDescription {
            format: color_format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        });

        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };

        let mut depth_attachment_ref = vk::AttachmentReference::default();
        if let Some(df) = descriptor.depth_stencil_format {
            let depth_format = match df {
                TextureFormat::Depth32Float => vk::Format::D32_SFLOAT,
                _ => return Err(LumeError::Generic("Only Depth32Float is supported for depth stencil currently")),
            };

            attachments.push(vk::AttachmentDescription {
                format: depth_format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::DONT_CARE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                ..Default::default()
            });

            depth_attachment_ref = vk::AttachmentReference {
                attachment: 1,
                layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            };
            has_depth = true;
        }

        let subpass = vk::SubpassDescription {
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_ref,
            p_depth_stencil_attachment: if has_depth { &depth_attachment_ref } else { std::ptr::null() },
            ..Default::default()
        };

        let dependency = vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            ..Default::default()
        };

        let create_info = vk::RenderPassCreateInfo {
            attachment_count: attachments.len() as u32,
            p_attachments: attachments.as_ptr(),
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 1,
            p_dependencies: &dependency,
            ..Default::default()
        };

        let render_pass = unsafe {
            self.inner.device.create_render_pass(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create render pass: {}", e)))?
        };

        Ok(crate::VulkanRenderPass(Arc::new(VulkanRenderPassInner {
            render_pass,
            device: self.inner.device.clone(),
        })))
    }

    pub fn create_pipeline_layout_impl(&self, descriptor: PipelineLayoutDescriptor<Self>) -> LumeResult<crate::VulkanPipelineLayout> {
        let set_layouts: Vec<vk::DescriptorSetLayout> = descriptor.bind_group_layouts.iter().map(|l| l.layout).collect();
        
        let create_info = vk::PipelineLayoutCreateInfo {
            set_layout_count: set_layouts.len() as u32,
            p_set_layouts: set_layouts.as_ptr(),
            ..Default::default()
        };

        let layout = unsafe {
            self.inner.device.create_pipeline_layout(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create pipeline layout: {}", e)))?
        };

        Ok(crate::VulkanPipelineLayout(Arc::new(VulkanPipelineLayoutInner {
            layout,
            set_layouts,
            device: self.inner.device.clone(),
        })))
    }

    pub fn create_graphics_pipeline_impl(&self, descriptor: GraphicsPipelineDescriptor<Self>) -> LumeResult<crate::VulkanGraphicsPipeline> {
        let entry_name = CString::new("main").unwrap();

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::VERTEX,
                module: descriptor.vertex_shader.0.module,
                p_name: entry_name.as_ptr(),
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::FRAGMENT,
                module: descriptor.fragment_shader.0.module,
                p_name: entry_name.as_ptr(),
                ..Default::default()
            },
        ];

        let mut vertex_binding_descriptions = Vec::new();
        let mut vertex_attribute_descriptions = Vec::new();

        if let Some(layout) = &descriptor.vertex_layout {
            vertex_binding_descriptions.push(vk::VertexInputBindingDescription {
                binding: 0,
                stride: layout.array_stride,
                input_rate: vk::VertexInputRate::VERTEX,
            });

            for attr in &layout.attributes {
                vertex_attribute_descriptions.push(vk::VertexInputAttributeDescription {
                    location: attr.location,
                    binding: 0,
                    format: match attr.format {
                        VertexFormat::Float32x2 => vk::Format::R32G32_SFLOAT,
                        VertexFormat::Float32x3 => vk::Format::R32G32B32_SFLOAT,
                        VertexFormat::Float32x4 => vk::Format::R32G32B32A32_SFLOAT,
                    },
                    offset: attr.offset,
                });
            }
        }

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            vertex_binding_description_count: vertex_binding_descriptions.len() as u32,
            p_vertex_binding_descriptions: vertex_binding_descriptions.as_ptr(),
            vertex_attribute_description_count: vertex_attribute_descriptions.len() as u32,
            p_vertex_attribute_descriptions: vertex_attribute_descriptions.as_ptr(),
            ..Default::default()
        };

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
            topology: match descriptor.primitive.topology {
                PrimitiveTopology::TriangleList => vk::PrimitiveTopology::TRIANGLE_LIST,
            },
            ..Default::default()
        };

        let rasterizer = vk::PipelineRasterizationStateCreateInfo {
            polygon_mode: vk::PolygonMode::FILL,
            line_width: 1.0,
            cull_mode: crate::device::resource::map_cull_mode(descriptor.primitive.cull_mode),
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            ..Default::default()
        };

        let multisampling = vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            color_write_mask: vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A,
            blend_enable: vk::FALSE,
            ..Default::default()
        };

        let color_blending = vk::PipelineColorBlendStateCreateInfo {
            attachment_count: 1,
            p_attachments: &color_blend_attachment,
            ..Default::default()
        };

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo {
            dynamic_state_count: dynamic_states.len() as u32,
            p_dynamic_states: dynamic_states.as_ptr(),
            ..Default::default()
        };

        let viewport_state = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            scissor_count: 1,
            ..Default::default()
        };

        let depth_stencil_info = if let Some(ds) = &descriptor.depth_stencil {
            vk::PipelineDepthStencilStateCreateInfo {
                depth_test_enable: vk::TRUE,
                depth_write_enable: if ds.depth_write_enabled { vk::TRUE } else { vk::FALSE },
                depth_compare_op: match ds.depth_compare {
                    CompareFunction::Less => vk::CompareOp::LESS,
                    CompareFunction::LessEqual => vk::CompareOp::LESS_OR_EQUAL,
                    CompareFunction::Greater => vk::CompareOp::GREATER,
                    CompareFunction::GreaterEqual => vk::CompareOp::GREATER_OR_EQUAL,
                    CompareFunction::Equal => vk::CompareOp::EQUAL,
                    CompareFunction::NotEqual => vk::CompareOp::NOT_EQUAL,
                    CompareFunction::Always => vk::CompareOp::ALWAYS,
                    CompareFunction::Never => vk::CompareOp::NEVER,
                },
                ..Default::default()
            }
        } else {
            vk::PipelineDepthStencilStateCreateInfo::default()
        };

        let create_info = vk::GraphicsPipelineCreateInfo {
            stage_count: 2,
            p_stages: shader_stages.as_ptr(),
            p_vertex_input_state: &vertex_input_info,
            p_input_assembly_state: &input_assembly,
            p_viewport_state: &viewport_state,
            p_rasterization_state: &rasterizer,
            p_multisample_state: &multisampling,
            p_color_blend_state: &color_blending,
            p_depth_stencil_state: &depth_stencil_info,
            p_dynamic_state: &dynamic_state_info,
            layout: descriptor.layout.0.layout,
            render_pass: descriptor.render_pass.0.render_pass,
            ..Default::default()
        };

        let pipelines = unsafe {
            self.inner.device.create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .map_err(|(_, e)| LumeError::PipelineCreationFailed(format!("Failed to create graphics pipeline: {:?}", e)))?
        };

        Ok(crate::VulkanGraphicsPipeline(Arc::new(VulkanGraphicsPipelineInner {
            pipeline: pipelines[0],
            layout: descriptor.layout.0.layout,
            device: self.inner.device.clone(),
        })))
    }

    pub fn create_compute_pipeline_impl(&self, descriptor: ComputePipelineDescriptor<Self>) -> LumeResult<crate::VulkanComputePipeline> {
        let entry_name = CString::new("main").unwrap();
        let stage_info = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::COMPUTE,
            module: descriptor.shader.0.module,
            p_name: entry_name.as_ptr(),
            ..Default::default()
        };

        let create_info = vk::ComputePipelineCreateInfo {
            stage: stage_info,
            layout: descriptor.layout.0.layout,
            ..Default::default()
        };

        let pipelines = unsafe {
            self.inner.device.create_compute_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .map_err(|(_, e)| LumeError::PipelineCreationFailed(format!("Failed to create compute pipeline: {:?}", e)))?
        };

        Ok(crate::VulkanComputePipeline(Arc::new(VulkanComputePipelineInner {
            pipeline: pipelines[0],
            layout: descriptor.layout.0.layout,
            device: self.inner.device.clone(),
        })))
    }

    pub fn create_framebuffer_impl(&self, descriptor: FramebufferDescriptor<Self>) -> LumeResult<crate::VulkanFramebuffer> {
        let vk_attachments: Vec<vk::ImageView> = descriptor.attachments.iter().map(|&a| a.view).collect();

        let create_info = vk::FramebufferCreateInfo {
            render_pass: descriptor.render_pass.0.render_pass,
            attachment_count: vk_attachments.len() as u32,
            p_attachments: vk_attachments.as_ptr(),
            width: descriptor.width,
            height: descriptor.height,
            layers: 1,
            ..Default::default()
        };

        let framebuffer = unsafe {
            self.inner.device.create_framebuffer(&create_info, None)
                .map_err(|e| LumeError::ResourceCreationFailed(format!("Failed to create framebuffer: {}", e)))?
        };

        Ok(crate::VulkanFramebuffer {
            framebuffer,
            width: descriptor.width,
            height: descriptor.height,
            device: self.inner.device.clone(),
        })
    }
}
