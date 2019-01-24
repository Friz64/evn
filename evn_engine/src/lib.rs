pub mod config;
pub mod logger;
pub mod prelude;
pub mod rendering;
pub mod resources;

use crate::{
    logger::{Logger, LoggerInitError},
    rendering::Renderer,
    resources::{ResourceBuilder, Resources},
};
use clap::{App, Arg};
use failure::Fail;
use log::info;
use rayon::{ThreadPoolBuildError, ThreadPoolBuilder};
use specs::World;
use std::sync::{Arc, RwLock};
use winit::{CreationError, EventsLoop, WindowBuilder};

#[macro_export]
macro_rules! include_resource {
    (open: $file:expr) => {
        include_bytes!(concat!("../../resources/open/", $file))
    };
    (closed: $file:expr) => {
        include_bytes!(concat!("../../resources/closed/", $file))
    };
}

#[derive(Debug, Fail)]
pub enum GameInitError {
    #[fail(display = "Failed to init logger: {}", err)]
    LoggerInit { err: LoggerInitError },
    #[fail(display = "Failed to create window: {}", err)]
    WindowCreation { err: CreationError },
    #[fail(display = "Failed to create threadpool: {}", err)]
    ThreadPoolCreation { err: ThreadPoolBuildError },
}

pub struct Game {
    pub world: Arc<RwLock<World>>,
    pub events_loop: EventsLoop,
}

impl Game {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<RB, WB>(version: &str, resources: RB, window: WB) -> Result<Self, GameInitError>
    where
        RB: FnOnce(ResourceBuilder) -> ResourceBuilder,
        WB: FnOnce(WindowBuilder) -> WindowBuilder,
    {
        let clap = App::new("evn")
            .version(version)
            .about("A hobby game with an selfmade engine written in Rust")
            .arg(Arg::with_name("dev").long("dev").help("Development mode"))
            .arg(
                Arg::with_name("no-color")
                    .long("no-color")
                    .short("c")
                    .help("Don't color the console log"),
            )
            .get_matches();

        Logger::init(!clap.is_present("no-color"))
            .map_err(|err| GameInitError::LoggerInit { err })?;

        let thread_pool = ThreadPoolBuilder::new()
            .build()
            .map_err(|err| GameInitError::ThreadPoolCreation { err })?;

        let events_loop = EventsLoop::new();

        let window = window(WindowBuilder::new())
            .build(&events_loop)
            .map_err(|err| GameInitError::WindowCreation { err })?;

        let renderer = Renderer::new(window);

        let mut world = World::new();

        world.add_resource(renderer);
        world.add_resource(clap);
        world.add_resource(thread_pool);
        world.add_resource(Resources::new());

        let world = Arc::new(RwLock::new(world));

        resources(ResourceBuilder {
            world: world.clone(),
        });

        info!("Game initialized");

        Ok(Game { world, events_loop })
    }

    pub fn run(&self) {
        // DEBUGGING
        for _ in 0..=100 {
            {
                let (shaders, config) = {
                    let read_world = self.world.read().unwrap();
                    let res = read_world.read_resource::<Resources>();
                    (
                        (*res).get_resource("shader_normal").is_loaded(),
                        (*res).get_resource("config").is_loaded(),
                    )
                };

                info!("shaders loaded: {} - config loaded: {}", shaders, config);
            }
        }

        info!("Exiting...");
    }
}
