mod platform;
pub mod shaders;

use ash::{
    extensions::ext::DebugUtils,
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
    vk, vk_make_version, Device, Entry, Instance, InstanceError, LoadingError,
};
use err_derive::Error;
use log::{error, info, warn};
use specs::System;
use std::{
    ffi::{CStr, CString},
    os::raw::c_void,
};
use winit::Window;

fn string_pointer(string: &str) -> *const i8 {
    CString::new(string).unwrap().into_raw() as *const i8
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> u32 {
    let message = CStr::from_ptr((*callback_data).p_message)
        .to_string_lossy()
        .chars()
        .filter(|c| *c != '\u{A}') // filter out newlines
        .collect::<String>();

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            info!("Vulkan: {}", message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!("Vulkan: {}", message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!("Vulkan: {}", message);
        }
        _ => (),
    };

    vk::FALSE
}

#[derive(Debug, Error)]
pub enum RendererInitError {
    #[error(display = "Failed to load Vulkan: {}", err)]
    LoadingError { err: String },
    #[error(display = "Failed to create Instance: {}", err)]
    InstanceError { err: String },
    #[error(display = "Failed to create Debug Callback: {}", err)]
    DebugCallbackError { err: vk::Result },
}

pub struct Renderer {
    #[allow(dead_code)]
    window: Window,
    #[allow(dead_code)]
    entry: Entry,
    instance: Instance,
    debug: Option<(DebugUtils, vk::DebugUtilsMessengerEXT)>,
}

impl Renderer {
    pub fn new(window: Window, debug_callback: bool) -> Result<Self, RendererInitError> {
        unsafe {
            let entry = Entry::new().map_err(|err| match err {
                LoadingError::LibraryLoadError(err) => RendererInitError::LoadingError { err },
            })?;

            let mut layer_names = Vec::new();
            let mut extension_names = platform::extension_names();

            if debug_callback {
                layer_names.push(string_pointer("VK_LAYER_LUNARG_standard_validation"));

                extension_names.push(string_pointer("VK_EXT_debug_utils"));
            }

            let application_name = CString::new("evn").unwrap();
            let engine_name = CString::new("evn_engine").unwrap();
            let app_info = vk::ApplicationInfo::builder()
                .application_name(&application_name)
                .application_version(0)
                .engine_name(&engine_name)
                .engine_version(0)
                .api_version(vk_make_version!(1, 0, 36));

            let instance_create_info = vk::InstanceCreateInfo::builder()
                .application_info(&app_info)
                .enabled_layer_names(&layer_names)
                .enabled_extension_names(&extension_names);

            let instance = entry
                .create_instance(&instance_create_info, None)
                .map_err(|err| RendererInitError::InstanceError {
                    err: match err {
                        InstanceError::LoadError(err) => format!("{:?}", err),
                        InstanceError::VkError(result) => result.to_string(),
                    },
                })?;

            let debug = if debug_callback {
                let debug_utils = DebugUtils::new(&entry, &instance);

                let debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                    .message_severity(
                        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                            | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                            | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                    )
                    .message_type(
                        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                    )
                    .pfn_user_callback(Some(vulkan_debug_callback))
                    .build();

                let debug_messenger = debug_utils
                    .create_debug_utils_messenger(&debug_create_info, None)
                    .map_err(|err| RendererInitError::DebugCallbackError { err })?;

                Some((debug_utils, debug_messenger))
            } else {
                None
            };

            Ok(Renderer {
                window,
                entry,
                instance,
                debug,
            })
        }
    }
}

impl<'a> System<'a> for Renderer {
    type SystemData = ();

    fn run(&mut self, (): Self::SystemData) {}
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            if let Some((debug_utils, debug)) = &self.debug {
                debug_utils.destroy_debug_utils_messenger(*debug, None);
            }

            self.instance.destroy_instance(None);
        }
    }
}
