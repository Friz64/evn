mod platform;

use crate::{
    logger::UnwrapOrLog,
    resources::{Resource, ResourceState, ResourcesData},
};
use ash::{
    extensions::{
        ext::DebugUtils,
        khr::{Surface, Swapchain},
    },
    prelude::VkResult,
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
    vk, vk_make_version, Device, Entry, Instance, InstanceError, LoadingError,
};
use either::Either;
use err_derive::Error;
use fnv::FnvBuildHasher;
use log::{error, info, warn};
use specs::System;
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    os::raw::c_void,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};
use winit::Window;

const VALIDATION_LAYERS: [&str; 1] = ["VK_LAYER_LUNARG_standard_validation"];
const INSTANCE_EXTENSIONS: [&str; 1] = ["VK_EXT_debug_utils"];
const DEVICE_EXTENSIONS: [&str; 1] = ["VK_KHR_swapchain"];
const MAX_FRAMES_IN_FLIGHT: usize = 2;

#[derive(Debug)]
pub struct Shader {
    pub vert: Vec<u32>,
    pub frag: Vec<u32>,
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
    #[error(display = "Failed to create Swapchain: {}", err)]
    SwapchainCreationError { err: Either<vk::Result, String> },
    #[error(display = "Failed to load Shader \"{}\": {}", name, err)]
    ShaderLoadingError {
        name: String,
        err: Either<vk::Result, String>,
    },
    #[error(display = "Failed to create Pipeline: {}", err)]
    PipelineCreationError { err: vk::Result },
    #[error(display = "Failed to create Framebuffer: {}", err)]
    FramebufferCreationError { err: vk::Result },
    #[error(display = "Failed to create Command buffer: {}", err)]
    CommandBufferError { err: vk::Result },
    #[error(display = "Failed to create Sync: {}", err)]
    SyncCreationError { err: vk::Result },
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
    graphics_family_index: u32,
    present_queue: vk::Queue,
    present_family_index: u32,
    swapchain_loader: Swapchain,
    swapchain: vk::SwapchainKHR,
    image_views: Vec<vk::ImageView>,
    shader_modules: Vec<(vk::ShaderModule, vk::ShaderModule)>,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
    swapchain_framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    current_frame: usize,
}

