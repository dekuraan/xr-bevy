pub mod camera;
mod conversion;
mod utils;

use conversion::*;
mod interaction;
mod presentation;
mod swapchain;
use swapchain::*;

use ash::vk;
use ash::vk::Handle;

use bevy_render::{
    camera::{CameraPlugin, CameraProjection, CameraProjectionPlugin, ManualTextureViews},
    prelude::Msaa,
    renderer,
};

use bevy_utils::Uuid;
pub use interaction::*;

use bevy_app::{App, AppExit, Plugin};
use bevy_ecs::{
    event::{Events, ManualEventReader},
    system::Resource,
};
use bevy_xr::{
    presentation::{XrEnvironmentBlendMode, XrGraphicsContext, XrInteractionMode},
    XrActionDescriptor, XrActionSet, XrActionType, XrProfileDescriptor, XrProfiles, XrSessionMode,
    XrSystem, XrTrackingSource, XrVibrationEvent, XrVisibilityState,
};
use openxr::{self as xr, sys};
use parking_lot::RwLock;
use presentation::GraphicsContextHandles;
use serde::{Deserialize, Serialize};
use xr::{ActiveActionSet, ViewStateFlags};

use std::{error::Error, ops::Deref, sync::Arc, thread, time::Duration};
use wgpu::{TextureUsages, TextureViewDescriptor};
use wgpu_hal::TextureUses;

pub use crate::camera::XrPawn;
use crate::camera::XrViews;

// The form-factor is selected at plugin-creation-time and cannot be changed anymore for the entire
// lifetime of the app. This will restrict which XrSessionMode can be selected.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum OpenXrFormFactor {
    HeadMountedDisplay,
    Handheld,
}

enum SessionBackend {
    Vulkan(xr::Session<xr::Vulkan>),
    #[cfg(all(windows, feature = "wip"))]
    D3D11(xr::Session<xr::D3D11>),
}

enum FrameStream {
    Vulkan(xr::FrameStream<xr::Vulkan>),
    #[cfg(all(windows, feature = "wip"))]
    D3D11(xr::FrameStream<xr::D3D11>),
}

#[derive(Clone, Resource)]
pub struct OpenXrSession {
    inner: Option<xr::Session<xr::AnyGraphics>>,
    _wgpu_device: Arc<wgpu::Device>,
}

