pub mod conversion;
pub mod initialization;
pub mod interaction;
pub mod webxr_context;

use bevy_app::{App, Plugin};
use bevy_ecs::{
    prelude::World,
    system::{Res, ResMut, Resource},
};
use bevy_math::UVec2;
use bevy_render::{
    camera::ManualTextureViews,
    renderer::{RenderAdapterInfo, RenderDevice, RenderQueue},
};
use bevy_utils::Uuid;
use initialization::InitializedState;
use interaction::TrackingSource;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::XrWebGlLayer;
use webxr_context::*;

#[derive(Default)]
pub struct WebXrPlugin;

impl Plugin for WebXrPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.set_runner(webxr_runner);
        setup(&mut app.world);
        app.add_system(update_manual_texture_views);
    }
}

/// Resource that contains the `Uuid` corresponding to WebGlFramebuffer
#[derive(Resource)]
pub struct FramebufferUuid(pub Uuid);

#[derive(Resource)]
/// Wrapper for the WebXR Framebuffer
pub struct VrFramebufferTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl VrFramebufferTexture {
    pub fn new(texture: wgpu::Texture) -> Self {
        Self {
            view: texture.create_view(&Default::default()),
            texture,
        }
    }

    pub fn new_cubemap(texture: wgpu::Texture) -> Self {
        Self {
            view: texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::Cube),
                ..Default::default()
            }),
            texture,
        }
    }
}

fn setup(world: &mut World) {
    let InitializedState {
        webgl2_context,
        webxr_context,
        adapter,
        device,
        queue,
    } = world.remove_resource().unwrap();
    let adapter_info = adapter.get_info();
    let layer_init = web_sys::XrWebGlLayerInit::new();

    let xr_gl_layer = web_sys::XrWebGlLayer::new_with_web_gl2_rendering_context_and_layer_init(
        &webxr_context.session,
        &webgl2_context,
        &layer_init,
    )
    .unwrap();

    let mut render_state_init = web_sys::XrRenderStateInit::new();
    render_state_init
        .depth_near(0.001)
        .base_layer(Some(&xr_gl_layer));

    webxr_context
        .session
        .update_render_state_with_state(&render_state_init);

    world.insert_resource(RenderDevice::from(Arc::new(device)));
    world.insert_resource(RenderQueue(Arc::new(queue)));
    world.insert_resource(RenderAdapterInfo(adapter_info));
    world.insert_non_send_resource(webxr_context);
    let id = Uuid::new_v4();
    world.insert_resource(FramebufferUuid(id));
}

/// System that updates `ManualTextureViews` with the new `WebGlFramebuffer`
fn update_manual_texture_views(
    frame: bevy_ecs::prelude::NonSend<web_sys::XrFrame>,
    device: Res<RenderDevice>,
    framebuffer_uuid: Res<FramebufferUuid>,
    mut manual_tex_view: ResMut<ManualTextureViews>,
) {
    let base_layer: XrWebGlLayer = frame.session().render_state().base_layer().unwrap();

    //Reflect hack because base_layer.framebuffer is technically null
    let framebuffer: web_sys::WebGlFramebuffer =
        js_sys::Reflect::get(&base_layer, &"framebuffer".into())
            .unwrap()
            .into();
    let device = device.wgpu_device();

    let framebuffer_colour_attachment: VrFramebufferTexture = create_view_from_device_framebuffer(
        device,
        framebuffer.clone(),
        &base_layer,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        "Device Framebuffer (Color)",
    );

    let resolution = UVec2::new(
        base_layer.framebuffer_width(),
        base_layer.framebuffer_height(),
    );

    manual_tex_view.insert(
        framebuffer_uuid.0,
        (framebuffer_colour_attachment.view.into(), resolution),
    );
}

/// Bevy runner that works with WebXR
fn webxr_runner(mut app: App) {
    let webxr_context = app.world.get_non_send_resource::<WebXrContext>().unwrap();
    let session = webxr_context.session.clone();
    type XrFrameHandler = Closure<dyn FnMut(f64, web_sys::XrFrame)>;
    let f: Rc<RefCell<Option<XrFrameHandler>>> = Rc::new(RefCell::new(None));
    let g: Rc<RefCell<Option<XrFrameHandler>>> = f.clone();


    *g.borrow_mut() = Some(Closure::new(move |_time: f64, frame: web_sys::XrFrame| {
        app.world.insert_non_send_resource(frame.clone());

        let (space, space_type) = webxr_context.space_info.clone();
        // update the current frame inside tracking soource
        let tracking_source = TrackingSource::new(space, space_type, frame.clone());
        // Resource used to track poses
        app.world
            .insert_resource(bevy_xr::XrTrackingSource::new(Box::new(tracking_source)));

        app.update();

        let session = frame.session();
        session.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }));
    session.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref());
}

pub fn create_view_from_device_framebuffer(
    device: &wgpu::Device,
    framebuffer: web_sys::WebGlFramebuffer,
    base_layer: &web_sys::XrWebGlLayer,
    format: wgpu::TextureFormat,
    label: &'static str,
) -> VrFramebufferTexture {
    VrFramebufferTexture::new(unsafe {
        device.create_texture_from_hal::<wgpu_hal::api::Gles>(
            wgpu_hal::gles::Texture {
                inner: wgpu_hal::gles::TextureInner::ExternalFramebuffer { inner: framebuffer },
                mip_level_count: 1,
                array_layer_count: 1,
                format,
                format_desc: wgpu_hal::gles::TextureFormatDesc {
                    internal: glow::RGBA,
                    external: glow::RGBA,
                    data_type: glow::UNSIGNED_BYTE,
                },
                copy_size: wgpu_hal::CopyExtent {
                    width: base_layer.framebuffer_width(),
                    height: base_layer.framebuffer_height(),
                    depth: 1,
                },
                is_cubemap: false,
                drop_guard: None,
            },
            &wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: base_layer.framebuffer_width(),
                    height: base_layer.framebuffer_height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            },
        )
    })
}
