mod node;

use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    ecs::query::QueryItem,
    prelude::{
        App, Camera, Component, HandleUntyped, Plugin, ReflectComponent, Resource, Shader, With,
    },
    reflect::{Reflect, TypeUuid},
    render::{
        extract_component::ExtractComponent,
        render_resource::{
            BindGroupLayout, CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState,
            MultisampleState, PrimitiveState, RenderPipelineDescriptor, Sampler, ShaderType,
            SpecializedRenderPipeline, TextureFormat,
        },
    },
};
pub use node::SmaaNode;

#[derive(Reflect, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BevySmaaMode {
    Disabled,
    Smaa1X,
    // SmaaT2X,
    // SmaaS2X,
}

// impl From<BevySmaaMode> for SmaaMode {
//     fn from(value: BevySmaaMode) -> Self {
//         match value {
//             BevySmaaMode::Disabled => SmaaMode::Disabled,
//             BevySmaaMode::Smaa1X => SmaaMode::Smaa1X,
//         }
//     }
// }

#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct SmaaSettings {
    /// Enable or disable Smaa.
    pub smaa_mode: BevySmaaMode,
}

impl Default for SmaaSettings {
    fn default() -> Self {
        SmaaSettings {
            smaa_mode: BevySmaaMode::Smaa1X,
        }
    }
}

#[doc(hidden)]
#[derive(Component, ShaderType, Clone)]
pub struct SmaaUniform {
    temp: f32,
}

impl ExtractComponent for SmaaSettings {
    type Query = &'static Self;
    type Filter = With<Camera>;
    type Out = SmaaUniform;

    fn extract_component(item: QueryItem<Self::Query>) -> Option<Self::Out> {
        if item.smaa_mode == BevySmaaMode::Disabled {
            return None;
        }
        Some(SmaaUniform { temp: 1.0 })
    }
}

const SMAA_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 224869679984874407);

pub struct SmaaPlugin;

impl Plugin for SmaaPlugin {
    fn build(&self, app: &mut App) {}
}

// struct BindGroupLayouts {
//     edge_detect_bind_group_layout: wgpu::BindGroupLayout,
//     blend_weight_bind_group_layout: wgpu::BindGroupLayout,
//     neighborhood_blending_bind_group_layout: wgpu::BindGroupLayout,
// }
// struct Pipelines {
//     edge_detect: wgpu::RenderPipeline,
//     blend_weight: wgpu::RenderPipeline,
//     neighborhood_blending: wgpu::RenderPipeline,
// }
// struct Resources {
//     area_texture: wgpu::Texture,
//     search_texture: wgpu::Texture,
//     linear_sampler: wgpu::Sampler,
// }
// struct Targets {
//     rt_uniforms: wgpu::Buffer,
//     color_target: wgpu::TextureView,
//     edges_target: wgpu::TextureView,
//     blend_target: wgpu::TextureView,
// }
// struct BindGroups {
//     edge_detect_bind_group: wgpu::BindGroup,
//     blend_weight_bind_group: wgpu::BindGroup,
//     neighborhood_blending_bind_group: wgpu::BindGroup,
// }

#[derive(Resource)]
pub struct SmaaPipeline {
    texture_bind_group: BindGroupLayout,
    sampler: Sampler,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct SmaaPipelineKey {
    texture_format: TextureFormat,
}

impl SpecializedRenderPipeline for SmaaPipeline {
    type Key = SmaaPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![];
        // if key.denoise {
        //     shader_defs.push("RCAS_DENOISE".into());
        // }
        RenderPipelineDescriptor {
            label: Some("smaa".into()),
            layout: vec![self.texture_bind_group.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: SMAA_SHADER_HANDLE.typed(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: Vec::new(),
        }
    }
}

#[derive(Component)]
pub struct ViewSmaaPipeline(CachedRenderPipelineId);
