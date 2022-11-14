use bevy_app::Plugin;
use js_sys::Boolean;
use js_sys::Reflect;
use std::rc::Rc;
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
    session: web_sys::XrSession,
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

        Ok(WebXrContext { session })
    }
}

#[derive(Default)]
pub struct WebXrPlugin;

impl Plugin for WebXrPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let window = gloo_utils::window();
        let webxr_context = app
            .world
            .get_non_send_resource_mut::<WebXrContext>()
            .expect("Webxr context has to be inserted before `app.run()`");
        let session: &web_sys::XrSession = webxr_context.session.as_ref();

        let document = window.document().unwrap();

        let canvas = document
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();

        let gl_attribs = js_sys::Object::new();

        Reflect::set(
            &gl_attribs,
            &JsValue::from_str("xrCompatible"),
            &JsValue::TRUE,
        )
        .unwrap();

        let gl = Rc::new(
            canvas
                .get_context_with_context_options("webgl2", &gl_attribs)
                .expect("Can get webgl2 context from canvas")
                .unwrap()
                .unchecked_into::<web_sys::WebGl2RenderingContext>(),
        );

        let mut state = web_sys::XrRenderStateInit::new();

        let base_layer =
            web_sys::XrWebGlLayer::new_with_web_gl2_rendering_context(&webxr_context.session, &gl)
                .expect("can get base layer");

        state.base_layer(Some(&base_layer));

        session.update_render_state_with_state(&state);
        bevy_log::info!("ayo?");
    }
}