impl Renderer {
    pub fn new(
        window: Window,
        validation: bool,
        res: Arc<RwLock<ResourcesData>>,
        names: HashMap<String, Vec<String>, FnvBuildHasher>,
    ) -> Result<Self, RendererInitError> {
        unsafe {
            let entry = Entry::new().map_err(|err| match err {
                LoadingError::LibraryLoadError(err) => RendererInitError::LoadingError { err },
            })?;

            let mut instance_layer_names = Vec::new();
            let mut instance_extension_names = platform::extension_names();

            if validation {
                for layer in VALIDATION_LAYERS.iter() {
                    instance_layer_names.push(string_pointer(layer));
                }

                for extension in INSTANCE_EXTENSIONS.iter() {
                    instance_extension_names.push(string_pointer(extension));
                }
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

            let debug = if validation {
                let debug_loader = DebugUtils::new(&entry, &instance);

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

                let debug_messenger = debug_loader
                    .create_debug_utils_messenger(&debug_create_info, None)
                    .map_err(|err| RendererInitError::DebugCallbackError { err })?;

                Some((debug_loader, debug_messenger))
            } else {
                None
            };

            let surface = platform::create_surface(&entry, &instance, &window)
                .map_err(|err| RendererInitError::SurfaceCreationError { err })?;
            let surface_loader = Surface::new(&entry, &instance);

            // maybe provide gui for device selection down the line?
            let (
                physical_device,
                graphics_family_index,
                present_family_index,
                surface_capabilites,
                surface_present_modes,
                surface_formats,
            ) = instance
                .enumerate_physical_devices()
                .map_err(|err| RendererInitError::PhysicalDeviceError {
                    err: Either::Left(err),
                })?
                .into_iter()
                .filter_map(|physical_device| {
                    let device_extension_properties =
                        instance.enumerate_device_extension_properties(physical_device);
                    let device_extension_properties = match device_extension_properties {
                        Ok(device_extension_properties) => device_extension_properties
                            .iter()
                            .map(|property| {
                                CStr::from_ptr(&property.extension_name as *const i8)
                                    .to_string_lossy()
                                    .into_owned()
                            })
                            .collect::<Vec<_>>(),
                        Err(err) => return Some(Err(err)),
                    };

                    if !DEVICE_EXTENSIONS.iter().all(|extension| {
                        device_extension_properties.contains(&extension.to_string())
                    }) {
                        return None;
                    };

                    let (surface_capabilites, surface_present_modes, surface_formats) =
                        match surface_information(&surface_loader, surface, physical_device) {
                            Ok(information) => information,
                            Err(err) => return Some(Err(err)),
                        };

                    if surface_formats.is_empty() || surface_present_modes.is_empty() {
                        return None;
                    }

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
                        (Some(graphics), Some(present)) => Some(Ok((
                            physical_device,
                            graphics,
                            present,
                            surface_capabilites,
                            surface_present_modes,
                            surface_formats,
                        ))),
                        _ => None,
                    }
                })
                .nth(0)
                .ok_or(RendererInitError::PhysicalDeviceError {
                    err: Either::Right("Failed to find suitable device".into()),
                })?
                .map_err(|err| RendererInitError::PhysicalDeviceError {
                    err: Either::Left(err),
                })?;

            let device_features = vk::PhysicalDeviceFeatures::builder();

            let mut device_layer_names = Vec::new();
            let mut device_extension_names = Vec::new();
            if validation {
                for layer in VALIDATION_LAYERS.iter() {
                    device_layer_names.push(string_pointer(layer));
                }
            }

            for extension in DEVICE_EXTENSIONS.iter() {
                device_extension_names.push(string_pointer(extension));
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

            let shader_modules = names["shaders"]
                .iter()
                .map(|shader_name| {
                    // wait until loaded
                    while res.read().unwrap().get_resource(shader_name).is_loading() {
                        thread::sleep(Duration::from_millis(10));
                    }

                    let resource = res.read().unwrap().get_resource(shader_name);
                    match *resource {
                        ResourceState::Loaded(ref resource) => match resource {
                            Resource::Shader(Shader { vert, frag }) => {
                                let vertex_shader_module_create_info =
                                    vk::ShaderModuleCreateInfo::builder().code(&vert);
                                let vertex_shader_module = device
                                    .create_shader_module(&vertex_shader_module_create_info, None)
                                    .map_err(|err| RendererInitError::ShaderLoadingError {
                                        name: shader_name.clone(),
                                        err: Either::Left(err),
                                    })?;

                                let fragment_shader_module_create_info =
                                    vk::ShaderModuleCreateInfo::builder().code(&frag);
                                let fragment_shader_module = device
                                    .create_shader_module(&fragment_shader_module_create_info, None)
                                    .map_err(|err| RendererInitError::ShaderLoadingError {
                                        name: shader_name.clone(),
                                        err: Either::Left(err),
                                    })?;

                                Ok((vertex_shader_module, fragment_shader_module))
                            }
                            _ => panic!("Non shader resource in shader List"),
                        },
                        _ => Err(RendererInitError::ShaderLoadingError {
                            name: shader_name.clone(),
                            err: Either::Right("Failed shader requested".into()),
                        }),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;

            let swapchain_loader = Swapchain::new(&instance, &device);

            let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
                .queue_family_index(graphics_family_index)
                .flags(vk::CommandPoolCreateFlags::empty());

            let command_pool = device
                .create_command_pool(&command_pool_create_info, None)
                .map_err(|err| RendererInitError::CommandBufferError { err })?;

            let (
                swapchain,
                image_views,
                pipeline_layout,
                render_pass,
                pipeline,
                swapchain_framebuffers,
                command_buffers,
            ) = create_swapchain(
                surface_formats,
                surface_present_modes,
                surface_capabilites,
                surface,
                &window,
                graphics_family_index,
                present_family_index,
                &device,
                &swapchain_loader,
                &shader_modules,
                command_pool,
            )?;

            let semaphore_create_info = vk::SemaphoreCreateInfo::builder();
            let fence_create_info =
                vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
            let sync = (0..MAX_FRAMES_IN_FLIGHT)
                .map(|_| {
                    let image_available_semaphore = device
                        .create_semaphore(&semaphore_create_info, None)
                        .map_err(|err| RendererInitError::SyncCreationError { err })?;
                    let render_finished_semaphore = device
                        .create_semaphore(&semaphore_create_info, None)
                        .map_err(|err| RendererInitError::SyncCreationError { err })?;
                    let in_flight_fence = device
                        .create_fence(&fence_create_info, None)
                        .map_err(|err| RendererInitError::SyncCreationError { err })?;

                    Ok((
                        image_available_semaphore,
                        render_finished_semaphore,
                        in_flight_fence,
                    ))
                })
                .collect::<Result<Vec<_>, _>>()?;

            let mut image_available_semaphores = Vec::new();
            let mut render_finished_semaphores = Vec::new();
            let mut in_flight_fences = Vec::new();

            for (image_available_semaphore, render_finished_semaphore, in_flight_fence) in sync {
                image_available_semaphores.push(image_available_semaphore);
                render_finished_semaphores.push(render_finished_semaphore);
                in_flight_fences.push(in_flight_fence);
            }

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
                graphics_family_index,
                present_queue,
                present_family_index,
                swapchain_loader,
                swapchain,
                image_views,
                shader_modules,
                pipeline_layout,
                render_pass,
                pipeline,
                swapchain_framebuffers,
                command_pool,
                command_buffers,
                image_available_semaphores,
                render_finished_semaphores,
                in_flight_fences,
                current_frame: 0,
            })
        }
    }
}

impl<'a> System<'a> for Renderer {
    type SystemData = ();

    fn run(&mut self, (): Self::SystemData) {
        unsafe {
            self.device
                .wait_for_fences(
                    &[self.in_flight_fences[self.current_frame]],
                    true,
                    u64::max_value(),
                )
                .unwrap_or_log("Failed to wait for fences");

            let next_image = self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::max_value(),
                self.image_available_semaphores[self.current_frame],
                vk::Fence::null(),
            );
            let image_index = match next_image {
                Ok((image_index, _)) => image_index,
                Err(err) => match err {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => {
                        // recreate swapchain
                        self.device
                            .queue_wait_idle(self.present_queue)
                            .unwrap_or_log("Failed to wait on present queue");

                        let (surface_capabilites, surface_present_modes, surface_formats) =
                            surface_information(
                                &self.surface_loader,
                                self.surface,
                                self.physical_device,
                            )
                            .unwrap_or_log("Failed to get surface information");

                        cleanup_swapchain(
                            &self.device,
                            self.command_pool,
                            &self.command_buffers,
                            &self.swapchain_framebuffers,
                            self.pipeline,
                            self.pipeline_layout,
                            self.render_pass,
                            &self.image_views,
                            &self.swapchain_loader,
                            self.swapchain,
                        );

                        let (
                            swapchain,
                            image_views,
                            pipeline_layout,
                            render_pass,
                            pipeline,
                            swapchain_framebuffers,
                            command_buffers,
                        ) = create_swapchain(
                            surface_formats,
                            surface_present_modes,
                            surface_capabilites,
                            self.surface,
                            &self.window,
                            self.graphics_family_index,
                            self.present_family_index,
                            &self.device,
                            &self.swapchain_loader,
                            &self.shader_modules,
                            self.command_pool,
                        )
                        .unwrap_or_log("Failed to recreate swapchain");

                        self.image_views = image_views;
                        self.pipeline_layout = pipeline_layout;
                        self.render_pass = render_pass;
                        self.pipeline = pipeline;
                        self.swapchain_framebuffers = swapchain_framebuffers;
                        self.command_buffers = command_buffers;
                        self.swapchain = swapchain;

                        return;
                    }
                    _ => Err(err).unwrap_or_log("Failed to acquire next image"),
                },
            };

            self.device
                .reset_fences(&[self.in_flight_fences[self.current_frame]])
                .unwrap_or_log("Failed to reset fences");

            let wait_semaphores = [self.image_available_semaphores[self.current_frame]];
            let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .signal_semaphores(&signal_semaphores)
                .command_buffers(&[self.command_buffers[image_index as usize]])
                .build();

            self.device
                .queue_submit(
                    self.graphics_queue,
                    &[submit_info],
                    self.in_flight_fences[self.current_frame],
                )
                .unwrap_or_log("Failed to submit to queue");

            let swapchains = [self.swapchain];

            let present_info_image_indices = [image_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&signal_semaphores)
                .swapchains(&swapchains)
                .image_indices(&present_info_image_indices);

            self.swapchain_loader
                .queue_present(self.present_queue, &present_info)
                .unwrap_or_log("Failed to submit to present queue");

            self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.device
                .queue_wait_idle(self.present_queue)
                .unwrap_or_log("Failed to wait on present queue");

            for semaphore in &self.image_available_semaphores {
                self.device.destroy_semaphore(*semaphore, None);
            }

            for semaphore in &self.render_finished_semaphores {
                self.device.destroy_semaphore(*semaphore, None);
            }

            for fence in &self.in_flight_fences {
                self.device.destroy_fence(*fence, None);
            }

            cleanup_swapchain(
                &self.device,
                self.command_pool,
                &self.command_buffers,
                &self.swapchain_framebuffers,
                self.pipeline,
                self.pipeline_layout,
                self.render_pass,
                &self.image_views,
                &self.swapchain_loader,
                self.swapchain,
            );

            self.device.destroy_command_pool(self.command_pool, None);

            for shader_module in &self.shader_modules {
                self.device.destroy_shader_module(shader_module.0, None); // vertex
                self.device.destroy_shader_module(shader_module.1, None); // fragment
            }

            self.surface_loader.destroy_surface(self.surface, None);

            self.device.destroy_device(None);

            if let Some((debug_loader, debug)) = &self.debug {
                debug_loader.destroy_debug_utils_messenger(*debug, None);
            }

            self.instance.destroy_instance(None);
        }
    }
}

unsafe fn cleanup_swapchain(
    device: &Device,
    command_pool: vk::CommandPool,
    command_buffers: &[vk::CommandBuffer],
    swapchain_framebuffers: &Vec<vk::Framebuffer>,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    image_views: &Vec<vk::ImageView>,
    swapchain_loader: &Swapchain,
    swapchain: vk::SwapchainKHR,
) {
    device.free_command_buffers(command_pool, command_buffers);

    for framebuffer in swapchain_framebuffers {
        device.destroy_framebuffer(*framebuffer, None);
    }

    device.destroy_pipeline(pipeline, None);

    device.destroy_pipeline_layout(pipeline_layout, None);

    device.destroy_render_pass(render_pass, None);

    for image_view in image_views {
        device.destroy_image_view(*image_view, None);
    }

    swapchain_loader.destroy_swapchain(swapchain, None);
}

fn string_pointer(string: &str) -> *const i8 {
    CString::new(string).unwrap().into_raw() as *const i8
}

unsafe fn surface_information(
    surface_loader: &Surface,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
) -> VkResult<(
    vk::SurfaceCapabilitiesKHR,
    Vec<vk::PresentModeKHR>,
    Vec<vk::SurfaceFormatKHR>,
)> {
    let surface_capabilites =
        surface_loader.get_physical_device_surface_capabilities(physical_device, surface)?;

    let surface_present_modes =
        surface_loader.get_physical_device_surface_present_modes(physical_device, surface)?;

    let surface_formats =
        surface_loader.get_physical_device_surface_formats(physical_device, surface)?;

    Ok((surface_capabilites, surface_present_modes, surface_formats))
}

fn choose_swap_surface_format(
    available_formats: Vec<vk::SurfaceFormatKHR>,
) -> vk::SurfaceFormatKHR {
    if available_formats.len() == 1 && available_formats[0].format == vk::Format::UNDEFINED {
        return vk::SurfaceFormatKHR::builder()
            .format(vk::Format::B8G8R8A8_UNORM)
            .color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .build();
    }

    for available_format in &available_formats {
        if available_format.format == vk::Format::B8G8R8A8_UNORM
            && available_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            return available_format.clone();
        }
    }

