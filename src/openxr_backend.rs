use crate::{AppInfo, MainLoop, Platform, PlatformEvent, Core, SharedCore};
use anyhow::{Result, ensure, bail};
use openxr as xr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

pub type SharedXrCore = Arc<XrCore>;

/// A container for several commonly-used OpenXR constants.
pub struct XrCore {
    pub instance: xr::Instance,
    pub session: xr::Session<xr::Vulkan>,
    pub system: xr::SystemId,
}

/// Launch an `App` using OpenXR as a surface and input mechanism for VR
pub fn launch<M: MainLoop>(info: AppInfo) -> Result<()> {
    // Handle interrupts gracefully
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::Relaxed);
    })
    .expect("setting Ctrl-C handler");

    let (core, xr_core) = todo!();
    let mut app = M::new(&core, Platform::OpenXr {
        xr_core: &mut xr_core,
    })?;

    let mut event_storage = xr::EventDataBuffer::new();
    let mut session_running = false;

    // TODO: STATE TRANSITIONS
    'main_loop: loop {
        if !running.load(Ordering::Relaxed) {
            println!("Requesting exit");
            let res = xr_core.session.request_exit();
            if let Err(xr::sys::Result::ERROR_SESSION_NOT_RUNNING) = res {
                println!("OpenXR Exiting gracefully");
                break Ok(());
            }
            res?;
        }

        while let Some(event) = xr_core.instance.poll_event(&mut event_storage).unwrap() {
            use xr::Event::*;
            match event {
                SessionStateChanged(e) => {
                    println!("OpenXR entered state {:?}", e.state());
                    match e.state() {
                        xr::SessionState::READY => {
                            xr_core
                                .session
                                .begin(xr::ViewConfigurationType::PRIMARY_STEREO)
                                .unwrap();
                            session_running = true;
                        }
                        xr::SessionState::STOPPING => {
                            xr_core.session.end().unwrap();
                            session_running = false;
                        }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                            println!("OpenXR Exiting");
                            break 'main_loop Ok(());
                        }
                        _ => {}
                    }
                }
                InstanceLossPending(_) => {
                    println!("OpenXR Pending instance loss");
                    break 'main_loop Ok(());
                }
                EventsLost(e) => {
                    println!("OpenXR lost {} events", e.lost_event_count());
                }
                _ => {}
            }
            app.event(PlatformEvent::OpenXr(
                &event
            ), &core, Platform::OpenXr {
                xr_core: &xr_core,
            })?;
        }

        if !session_running {
            // Don't grind up the CPU
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }

        let swapchain_index = todo!();
        app.frame(crate::Frame { swapchain_index }, &core, Platform::OpenXr {
            xr_core: &xr_core,
        })?;
    }
}