impl Deref for OpenXrSession {
    type Target = xr::Session<xr::AnyGraphics>;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl Drop for OpenXrSession {
    fn drop(&mut self) {
        // Drop OpenXR session before wgpu::Device.
        self.inner.take();
    }
}

#[derive(Debug)]
pub enum OpenXrError {
    #[cfg(any(target_os = "android", target_os = "linux"))]
    Loader(xr::LoadError),
    InstanceCreation(sys::Result),
    UnsupportedFormFactor,
    UnavailableFormFactor,
    GraphicsCreation(Box<dyn Error>),
    SwapchainCreation(sys::Result),
}

fn selected_extensions(entry: &xr::Entry) -> xr::ExtensionSet {
    let available = entry.enumerate_extensions().unwrap();

    let mut exts = xr::ExtensionSet::default();
    // Complete list: https://www.khronos.org/registry/OpenXR/specs/1.0/html/xrspec.html#extension-appendices-list
    exts.khr_composition_layer_depth = available.khr_composition_layer_depth;
    // todo: set depth layer
    exts.khr_vulkan_enable = available.khr_vulkan_enable;
    //  required for how we use openxr library
    exts.khr_vulkan_enable2 = true;
    if cfg!(debug_assertions) {
        exts.ext_debug_utils = available.ext_debug_utils;
    }
    exts.ext_eye_gaze_interaction = available.ext_eye_gaze_interaction;
    // todo: implement eye tracking
    exts.ext_hand_tracking = available.ext_hand_tracking;
    exts.ext_hp_mixed_reality_controller = available.ext_hp_mixed_reality_controller;
    exts.ext_performance_settings = available.ext_performance_settings;
    // todo: implement performance API
    exts.ext_samsung_odyssey_controller = available.ext_samsung_odyssey_controller;
    exts.ext_thermal_query = available.ext_thermal_query;
    // todo: implement thermal API
    exts.fb_color_space = available.fb_color_space;
    // todo: implement color space API
    exts.fb_display_refresh_rate = available.fb_display_refresh_rate;
    // todo: implement refresh rate API
    exts.htc_vive_cosmos_controller_interaction = available.htc_vive_cosmos_controller_interaction;
    exts.huawei_controller_interaction = available.huawei_controller_interaction;
    exts.msft_hand_interaction = available.msft_hand_interaction;
    // exts.msft_scene_unserstanding = available.msft_scene_unserstanding -> not available in openxrs
    // todo: implement scene understanding API
    // exts.msft_scene_unserstanding_serialization = available.msft_scene_unserstanding_serialization -> not available in openxrs
    // todo: implement scene serialization
    exts.msft_secondary_view_configuration = available.msft_secondary_view_configuration;
    // todo: implement secondary view. This requires integration with winit.
    exts.msft_spatial_anchor = available.msft_spatial_anchor;
    // todo: implement spatial anchors API
    exts.varjo_quad_views = available.varjo_quad_views;

    #[cfg(target_os = "android")]
    {
        exts.khr_android_create_instance = available.khr_android_create_instance;
        exts.khr_android_thread_settings = available.khr_android_thread_settings;
        exts.ext_debug_utils = true;
        // todo: set APPLICATION_MAIN and RENDER_MAIN threads
    }
    #[cfg(windows)]
    {
        exts.khr_d3d11_enable = available.khr_d3d11_enable;
    }
    bevy_log::info!("selected xr extensions: \n{:#?}", exts);

    exts
}

#[derive(Resource)]
pub struct XrInstanceRes(pub xr::Instance);

#[derive(Resource)]
pub struct OpenXrContext {
    instance: xr::Instance,
    form_factor: xr::FormFactor,
    system: xr::SystemId,
    // Note: the lifecycle of graphics handles is managed by wgpu objects
    graphics_handles: GraphicsContextHandles,
    wgpu_device: Arc<wgpu::Device>,
    graphics_context: Option<XrGraphicsContext>,
}

impl OpenXrContext {
    fn new(form_factor: OpenXrFormFactor) -> Result<Self, OpenXrError> {
        #[cfg(any(target_os = "android", target_os = "linux"))]
        let entry = unsafe { xr::Entry::load().map_err(OpenXrError::Loader)? };
        #[cfg(not(any(target_os = "android", target_os = "linux")))]
        let entry = xr::Entry::linked();

        #[cfg(target_os = "android")]
        entry.initialize_android_loader().unwrap();

        let extensions = selected_extensions(&entry);

        let instance = entry
            .create_instance(
                &xr::ApplicationInfo {
                    application_name: "Bevy App",
                    application_version: 0,
                    engine_name: "Bevy Engine",
                    engine_version: 0,
                },
                &extensions,
                &[], // todo: add debug layer
            )
            .map_err(OpenXrError::InstanceCreation)?;

        let form_factor = match form_factor {
            OpenXrFormFactor::HeadMountedDisplay => xr::FormFactor::HEAD_MOUNTED_DISPLAY,
            OpenXrFormFactor::Handheld => xr::FormFactor::HEAD_MOUNTED_DISPLAY,
        };

        let system = instance.system(form_factor).map_err(|e| match e {
            sys::Result::ERROR_FORM_FACTOR_UNSUPPORTED => OpenXrError::UnsupportedFormFactor,
            sys::Result::ERROR_FORM_FACTOR_UNAVAILABLE => OpenXrError::UnavailableFormFactor,
            e => panic!("{}", e), // should never happen
        })?;

        let (graphics_handles, graphics_context) =
            presentation::create_graphics_context(&instance, system)
                .map_err(OpenXrError::GraphicsCreation)?;

        Ok(Self {
            instance,
            form_factor,
            system,
            graphics_handles,
            wgpu_device: graphics_context.device.clone(),
            graphics_context: Some(graphics_context),
        })
    }
}

fn get_system_info(
    instance: &xr::Instance,
    system: xr::SystemId,
    mode: XrSessionMode,
) -> Option<(xr::ViewConfigurationType, xr::EnvironmentBlendMode)> {
    let view_type = match mode {
        XrSessionMode::ImmersiveVR | XrSessionMode::ImmersiveAR => {
            if instance.exts().varjo_quad_views.is_some() {
                xr::ViewConfigurationType::PRIMARY_QUAD_VARJO
            } else {
                xr::ViewConfigurationType::PRIMARY_STEREO
            }
        }
        XrSessionMode::InlineVR | XrSessionMode::InlineAR => {
            xr::ViewConfigurationType::PRIMARY_MONO
        }
    };

    let blend_modes = match instance.enumerate_environment_blend_modes(system, view_type) {
        Ok(blend_modes) => blend_modes,
        _ => return None,
    };

    let blend_mode = match mode {
        XrSessionMode::ImmersiveVR | XrSessionMode::InlineVR => blend_modes
            .into_iter()
            .find(|b| *b == xr::EnvironmentBlendMode::OPAQUE)?,
        XrSessionMode::ImmersiveAR | XrSessionMode::InlineAR => blend_modes
            .iter()
            .cloned()
            .find(|b| *b == xr::EnvironmentBlendMode::ALPHA_BLEND)
            .or_else(|| {
                blend_modes
                    .into_iter()
                    .find(|b| *b == xr::EnvironmentBlendMode::ADDITIVE)
            })?,
    };

    Some((view_type, blend_mode))
}

#[derive(Default)]
pub struct OpenXrPlugin;

impl Plugin for OpenXrPlugin {
    fn build(&self, app: &mut App) {
        if !app.world.contains_resource::<OpenXrContext>() {
            let context =
                OpenXrContext::new(OpenXrFormFactor::HeadMountedDisplay).unwrap_or_else(|_| {
                    match OpenXrContext::new(OpenXrFormFactor::Handheld) {
                        Ok(context) => context,
                        // In case OpenXR is suported, there should be always at least one supported
                        // form factor. If "Handheld" is unsupported, "HeadMountedDisplay" is
                        // supported (but in this case unavailable).
                        Err(
                            OpenXrError::UnsupportedFormFactor | OpenXrError::UnavailableFormFactor,
                        ) => panic!(
                            "OpenXR: No available form factors. Consider manually handling {}",
                            "the creation of the OpenXrContext resource."
                        ),
                        Err(OpenXrError::InstanceCreation(sys::Result::ERROR_RUNTIME_FAILURE)) => {
                            panic!(
                                "OpenXR: Failed to create OpenXrContext: {:?}\n{} {}",
                                sys::Result::ERROR_RUNTIME_FAILURE,
                                "Is your headset connected? Also, consider manually handling",
                                "the creation of the OpenXrContext resource."
                            )
                        }
                        Err(e) => panic!(
                            "OpenXR: Failed to create OpenXrContext: {:?}\n{} {}",
                            e,
                            "Consider manually handling",
                            "the creation of the OpenXrContext resource."
                        ),
                    }
                });
            app.world.insert_resource(context);
        }

        let mut context = app.world.get_resource_mut::<OpenXrContext>().unwrap();
        let graphics_context = context.graphics_context.take().unwrap();
        println!("got graphics context");

        let dev = renderer::RenderDevice::from(graphics_context.device.clone());
        let queue = renderer::RenderQueue(graphics_context.queue.clone());
        let adapter_info = renderer::RenderAdapterInfo(graphics_context.adapter_info.clone());

        //override default render stuff
        app.insert_resource(dev)
            .insert_resource(queue)
            .insert_resource(adapter_info);

        app.insert_resource::<XrGraphicsContext>(graphics_context)
            .set_runner(runner);

        app.insert_resource(Msaa { samples: 1 });
    }
}

fn setup_interaction(system: &mut XrSystem) {
    if cfg!(target_os = "android") {
        let oculus_profile = XrProfileDescriptor {
            profile: OCULUS_TOUCH_PROFILE.into(),
            bindings: vec![
                (
                    XrActionDescriptor {
                        name: "left_trigger".into(),
                        action_type: XrActionType::Button {
                            touch: true,
                            click: false,
                            value: true,
                        },
                    },
                    "/user/hand/left/input/trigger".into(),
                ),
                (
                    XrActionDescriptor {
                        name: "left_primary".into(),
                        action_type: XrActionType::Button {
                            touch: true,
                            click: true,
                            value: false,
                        },
                    },
                    "/user/hand/left/input/x".into(),
                ),
                (
                    XrActionDescriptor {
                        name: "right_trigger".into(),
                        action_type: XrActionType::Button {
                            touch: true,
                            click: false,
                            value: true,
                        },
                    },
                    "/user/hand/right/input/trigger".into(),
                ),
                (
                    XrActionDescriptor {
                        name: "right_primary".into(),
                        action_type: XrActionType::Button {
                            touch: true,
                            click: true,
                            value: false,
                        },
                    },
                    "/user/hand/right/input/a".into(),
                ),
            ],
            tracked: true,
            has_haptics: true,
        };
        system.set_action_set(vec![oculus_profile]);
    } else {
        //  TODO: use runtime settings or build-time features to pick active Profile
        let action_set = XrProfileDescriptor {
            profile: VALVE_INDEX_PROFILE.into(),
            bindings: vec![
                (
                    XrActionDescriptor {
                        name: "left_trigger".into(),
                        action_type: XrActionType::Button {
                            touch: true,
                            click: true,
                            value: true,
                        },
                    },
                    "/user/hand/left/input/trigger".into(),
                ),
                (
                    XrActionDescriptor {
                        name: "left_primary".into(),
                        action_type: XrActionType::Button {
                            touch: true,
                            click: true,
                            value: false,
                        },
                    },
                    "/user/hand/left/input/a".into(),
                ),
                (
                    XrActionDescriptor {
                        name: "left_secondary".into(),
                        action_type: XrActionType::Button {
                            touch: true,
                            click: true,
                            value: false,
                        },
                    },
                    "/user/hand/left/input/b".into(),
                ),
                (
                    XrActionDescriptor {
                        name: "right_trigger".into(),
                        action_type: XrActionType::Button {
                            touch: true,
                            click: true,
                            value: true,
                        },
                    },
                    "/user/hand/right/input/trigger".into(),
                ),
                (
                    XrActionDescriptor {
                        name: "right_primary".into(),
                        action_type: XrActionType::Button {
                            touch: true,
                            click: true,
                            value: false,
                        },
                    },
                    "/user/hand/right/input/a".into(),
                ),
                (
                    XrActionDescriptor {
                        name: "right_secondary".into(),
                        action_type: XrActionType::Button {
                            touch: true,
                            click: true,
                            value: false,
                        },
                    },
                    "/user/hand/right/input/a".into(),
                ),
            ],
            tracked: true,
            has_haptics: true,
        };
        system.set_action_set(vec![action_set]);
    }
}

// Currently, only the session loop is implemented. If the session is destroyed or fails to
// create, the app will exit.
// todo: Implement the instance loop when the the lifecycle API is implemented.
fn runner(mut app: App) {
    let ctx = app.world.remove_resource::<OpenXrContext>().unwrap();

    app.world
        .insert_resource(XrInstanceRes(ctx.instance.clone()));

    let mut app_exit_event_reader = ManualEventReader::default();

    let interaction_mode = if ctx.form_factor == xr::FormFactor::HEAD_MOUNTED_DISPLAY {
        XrInteractionMode::WorldSpace
    } else {
        XrInteractionMode::ScreenSpace
    };
    app.world.insert_resource(interaction_mode);

    // Find the available session modes
    let available_session_modes = [
        XrSessionMode::ImmersiveVR,
        XrSessionMode::ImmersiveAR,
        XrSessionMode::InlineVR,
        XrSessionMode::InlineAR,
    ]
    .iter()
    .filter_map(|mode| get_system_info(&ctx.instance, ctx.system, *mode).map(|_| *mode))
    .collect();

    app.world
        .insert_resource(XrSystem::new(available_session_modes));
    println!("inserted XrSystem");

    if app_exit_event_reader
        .iter(&app.world.get_resource_mut::<Events<AppExit>>().unwrap())
        .next_back()
        .is_some()
    {
        println!("app exit event received");
        return;
    }

    let mut xr_system = app.world.get_resource_mut::<XrSystem>().unwrap();
    setup_interaction(&mut xr_system);

    let mode = xr_system.selected_session_mode();
    let bindings = xr_system.action_set();
    dbg!(bindings.iter().map(|b| &b.profile).collect::<Vec<_>>());

    let interaction_context = InteractionContext::new(&ctx.instance, bindings);

    // Remove XrSystem. The user cannot make any more changes to the session mode.
    // todo: when the lifecycle API is implemented, allow the user to change the session mode at any
    // moment.
    // app.world.remove_resource::<XrSystem>();

    let (view_type, blend_mode) = get_system_info(&ctx.instance, ctx.system, mode).unwrap();

    let environment_blend_mode = match blend_mode {
        xr::EnvironmentBlendMode::OPAQUE => XrEnvironmentBlendMode::Opaque,
        xr::EnvironmentBlendMode::ALPHA_BLEND => XrEnvironmentBlendMode::AlphaBlend,
        xr::EnvironmentBlendMode::ADDITIVE => XrEnvironmentBlendMode::Additive,
        _ => unreachable!(),
    };
    app.world.insert_resource(environment_blend_mode);

    let (vk_session, session, _graphics_session, mut frame_waiter, mut frame_stream) =
        match ctx.graphics_handles {
            GraphicsContextHandles::Vulkan {
                instance,
                physical_device,
                device,
                queue_family_index,
                queue_index,
            } => {
                let (session, frame_waiter, frame_stream) = unsafe {
                    ctx.instance
                        .create_session(
                            ctx.system,
                            &xr::vulkan::SessionCreateInfo {
                                instance: instance.handle().as_raw() as *const _,
                                physical_device: physical_device.as_raw() as *const _,
                                device: device.handle().as_raw() as *const _,
                                queue_family_index,
                                queue_index,
                            },
                        )
                        .unwrap()
                };
                (
                    session.clone(),
                    session.clone().into_any_graphics(),
                    SessionBackend::Vulkan(session),
                    frame_waiter,
                    frame_stream,
                )
            }
        };

    let session = OpenXrSession {
        inner: Some(session),
        _wgpu_device: ctx.wgpu_device.clone(),
    };

    // The user can have a limited access to the OpenXR session using OpenXrSession, which is
    // clonable but safe because of the _wgpu_device internal handle.
    app.world.insert_resource(session.clone());

    session
        .attach_action_sets(&[&interaction_context.action_set.lock()])
        .unwrap();

    let tracking_context = Arc::new(OpenXrTrackingContext::new(
        &ctx.instance,
        ctx.system,
        &interaction_context,
        session.clone(),
    ));

    let next_vsync_time = Arc::new(RwLock::new(xr::Time::from_nanos(0)));

    let tracking_source = TrackingSource {
        view_type,
        action_set: interaction_context.action_set.clone(),
        session: session.clone(),
        context: tracking_context.clone(),
        next_vsync_time: next_vsync_time.clone(),
    };

    app.world.init_resource::<XrActionSet>();
    app.world
        .insert_resource(OpenXrTrackingContextRes(tracking_context.clone()));
    app.world
        .insert_resource(XrTrackingSource::new(Box::new(tracking_source)));

    // todo: use these views limits and recommendations
    let _views = ctx
        .instance
        .enumerate_view_configuration_views(ctx.system, view_type)
        .unwrap();

    let stage = session
        .create_reference_space(xr::ReferenceSpaceType::STAGE, xr::Posef::IDENTITY)
        .unwrap();

    let mut vibration_event_reader = ManualEventReader::default();

    let mut event_storage = xr::EventDataBuffer::new();

    let mut swapchain = None;
    let mut running = false;

    let left_id = Uuid::new_v4();
    let right_id = Uuid::new_v4();
    XrPawn::spawn(app.world.spawn_empty(), left_id, right_id);

    let mut frame_count = 0usize;
    'session_loop: loop {
        frame_count += 1;
        while let Some(event) = ctx.instance.poll_event(&mut event_storage).unwrap() {
            match event {
                xr::Event::EventsLost(e) => {
                    bevy_log::error!("OpenXR: Lost {} events", e.lost_event_count());
                }
                xr::Event::InstanceLossPending(_) => {
                    bevy_log::info!("OpenXR: Shutting down for runtime request");
                    break 'session_loop;
                }
                xr::Event::SessionStateChanged(e) => {
                    bevy_log::debug!("entered state {:?}", e.state());

                    match e.state() {
                        xr::SessionState::UNKNOWN | xr::SessionState::IDLE => (),
                        xr::SessionState::READY => {
                            session.begin(view_type).unwrap();
                            running = true;
                        }
                        xr::SessionState::SYNCHRONIZED => {
                            app.world.insert_resource(XrVisibilityState::Hidden)
                        }
                        xr::SessionState::VISIBLE => app
                            .world
                            .insert_resource(XrVisibilityState::VisibleUnfocused),
                        xr::SessionState::FOCUSED => {
                            app.world.insert_resource(XrVisibilityState::VisibleFocused)
                        }
                        xr::SessionState::STOPPING => {
                            session.end().unwrap();
                            running = false;
                        }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                            println!("Exiting | Loss Pending");
                            break 'session_loop;
                        }
                        _ => unreachable!(),
                    }
                }
                xr::Event::ReferenceSpaceChangePending(e) => {
                    let reference_ref = &mut tracking_context.reference.write();

                    reference_ref.space_type = e.reference_space_type();
                    reference_ref.change_time = e.change_time();
                    reference_ref.previous_pose_offset =
                        openxr_pose_to_rigid_transform(e.pose_in_previous_space())
                }
                xr::Event::PerfSettingsEXT(e) => {
                    let sub_domain = match e.sub_domain() {
                        xr::PerfSettingsSubDomainEXT::COMPOSITING => "compositing",
                        xr::PerfSettingsSubDomainEXT::RENDERING => "rendering",
                        xr::PerfSettingsSubDomainEXT::THERMAL => "thermal",
                        _ => unreachable!(),
                    };
                    let domain = match e.domain() {
                        xr::PerfSettingsDomainEXT::CPU => "CPU",
                        xr::PerfSettingsDomainEXT::GPU => "GPU",
                        _ => unreachable!(),
                    };
                    let from = match e.from_level() {
                        xr::PerfSettingsNotificationLevelEXT::NORMAL => "normal",
                        xr::PerfSettingsNotificationLevelEXT::WARNING => "warning",
                        xr::PerfSettingsNotificationLevelEXT::IMPAIRED => "critical",
                        _ => unreachable!(),
                    };
                    let to = match e.to_level() {
                        xr::PerfSettingsNotificationLevelEXT::NORMAL => "normal",
                        xr::PerfSettingsNotificationLevelEXT::WARNING => "warning",
                        xr::PerfSettingsNotificationLevelEXT::IMPAIRED => "critical",
                        _ => unreachable!(),
                    };
                    bevy_log::warn!(
                        "OpenXR: The {} state of the {} went from {} to {}",
                        sub_domain,
                        domain,
                        from,
                        to
                    );

                    // todo: react to performance notifications
                }
                xr::Event::VisibilityMaskChangedKHR(_) => (), // todo: update visibility mask
                xr::Event::InteractionProfileChanged(_) => {
                    let left_hand = ctx
                        .instance
                        .path_to_string(
                            session
                                .current_interaction_profile(
                                    ctx.instance.string_to_path("/user/hand/left").unwrap(),
                                )
                                .unwrap(),
                        )
                        .ok();
                    let right_hand = ctx
                        .instance
                        .path_to_string(
                            session
                                .current_interaction_profile(
                                    ctx.instance.string_to_path("/user/hand/right").unwrap(),
                                )
                                .unwrap(),
                        )
                        .ok();

                    app.world.insert_resource(XrProfiles {
                        left_hand,
                        right_hand,
                    })
                }
                xr::Event::MainSessionVisibilityChangedEXTX(_) => (), // unused
                xr::Event::DisplayRefreshRateChangedFB(evt) => {
                    //  BUG: on oculus quest2 this will fire even when a requested refresh rate fails
                    bevy_log::info!(
                        "refresh rate changed: {} -> {}",
                        evt.from_display_refresh_rate(),
                        evt.to_display_refresh_rate()
                    );
                }
                _ => bevy_log::debug!("OpenXR: Unhandled event"),
            }
        }

