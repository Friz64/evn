mod platform;
pub mod shaders;

use ash::{
    extensions::{
        ext::DebugUtils,
        khr::{Surface, Swapchain},
    },
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
    vk, vk_make_version, Device, Entry, Instance, InstanceError, LoadingError,
};
use either::Either;
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
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info!("Vulkan: {}", message);
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
    #[error(display = "Failed to create Surface: {}", err)]
    SurfaceCreationError { err: vk::Result },
    #[error(display = "Physical Device Error: {}", err)]
    PhysicalDeviceError { err: Either<vk::Result, String> },
    #[error(display = "Failed to create Device: {}", err)]
    DeviceCreationError { err: vk::Result },
}

pub struct Renderer {
    #[allow(dead_code)]
    window: Window,
    #[allow(dead_code)]
    entry: Entry,
    instance: Instance,
    debug: Option<(DebugUtils, vk::DebugUtilsMessengerEXT)>,
    surface: vk::SurfaceKHR,
    surface_loader: Surface,
    physical_device: vk::PhysicalDevice,
    device: Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
}

impl Renderer {
    pub fn new(window: Window, debug_callback: bool) -> Result<Self, RendererInitError> {
        unsafe {
            let entry = Entry::new().map_err(|err| match err {
                LoadingError::LibraryLoadError(err) => RendererInitError::LoadingError { err },
            })?;

            let mut instance_layer_names = Vec::new();
            let mut instance_extension_names = platform::extension_names();

            if debug_callback {
                instance_layer_names.push(string_pointer("VK_LAYER_LUNARG_standard_validation"));

                instance_extension_names.push(string_pointer("VK_EXT_debug_utils"));
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
                .enabled_layer_names(&instance_layer_names)
                .enabled_extension_names(&instance_extension_names);

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
                            //| vk::DebugUtilsMessageSeverityFlagsEXT::INFO
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

            let surface = platform::create_surface(&entry, &instance, &window)
                .map_err(|err| RendererInitError::SurfaceCreationError { err })?;
            let surface_loader = Surface::new(&entry, &instance);

            // maybe provide gui for device selection down the line?
            let (physical_device, graphics_family_index, present_family_index) = instance
                .enumerate_physical_devices()
                .map_err(|err| RendererInitError::PhysicalDeviceError {
                    err: Either::Left(err),
                })?
                .into_iter()
                .filter_map(|physical_device| {
                    let queue_family_properties =
                        instance.get_physical_device_queue_family_properties(physical_device);

                    let graphics_family_index = queue_family_properties
                        .iter()
                        .enumerate()
                        .filter_map(|(index, properties)| {
                            let support = properties.queue_flags.contains(vk::QueueFlags::GRAPHICS);

                            if support {
                                Some(index as u32)
                            } else {
                                None
                            }
                        })
                        .nth(0);

                    let present_family_index = (0..queue_family_properties.len())
                        .filter_map(|index| {
                            let support = surface_loader.get_physical_device_surface_support(
                                physical_device,
                                index as u32,
                                surface,
                            );

                            if support {
                                Some(index as u32)
                            } else {
                                None
                            }
                        })
                        .nth(0);

                    match (graphics_family_index, present_family_index) {
                        (Some(graphics), Some(present)) => {
                            Some((physical_device, graphics, present))
                        }
                        _ => None,
                    }
                })
                .nth(0)
                .ok_or(RendererInitError::PhysicalDeviceError {
                    err: Either::Right("Failed to find suitable device".into()),
                })?;

            let device_extension_names = [Swapchain::name().as_ptr()];
            let mut device_layer_names = Vec::new();
            let device_features = vk::PhysicalDeviceFeatures::builder();

            if debug_callback {
                device_layer_names.push(string_pointer("VK_LAYER_LUNARG_standard_validation"));
            }

            let mut queue_family_indices = vec![graphics_family_index, present_family_index];
            // remove duplicates
            queue_family_indices.sort();
            queue_family_indices.dedup();
            let device_queue_create_info = queue_family_indices
                .into_iter()
                .map(|queue_family_index| {
                    vk::DeviceQueueCreateInfo::builder()
                        .queue_family_index(queue_family_index)
                        .queue_priorities(&[1.0])
                        .build()
                })
                .collect::<Vec<_>>();

            let device_create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(&device_queue_create_info)
                .enabled_extension_names(&device_extension_names)
                .enabled_layer_names(&device_layer_names)
                .enabled_features(&device_features);

            let device = instance
                .create_device(physical_device, &device_create_info, None)
                .map_err(|err| RendererInitError::DeviceCreationError { err })?;

            let graphics_queue = device.get_device_queue(graphics_family_index, 0);
            let present_queue = device.get_device_queue(present_family_index, 0);

            Ok(Renderer {
                window,
                entry,
                instance,
                debug,
                surface,
                surface_loader,
                physical_device,
                device,
                graphics_queue,
                present_queue,
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
            self.surface_loader.destroy_surface(self.surface, None);

            self.device.destroy_device(None);

            if let Some((debug_utils, debug)) = &self.debug {
                debug_utils.destroy_debug_utils_messenger(*debug, None);
            }

            self.instance.destroy_instance(None);
        }
    }
}