    available_formats[0]
}

fn choose_swap_present_mode(
    available_present_modes: Vec<vk::PresentModeKHR>,
) -> vk::PresentModeKHR {
    let mut best_mode = vk::PresentModeKHR::FIFO;

    for available_present_mode in available_present_modes {
        if available_present_mode == vk::PresentModeKHR::MAILBOX {
            return available_present_mode;
        } else if available_present_mode == vk::PresentModeKHR::IMMEDIATE {
            best_mode = available_present_mode;
        }
    }

    best_mode
}

fn choose_swap_extent(
    window_size: winit::dpi::LogicalSize,
    capabilites: vk::SurfaceCapabilitiesKHR,
) -> vk::Extent2D {
    if capabilites.current_extent.width != u32::max_value() {
        capabilites.current_extent
    } else {
        let (width, height) = window_size.into();

        vk::Extent2D {
            width: capabilites
                .min_image_extent
                .width
                .max(capabilites.max_image_extent.width.min(width)),
            height: capabilites
                .min_image_extent
                .height
                .max(capabilites.max_image_extent.height.min(height)),
        }
    }
}

unsafe fn create_swapchain(
    surface_formats: Vec<vk::SurfaceFormatKHR>,
    surface_present_modes: Vec<vk::PresentModeKHR>,
    surface_capabilites: vk::SurfaceCapabilitiesKHR,
    surface: vk::SurfaceKHR,
    window: &Window,
    graphics_family_index: u32,
    present_family_index: u32,
    device: &Device,
    swapchain_loader: &Swapchain,
    shader_modules: &Vec<(vk::ShaderModule, vk::ShaderModule)>,
    command_pool: vk::CommandPool,
) -> Result<
    (
        vk::SwapchainKHR,
        Vec<vk::ImageView>,
        vk::PipelineLayout,
        vk::RenderPass,
        vk::Pipeline,
        Vec<vk::Framebuffer>,
        Vec<vk::CommandBuffer>,
    ),
    RendererInitError,
