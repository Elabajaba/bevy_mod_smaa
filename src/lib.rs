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
    /// Single frame, no temporal supersampling, MSAA disabled.
    Smaa1X,
    // /// Temporal, blends current frame with previous frames.
    // SmaaT2X,
    // /// Msaa + Smaa1X.
    // SmaaS2X,
    // /// Msaa + Temporal Smaa.
    // Smaa4X,
}

pub struct SmaaShaderDefs {
    // pub mode: BevySmaaMode,
    pub quality: SmaaQuality,
    /// `edge_threshold` (aka SMAA_THRESHOLD) specifies the threshold or sensitivity to edges.
    /// Lowering this value you will be able to detect more edges at the expense of
    /// performance.
    ///
    /// Range: [0, 0.5]
    ///   0.1 is a reasonable value, and allows to catch most visible edges.
    ///   0.05 is a rather overkill value, that allows to catch 'em all.
    ///
    ///   If temporal supersampling is used, 0.2 could be a reasonable value, as low
    ///   contrast edges are properly filtered by just 2x.
    pub edge_threshold: f32,
    /// SMAA_DEPTH_THRESHOLD specifies the threshold for depth edge detection.
    /// (0.1 * SMAA_THRESHOLD)
    /// Range: depends on the depth range of the scene.
    pub depth_threshold: f32,
    /// SMAA_MAX_SEARCH_STEPS specifies the maximum steps performed in the
    /// horizontal/vertical pattern searches, at each side of the pixel.
    ///
    /// In number of pixels, it's actually the double. So the maximum line length
    /// perfectly handled by, for example 16, is 64 (by perfectly, we meant that
    /// longer lines won't look as good, but still antialiased).
    ///
    /// Range: [0, 112]
    pub max_search_steps: u32,
    /// SMAA_MAX_SEARCH_STEPS_DIAG specifies the maximum steps performed in the
    /// diagonal pattern searches, at each side of the pixel. In this case we jump
    /// one pixel at time, instead of two.
    ///
    /// Range: [0, 20]
    ///
    /// On high-end machines it is cheap (between a 0.8x and 0.9x slower for 16
    /// steps), but it can have a significant impact on extremely old machines.
    /// Remember, SMAA was developed for the 360 / PS3.
    ///
    /// Define SMAA_DISABLE_DIAG_DETECTION to disable diagonal processing.
    pub max_search_steps_diagonal: u32,
    /// Diagonal processing can be disabled altogether.
    ///
    /// On a modern system this is not recommended, as it adds very little to the
    /// cost while improving quality.
    pub diag_detection: bool,
    ///  SMAA_CORNER_ROUNDING specifies how much sharp corners will be rounded.
    ///
    ///  Range: [0, 100]
    ///
    ///  Define SMAA_DISABLE_CORNER_DETECTION to disable corner processing.
    pub corner_rounding: f32,
    /// Corner detection can be disabled.
    ///
    /// TODO: What is the cost/quality tradeoff?
    pub corner_detection: bool,
    /// If there is an neighbor edge that has SMAA_LOCAL_CONTRAST_FACTOR times
    /// bigger contrast than current edge, current edge will be discarded.
    ///
    /// This allows to eliminate spurious crossing edges, and is based on the fact
    /// that, if there is too much contrast in a direction, that will hide
    /// perceptually contrast in the other neighbors.
    pub local_contrast_adaptation_factor: f32,
    /// Predicated thresholding allows to better preserve texture details and to
    /// improve performance, by decreasing the number of detected edges using an
    /// additional buffer like the light accumulation buffer, object ids or even the
    /// depth buffer (the depth buffer usage may be limited to indoor or short range
    /// scenes).
    ///
    /// It locally decreases the luma or color threshold if an edge is found in an
    /// additional buffer (so the global threshold can be higher).
    ///
    /// This method was developed by Playstation EDGE MLAA team, and used in
    /// Killzone 3, by using the light accumulation buffer. More information here:
    /// http://iryoku.com/aacourse/downloads/06-MLAA-on-PS3.pptx
    ///
    /// TODO: Unimplemented
    pub predication: bool,
    /// Threshold to be used in the additional predication buffer.
    ///
    /// Range: depends on the input, so you'll have to find the magic number that
    /// works for you.
    pub predication_threshold: f32,
    /// How much to scale the global threshold used for luma or color edge
    /// detection when using predication.
    pub predication_scale: f32,
    /// How much to locally decrease the threshold.
    pub predication_strength: f32,
    /// Temporal reprojection allows to remove ghosting artifacts when using
    /// temporal supersampling. We use the CryEngine 3 method which also introduces
    /// velocity weighting. This feature is of extreme importance for totally
    /// removing ghosting. More information here:
    /// http://iryoku.com/aacourse/downloads/13-Anti-Aliasing-Methods-in-CryENGINE-3.pdf
    ///
    /// This must be enabled for SmaaT2X and Smaa4X.
    ///
    /// Note that you'll need to setup a velocity buffer for enabling reprojection.
    /// For static geometry, saving the previous depth buffer is a viable
    /// alternative.
    ///
    /// TODO: Unimplemented, see if we can use bevy's existing velocity buffer
    pub temporal_reprojection: bool,
    /// SMAA_REPROJECTION_WEIGHT_SCALE controls the velocity weighting. It allows to
    /// remove ghosting trails behind the moving object, which are not removed by
    /// just using reprojection. Using low values will exhibit ghosting, while using
    /// high values will disable temporal supersampling under motion.
    ///
    /// Behind the scenes, velocity weighting removes temporal supersampling when
    /// the velocity of the subsamples differs (meaning they are different objects).
    ///
    /// Range: [0, 80]
    pub temporal_reprojection_weight_scale: f32,
}

