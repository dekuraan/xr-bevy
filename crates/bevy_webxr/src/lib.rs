use bevy_app::App;
use bevy_app::Plugin;
use js_sys::Boolean;
use raw_window_handle;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

// WebXR <-> Bevy XR Conversion
pub(crate) trait XrFrom<T> {
    fn xr_from(_: T) -> Self;
}

pub(crate) trait XrInto<T> {
    fn xr_into(self) -> T;
}

impl<T, U> XrInto<U> for T
where
    U: XrFrom<T>,
{
    fn xr_into(self) -> U {
        U::xr_from(self)
    }
}

// XR Conversion Impls

impl XrFrom<web_sys::XrSessionMode> for bevy_xr::XrSessionMode {
    fn xr_from(mode: web_sys::XrSessionMode) -> Self {
        match mode {
            web_sys::XrSessionMode::Inline => bevy_xr::XrSessionMode::InlineVR,
            web_sys::XrSessionMode::ImmersiveVr => bevy_xr::XrSessionMode::ImmersiveVR,
            web_sys::XrSessionMode::ImmersiveAr => bevy_xr::XrSessionMode::ImmersiveAR,
            _ => panic!("Invalid XrSessionMode"),
        }
    }
}

impl XrFrom<bevy_xr::XrSessionMode> for web_sys::XrSessionMode {
    fn xr_from(web_xr: bevy_xr::XrSessionMode) -> Self {
        match web_xr {
            bevy_xr::XrSessionMode::ImmersiveVR => web_sys::XrSessionMode::ImmersiveVr,
            bevy_xr::XrSessionMode::ImmersiveAR => web_sys::XrSessionMode::ImmersiveAr,
            bevy_xr::XrSessionMode::InlineVR => web_sys::XrSessionMode::Inline,
            //TODO: remove bevy_xr::XrSessionMode::InlineAr?
            bevy_xr::XrSessionMode::InlineAR => web_sys::XrSessionMode::Inline,
        }
    }
}

pub struct WebXrContext {
    pub session: Rc<RefCell<web_sys::XrSession>>,
    pub canvas: Canvas,
    // adapter: Adapter,
    // device: Arc<Device>,
    // queue: Queue,
}

impl WebXrContext {
    /// Get a WebXrContext, you must do this in an async function, so you have to call this before `bevy_app::App::run()` in an async main fn and insett it
    pub async fn get_context(mode: bevy_xr::XrSessionMode) -> Result<Self, JsValue> {
        let mode = mode.xr_into();
        let window = gloo_utils::window();
        let navigator = window.navigator();
        let xr_system = navigator.xr();

        let session_supported =
            JsFuture::from(xr_system.is_session_supported(web_sys::XrSessionMode::ImmersiveVr))
                .await?
                .dyn_into::<Boolean>()?
                .value_of();

        if !session_supported {
            return Err("XrSessionMode not supported.".into());
        }

        let session = JsFuture::from(xr_system.request_session(mode))
            .await?
            .dyn_into::<web_sys::XrSession>()?;

        let canvas = Canvas::default();

        Ok(WebXrContext {
            session: Rc::new(RefCell::new(session)),
            canvas,
        })
    }
}

#[derive(Default)]
pub struct WebXrPlugin;

impl Plugin for WebXrPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        bevy_log::info!("in webxr");
        app.set_runner(webxr_runner);
        // let window = gloo_utils::window();
        // let webxr_context = app
        //     .world
        //     .get_non_send_resource_mut::<WebXrContext>()
        //     .expect("Webxr context has to be inserted before `app.run()`");
        // // let session: &web_sys::XrSession = webxr_context.session.as_ref();
        // // let mut state = web_sys::XrRenderStateInit::new();

        // // let base_layer =
        // //     web_sys::XrWebGlLayer::new_with_web_gl2_rendering_context(&webxr_context.session, &gl)
        // //         .expect("can get base layer");

        // // state.base_layer(Some(&base_layer));

        // // session.update_render_state_with_state(&state);
        // app.insert_resource(bevy_render::renderer::RenderDevice::from(
        //     webxr_context.device.clone(),
        // ));
    }
}

//TODO: get rid of rcrefcell and use frame.session
fn webxr_runner(mut app: App) {
    let webxr_context = app.world.get_non_send_resource::<WebXrContext>().unwrap();
    let session = webxr_context.session.clone();
    type XrFrameHandler = Closure<dyn FnMut(f64, web_sys::XrFrame)>;
    let f: Rc<RefCell<Option<XrFrameHandler>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    let closure_session = session.clone();
    *g.borrow_mut() = Some(Closure::new(move |_time: f64, _frame: web_sys::XrFrame| {
        //Tick Bevy World
        app.update();

        let session = closure_session.borrow();

        session.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }));

    session
        .borrow()
        .request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref());
}

pub struct Canvas {
    inner: web_sys::HtmlCanvasElement,
    id: u32,
}

impl Canvas {
    pub fn new_with_id(id: u32) -> Self {
        let canvas: web_sys::HtmlCanvasElement = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("canvas")
            .unwrap()
            .unchecked_into();

        let body = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap();

        canvas
            .set_attribute("data-raw-handle", &id.to_string())
            .unwrap();

        body.append_child(&web_sys::Element::from(canvas.clone()))
            .unwrap();

        Self { inner: canvas, id }
    }

    pub fn create_webgl2_context(
        &self,
        // options: ContextCreationOptions,
    ) -> web_sys::WebGl2RenderingContext {
        let js_gl_attribs = js_sys::Object::new();
        js_sys::Reflect::set(
            &js_gl_attribs,
            &"xrCompatible".into(),
            &wasm_bindgen::JsValue::TRUE,
        )
        .expect("Failed to set xrCompatible");
        // WebGL silently ignores any stencil writing or testing if this is not set.
        // (Atleast on Chrome). What a fantastic design decision.
        // js_sys::Reflect::set(
        //     &js_gl_attribs,
        //     &"stencil".into(),
        //     &wasm_bindgen::JsValue::from_bool(options.stencil),
        // )
        // .expect("Failed to set stencil");

        self.inner
            .get_context_with_context_options("webgl2", &js_gl_attribs)
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::WebGl2RenderingContext>()
            .unwrap()
    }
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new_with_id(0)
    }
}

unsafe impl raw_window_handle::HasRawWindowHandle for Canvas {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        let mut web = raw_window_handle::WebHandle::empty();
        web.id = self.id;

        raw_window_handle::RawWindowHandle::Web(web)
    }
}

// unsafe impl raw_window_handle::HasRawDisplayHandle for Canvas {
//     fn raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
//         raw_window_handle::RawDisplayHandle::Web(raw_window_handle::WebDisplayHandle::empty())
//     }
// }

pub fn create_view_from_device_framebuffer(
    device: &wgpu::Device,
    framebuffer: web_sys::WebGlFramebuffer,
    base_layer: &web_sys::XrWebGlLayer,
    format: wgpu::TextureFormat,
    label: &'static str,
) -> Texture {
    Texture::new(unsafe {
        device.create_texture_from_hal::<wgpu_hal::gles::Api>(
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

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl Texture {
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