> {
    let p_name = string_pointer("main");
    let mut stage_create_infos = Vec::new();
    for (vertex_shader_module, fragment_shader_module) in shader_modules {
        let vertex_shader_stage_create_info = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::VERTEX,
            module: *vertex_shader_module,
            p_name,
            ..Default::default()
        };

        stage_create_infos.push(vertex_shader_stage_create_info);

        let fragment_shader_stage_create_info = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::FRAGMENT,
            module: *fragment_shader_module,
            p_name,
            ..Default::default()
        };

        stage_create_infos.push(fragment_shader_stage_create_info);
    }

    let surface_format = choose_swap_surface_format(surface_formats);
    let present_mode = choose_swap_present_mode(surface_present_modes);
    let extent = choose_swap_extent(
        window
            .get_inner_size()
            .ok_or(RendererInitError::SwapchainCreationError {
                err: Either::Right("Failed to get window size".into()),
            })?,
        surface_capabilites,
    );

    let mut image_count = surface_capabilites.min_image_count + 1;
    if surface_capabilites.max_image_count > 0 && image_count > surface_capabilites.max_image_count
    {
        image_count = surface_capabilites.max_image_count;
    }

    let mut swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .pre_transform(surface_capabilites.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true);

    let swapchain_queue_family_indices = [graphics_family_index, present_family_index];
    if graphics_family_index != present_family_index {
        swapchain_create_info = swapchain_create_info
            .image_sharing_mode(vk::SharingMode::CONCURRENT)
            .queue_family_indices(&swapchain_queue_family_indices)
    } else {
        swapchain_create_info = swapchain_create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE)
    };

    let swapchain = swapchain_loader
        .create_swapchain(&swapchain_create_info, None)
        .map_err(|err| RendererInitError::SwapchainCreationError {
            err: Either::Left(err),
        })?;

    let images = swapchain_loader
        .get_swapchain_images(swapchain)
        .map_err(|err| RendererInitError::SwapchainCreationError {
            err: Either::Left(err),
        })?;

    let image_views = images
        .into_iter()
        .map(|image| {
            let view_create_info = vk::ImageViewCreateInfo::builder()
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image(image);

            device.create_image_view(&view_create_info, None)
        })
        .collect::<VkResult<Vec<_>>>()
        .map_err(|err| RendererInitError::SwapchainCreationError {
            err: Either::Left(err),
        })?;

    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder();

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(extent.width as f32)
        .height(extent.width as f32)
        .min_depth(0.0)
        .max_depth(1.0)
        .build();

    let scissor = vk::Rect2D::builder()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(extent)
        .build();

    let viewport_state_viewports = [viewport];
    let viewport_state_scissors = [scissor];
    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(&viewport_state_viewports)
        .scissors(&viewport_state_scissors);

    let rasterizer = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::CLOCKWISE)
        .depth_bias_enable(false);

    let multisampling = vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(
            vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
        )
        .blend_enable(false)
        .build();

    let color_blending_attachments = [color_blend_attachment];
    let color_blending = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .attachments(&color_blending_attachments);

    let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder();

    let pipeline_layout = device
        .create_pipeline_layout(&pipeline_layout_create_info, None)
        .map_err(|err| RendererInitError::PipelineCreationError { err })?;

    let color_attachment = vk::AttachmentDescription::builder()
        .format(surface_format.format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
        .build();

    let color_attachment_ref = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .build();

    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&[color_attachment_ref])
        .build();

    let dependencies = [vk::SubpassDependency {
        src_subpass: vk::SUBPASS_EXTERNAL,
        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
            | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        ..Default::default()
    }];

    let render_pass_info_attachments = [color_attachment];
    let render_pass_info_subpasses = [subpass];
    let render_pass_info = vk::RenderPassCreateInfo::builder()
        .attachments(&render_pass_info_attachments)
        .dependencies(&dependencies)
        .subpasses(&render_pass_info_subpasses);

    let render_pass = device
        .create_render_pass(&render_pass_info, None)
        .map_err(|err| RendererInitError::PipelineCreationError { err })?;

    let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&stage_create_infos)
        .vertex_input_state(&vertex_input_info)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterizer)
        .multisample_state(&multisampling)
        .color_blend_state(&color_blending)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0)
        .build();

    // just create one pipeline for now because we only have one shader
    let pipeline = device
        .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_create_info], None)
        .map_err(|err| RendererInitError::PipelineCreationError { err: err.1 })?[0];

    let swapchain_framebuffers = image_views
        .iter()
        .map(|&image_view| {
            let attachments = [image_view];

            let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(extent.width)
                .height(extent.height)
                .layers(1);

            device
                .create_framebuffer(&framebuffer_create_info, None)
                .map_err(|err| RendererInitError::FramebufferCreationError { err })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(swapchain_framebuffers.len() as u32);

    let command_buffers = device
        .allocate_command_buffers(&command_buffer_alloc_info)
        .map_err(|err| RendererInitError::CommandBufferError { err })?;

    for (i, &command_buffer) in command_buffers.iter().enumerate() {
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE);

        device
            .begin_command_buffer(command_buffer, &begin_info)
            .map_err(|err| RendererInitError::CommandBufferError { err })?;

        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];

        let render_pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(render_pass)
            .framebuffer(swapchain_framebuffers[i])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: extent,
            })
            .clear_values(&clear_values);

        device.cmd_begin_render_pass(
            command_buffer,
            &render_pass_info,
            vk::SubpassContents::INLINE,
        );

        device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);

        device.cmd_draw(command_buffer, 3, 1, 0, 0);

        device.cmd_end_render_pass(command_buffer);

        device
            .end_command_buffer(command_buffer)
            .map_err(|err| RendererInitError::CommandBufferError { err })?;
    }

    Ok((
        swapchain,
        image_views,
        pipeline_layout,
        render_pass,
        pipeline,
        swapchain_framebuffers,
        command_buffers,
    ))
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> u32 {
    let message = CStr::from_ptr((*callback_data).p_message).to_string_lossy();
    let message = message.trim();

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