        if !running {
            thread::sleep(Duration::from_millis(200));
            continue;
        }

        if frame_count % 1000 == 0 {
            utils::increase_refresh_rate(&ctx.instance, session.as_raw());
        }

        let frame_state = frame_waiter.wait().unwrap();
        session
            .sync_actions(&[(ActiveActionSet::new(&interaction_context.action_set.lock()))])
            .unwrap();

        frame_stream.begin().unwrap();

        if !frame_state.should_render {
            frame_stream
                .end(frame_state.predicted_display_time, blend_mode, &[])
                .unwrap();
            continue;
        }

        //  TODO: override bevy time with predicted frame time?
        *next_vsync_time.write() = frame_state.predicted_display_time;

        {
            let _world_cell = app.world.cell();
            handle_input(
                &interaction_context,
                &session,
                &mut _world_cell.get_resource_mut::<XrActionSet>().unwrap(),
            );
        }

        let (view_state_flags, views) = session
            .locate_views(view_type, frame_state.predicted_display_time, &stage)
            .unwrap();

        let view_cfgs = session
            .instance()
            .enumerate_view_configuration_views(ctx.system, view_type)
            .unwrap();

        // let resolutions: [vk::Extent2D; 2] = view_cfgs.iter().map();
        let resolutions: &Vec<vk::Extent2D> = &view_cfgs
            .iter()
            .map(|view_cfg| vk::Extent2D {
                width: view_cfg.recommended_image_rect_width,
                height: view_cfg.recommended_image_rect_height,
            })
            .collect();
        let device = ctx.wgpu_device.clone();
        let swapchains = swapchain
            .get_or_insert_with(|| EyeSwapchains::new(&vk_session, resolutions, device).unwrap());