/// Use these for when you want to modify the default settings for a given preset.
/// Use the SmaaQuality enum for when you want to use an unmodified preset.
impl SmaaShaderDefs {
    pub fn low() -> Self {
        SmaaShaderDefs {
            edge_threshold: 0.15,
            max_search_steps: 4,
            diag_detection: false,
            corner_detection: false,
            ..Default::default()
        }
    }

    pub fn medium() -> Self {
        SmaaShaderDefs {
            edge_threshold: 0.1,
            max_search_steps: 8,
            diag_detection: false,
            corner_detection: false,
            ..Default::default()
        }
    }

    pub fn high() -> Self {
        SmaaShaderDefs {
            edge_threshold: 0.1,
            max_search_steps: 16,
            max_search_steps_diagonal: 8,
            corner_rounding: 25.0,
            diag_detection: true,
            corner_detection: true,
            ..Default::default()
        }
    }

    pub fn ultra() -> Self {
        SmaaShaderDefs {
            edge_threshold: 0.05,
            max_search_steps: 32,
            max_search_steps_diagonal: 16,
            diag_detection: true,
            corner_detection: true,
            ..Default::default()
        }
    }
}

impl Default for SmaaShaderDefs {
    fn default() -> Self {
        let edge_threshold = 0.1;
        SmaaShaderDefs {
            // mode: BevySmaaMode::Smaa1X,
            quality: SmaaQuality::Custom,
            edge_threshold,
            depth_threshold: 0.1 * edge_threshold,
            max_search_steps: 16,
            max_search_steps_diagonal: 8,
            diag_detection: true,
            corner_rounding: 25.0,
            corner_detection: true,
            local_contrast_adaptation_factor: 2.0,
            predication: false,
            predication_threshold: 0.01,
            predication_scale: 2.0,
            predication_strength: 0.4,
            temporal_reprojection: false,
            temporal_reprojection_weight_scale: 30.0,
        }
    }
}

// // On some compilers, discard cannot be used in vertex shaders. Thus, they need
// // to be compiled separately.
// #ifndef SMAA_INCLUDE_VS
// #define SMAA_INCLUDE_VS 1
// #endif
// #ifndef SMAA_INCLUDE_PS
// #define SMAA_INCLUDE_PS 1
// #endif

#[derive(Reflect, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SmaaQuality {
    Low,
    Medium,
    High,
    Ultra,
    Custom,
}

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
