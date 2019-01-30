pub mod components;
pub mod config;
pub mod logger;
pub mod prelude;
pub mod rendering;
pub mod resources;

use crate::{
    logger::{Logger, LoggerInitError},
    rendering::Renderer,
    resources::{ResourceBuilder, ResourcesData},
};
use clap::{App, Arg};
use failure::Fail;
use log::info;
use rayon::{ThreadPoolBuildError, ThreadPoolBuilder};
use specs::{Dispatcher, DispatcherBuilder, World};
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

pub struct Running(pub bool);

#[derive(Debug, Fail)]
pub enum GameInitError {
    #[fail(display = "Failed to init logger: {}", err)]
    LoggerInit { err: LoggerInitError },
    #[fail(display = "Failed to create window: {}", err)]
    WindowCreation { err: CreationError },
    #[fail(display = "Failed to create threadpool: {}", err)]
    ThreadPoolCreation { err: ThreadPoolBuildError },
}

pub struct Game<'a, 'b> {
    pub world: World,
    pub dispatcher: Dispatcher<'a, 'b>,
    pub events_loop: EventsLoop,
}

impl<'a, 'b> Game<'a, 'b> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<RB, WB, DB, WA>(
        version: &str,
        world_access: WA,
        dispatcher_builder: DB,
        resources: RB,
        window_builder: WB,
    ) -> Result<Self, GameInitError>
    where
        WA: FnOnce(&mut World),
        DB: FnOnce(DispatcherBuilder<'a, 'b>) -> DispatcherBuilder<'a, 'b>,
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

        let mut world = World::new();

        // register components
        components::register(&mut world);
        world_access(&mut world);

        // Resources
        let thread_pool = Arc::new(
            ThreadPoolBuilder::new()
                .build()
                .map_err(|err| GameInitError::ThreadPoolCreation { err })?,
        );

        // Resources
        let resources = resources(ResourceBuilder {
            res: Arc::new(RwLock::new(ResourcesData::new())),
            is_dev: clap.is_present("dev"),
        });

        world.add_resource(clap);
        world.add_resource(thread_pool.clone());
        world.add_resource(resources.res);
        world.add_resource(Running(true));

        // Renderer
        let events_loop = EventsLoop::new();
        let window = window_builder(WindowBuilder::new())
            .build(&events_loop)
            .map_err(|err| GameInitError::WindowCreation { err })?;

        let renderer = Renderer::new(window);

        // Dispatcher
        let dispatcher = dispatcher_builder(DispatcherBuilder::new().with_pool(thread_pool).with(
            renderer,
            "renderer",
            &[],
        ))
        .build();

        info!("Game initialized");

        Ok(Game {
            world,
            dispatcher,
            events_loop,
        })
    }

    pub fn run(&mut self) {
        while self.world.read_resource::<Running>().0 {
            // handle events

            self.dispatcher.dispatch(&mut self.world.res);
            self.world.maintain();
        }

        info!("Exiting...");
    }
}