        let left_tex = swapchains.left.acquire_texture_view().unwrap();
        let right_tex = swapchains.right.acquire_texture_view().unwrap();

        let mut manual_texture_views = app.world.get_resource_mut::<ManualTextureViews>().unwrap();
        manual_texture_views.insert(left_id, (left_tex.into(), resolutions[0].bevy()));
        manual_texture_views.insert(right_id, (right_tex.into(), resolutions[1].bevy()));

        app.world.insert_resource(XrViews(views.clone()));

        app.update();

        swapchains.left.release().unwrap();
        swapchains.right.release().unwrap();

        if view_state_flags
            .contains(ViewStateFlags::POSITION_VALID | ViewStateFlags::ORIENTATION_VALID)
        {
            frame_stream
                .end(
                    frame_state.predicted_display_time,
                    blend_mode,
                    &[
                        &xr::CompositionLayerProjection::new().space(&stage).views(&[
                            xr::CompositionLayerProjectionView::new()
                                .pose(views[0].pose)
                                .fov(views[0].fov)
                                .sub_image(
                                    xr::SwapchainSubImage::new()
                                        .swapchain(&swapchains.left.handle)
                                        .image_rect(resolutions[0].xr()),
                                ),
                            xr::CompositionLayerProjectionView::new()
                                .pose(views[1].pose)
                                .fov(views[1].fov)
                                .sub_image(
                                    xr::SwapchainSubImage::new()
                                        .swapchain(&swapchains.right.handle)
                                        .image_rect(resolutions[1].xr()),
                                ),
                        ]),
                    ],
                )
                .unwrap()
        } else {
            frame_stream
                .end(frame_state.predicted_display_time, blend_mode, &[])
                .unwrap()
        }

        handle_output(
            &interaction_context,
            &session,
            &mut vibration_event_reader,
            &mut app
                .world
                .get_resource_mut::<Events<XrVibrationEvent>>()
                .unwrap(),
        );

        if app_exit_event_reader
            .iter(&app.world.get_resource_mut::<Events<AppExit>>().unwrap())
            .next_back()
            .is_some()
        {
            println!("app exit event");
            session.request_exit().unwrap();
        }
    }
    println!("runner loop done");
}
