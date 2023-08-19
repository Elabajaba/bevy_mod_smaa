use std::sync::Mutex;

use bevy::{
    prelude::{FromWorld, QueryState, With, World},
    render::{
        extract_component::{ComponentUniforms, DynamicUniformIndex},
        render_graph::{Node, NodeRunError, RenderGraphContext},
        render_resource::{BindGroup, BufferId, PipelineCache, TextureViewId},
        renderer::RenderContext,
        view::{ExtractedView, ViewTarget},
    },
};

use crate::{SmaaPipeline, SmaaUniform, ViewSmaaPipeline};

pub struct SmaaNode {
    query: QueryState<
        (
            &'static ViewTarget,
            &'static ViewSmaaPipeline,
            &'static DynamicUniformIndex<SmaaUniform>,
        ),
        With<ExtractedView>,
    >,
    cached_bind_group: Mutex<Option<(BufferId, TextureViewId, BindGroup)>>,
}

impl FromWorld for SmaaNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
            cached_bind_group: Mutex::new(None),
        }
    }
}

impl Node for SmaaNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();
        let pipeline_cache = world.resource::<PipelineCache>();
        let sharpening_pipeline = world.resource::<SmaaPipeline>();
        let uniforms = world.resource::<ComponentUniforms<SmaaUniform>>();

        let Ok((target, pipeline, uniform_index)) = self.query.get_manual(world, view_entity) else { return Ok(()) };

        let uniforms_id = uniforms.buffer().unwrap().id();
        let Some(uniforms) = uniforms.binding() else { return Ok(()) };

        let pipeline = pipeline_cache.get_render_pipeline(pipeline.0).unwrap();

        let view_target = target.post_process_write();
        let source = view_target.source;
        let destination = view_target.destination;

        let mut cached_bind_group = self.cached_bind_group.lock().unwrap();

        let bind_group = match &mut *cached_bind_group {
            Some((buffer_id, texture_id, bind_group))
                if source.id() == *texture_id && uniforms_id == *buffer_id =>
            {
                bind_group
            }
            cached_bind_group => {
                todo!()
            }
        };
        
        Ok(())
    }
}
