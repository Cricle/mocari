//! Rendering pipeline orchestration.

use super::{EngineError, LoadedModel, RenderContext, WgpuContext, model};

/// Executes the full rendering pipeline.
pub(super) fn render_frame(
    ctx: &mut WgpuContext,
    renderer: &super::WgpuLive2dRenderer,
    models: &mut [LoadedModel],
    render_callbacks: &mut [Box<dyn FnMut(&mut RenderContext)>],
    plugins: &mut [Box<dyn super::Live2dPlugin>],
) -> Result<(), EngineError> {
    // Acquire surface texture
    let frame = match ctx.surface().get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(frame)
        | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
        wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
            return Ok(());
        }
        wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
            let config = ctx.config();
            let w = config.width.max(1);
            let h = config.height.max(1);
            ctx.resize(winit::dpi::PhysicalSize::new(w, h));
            let config = ctx.config();
            for m in models.iter_mut() {
                m.transform.update_matrix(
                    ctx.queue(),
                    &model::fit_model_matrix(m.bounds, config.width, config.height, m.scale),
                );
            }
            return Ok(());
        }
        wgpu::CurrentSurfaceTexture::Validation => {
            return Err(EngineError::Surface(
                "failed to acquire surface texture".into(),
            ));
        }
    };

    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = ctx
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("live2d.engine.encoder"),
        });

    // Mask pass — one render pass per model that has clipping contexts
    for m in models.iter() {
        if !m.mesh.clipping_resources.contexts().is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("live2d.engine.mask_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: m.mesh.mask_target.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            renderer.draw_masks_with_textures(
                &mut pass,
                &m.mesh.mesh_buffers,
                &m.mesh.clipping_resources,
                &m.mesh.textures,
            )?;
        }
    }

    // Main pass — renders all models into the surface view
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.engine.main_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.08,
                        g: 0.09,
                        b: 0.10,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        for m in models.iter() {
            renderer.draw_with_textures_clipping_and_transform(
                &mut pass,
                &m.mesh.mesh_buffers,
                &m.mesh.textures,
                &m.mesh.clipping_resources,
                &m.mesh.mask_target,
                &m.transform,
            )?;
        }
    }

    // Render callbacks + plugins
    {
        let mut render_ctx = RenderContext {
            encoder: &mut encoder,
            view: &view,
            renderer,
            device: ctx.device(),
            queue: ctx.queue(),
        };
        for callback in render_callbacks.iter_mut() {
            callback(&mut render_ctx);
        }
        for plugin in plugins.iter_mut() {
            plugin.on_render(&mut render_ctx);
        }
    }

    ctx.queue().submit([encoder.finish()]);
    ctx.queue().present(frame);

    Ok(())
}
