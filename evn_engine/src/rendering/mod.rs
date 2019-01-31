mod platform;
pub mod shaders;

use ash::{
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
    vk, vk_make_version, Device, Entry, Instance, InstanceError, LoadingError,
};
use err_derive::Error;
use specs::System;
use std::ffi::CString;
use winit::Window;

#[derive(Debug, Error)]
pub enum RendererInitError {
    #[error(display = "Failed to load Vulkan: {}", err)]
    LoadingError { err: String },
    #[error(display = "Failed to create Instance: {}", err)]
    InstanceError { err: String },
}

pub struct Renderer {
    window: Window,
    entry: Entry,
    instance: Instance,
}

impl Renderer {
    pub fn new(window: Window) -> Result<Self, RendererInitError> {
        unsafe {
            let entry = Entry::new().map_err(|err| match err {
                LoadingError::LibraryLoadError(err) => RendererInitError::LoadingError { err },
            })?;

            /* // Probably need to check for support first (Layer specified does not exist)
            let layer_names: Vec<*const c_char> = ["VK_LAYER_LUNARG_standard_validation"]
                .iter()
                .map(|name| CString::new(*name).unwrap().as_ptr())
                .collect();
            */

            let extension_names = platform::extension_names();

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
                //.enabled_layer_names(&layer_names)
                .enabled_extension_names(&extension_names);

            let instance = entry
                .create_instance(&instance_create_info, None)
                .map_err(|err| RendererInitError::InstanceError {
                    err: match err {
                        InstanceError::LoadError(err) => format!("{:?}", err),
                        InstanceError::VkError(result) => result.to_string(),
                    },
                })?;

            Ok(Renderer {
                window,
                entry,
                instance,
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
        // destory all vulkan things
    }
}
