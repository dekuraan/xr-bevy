use crate::webxr_context::WebXrContext;
use bevy_ecs::system::Resource;
use wasm_bindgen::JsCast;

pub async fn initialize_webxr() -> InitializedState {
    let webxr_context = WebXrContext::get_context(
        bevy_xr::XrSessionMode::ImmersiveVR,
        bevy_xr::XrReferenceSpaceType::Stage,
    )
    .await
    .unwrap();

    let webgl2_context = webxr_context.canvas.create_webgl2_context();

    // WGpu Setup
    let instance = wgpu::Instance::new(wgpu::Backends::GL);

    let surface = unsafe { instance.create_surface(&webxr_context.canvas) };

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("No suitable GPU adapters found on the system!");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("device"),
                features: adapter.features(),
                limits: adapter.limits(),
            },
            None,
        )
        .await
        .expect("Unable to find a suitable GPU adapter!");

    wasm_bindgen_futures::JsFuture::from(webgl2_context.make_xr_compatible())
        .await
        .expect("Failed to make the webgl context xr-compatible");

    InitializedState {
        webgl2_context,
        webxr_context,
        adapter,
        device,
        queue,
    }
}

#[derive(Resource)]
pub struct InitializedState {
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub webgl2_context: web_sys::WebGl2RenderingContext,
    pub webxr_context: WebXrContext,
}

unsafe impl Send for InitializedState {}

unsafe impl Sync for InitializedState {}

/// Wrapper for `HtmlCanvasElement`
pub struct Canvas {
    inner: web_sys::HtmlCanvasElement,
    id: u32,
}

impl Canvas {
    /// Create new Canvas
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

unsafe impl raw_window_handle::HasRawDisplayHandle for Canvas {
    fn raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
        raw_window_handle::RawDisplayHandle::Web(raw_window_handle::WebDisplayHandle::empty())
    }
}

unsafe impl raw_window_handle::HasRawWindowHandle for Canvas {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        let mut web = raw_window_handle::WebWindowHandle::empty();
        web.id = self.id;

        raw_window_handle::RawWindowHandle::Web(web)
    }
}
