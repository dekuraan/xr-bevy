use bevy_log::info;
use openxr::{sys, Instance, PerfSettingsDomainEXT, PerfSettingsLevelEXT, Session};

use crate::{OpenXrContext, OpenXrSession};

pub fn increase_refresh_rate(instance: &Instance, session: sys::Session) {
    instance.exts().fb_display_refresh_rate.map(|display_fps| {
        let mut fps = 0f32;
        unsafe { (display_fps.get_display_refresh_rate)(session, &mut fps) };
        bevy_log::info!("got refresh rate: {}", fps);
        let mut refresh_rates = [0f32; 10];
        let mut out_count = 0u32;
        let rates = unsafe {
            (display_fps.enumerate_display_refresh_rates)(
                session,
                refresh_rates.len() as u32,
                &mut out_count,
                refresh_rates.as_mut_ptr(),
            )
        };
        bevy_log::info!(
            "available refresh rates: {:?}",
            &refresh_rates[..out_count as usize]
        );
        if fps < 90. {
            let res = unsafe { (display_fps.request_display_refresh_rate)(session, 90.) };
            bevy_log::info!("requested refresh rate {}, result: {:?}", 90., res);
        }
    });
}

pub fn increase_performance_setting(instance: &Instance, session: sys::Session) {
    unsafe {
        instance.exts().ext_performance_settings.map(|perf| {
            let res = (perf.perf_settings_set_performance_level)(
                session,
                PerfSettingsDomainEXT::CPU,
                PerfSettingsLevelEXT::SUSTAINED_HIGH,
            );
            info!("cpu perf setting res: {:?}", res);
            let res = (perf.perf_settings_set_performance_level)(
                session,
                PerfSettingsDomainEXT::GPU,
                PerfSettingsLevelEXT::SUSTAINED_HIGH,
            );
            info!("gpu perf setting res: {:?}", res);
        });
    }
}
