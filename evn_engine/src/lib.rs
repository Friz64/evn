pub mod components;
pub mod config;
pub mod logger;
pub mod prelude;
pub mod rendering;
pub mod resources;
pub mod systems;

use crate::{
    logger::Logger,
    rendering::{Renderer, RendererInitError},
    resources::{ResourceBuilder, ResourcesData},
    systems::EventHandler,
};
use clap::{App, Arg};
use crossbeam::{channel, Sender};
use err_derive::Error;
use fnv::FnvHashMap;
use log::info;
use rayon::{ThreadPoolBuildError, ThreadPoolBuilder};
use specs::{Dispatcher, DispatcherBuilder, World};
use std::sync::{Arc, RwLock};
use winit::{CreationError, Event, EventsLoop, WindowBuilder};

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

#[derive(Debug, Error)]
pub enum GameInitError {
    #[error(display = "Failed to create window: {}", err)]
    WindowCreation { err: CreationError },
    #[error(display = "Failed to create threadpool: {}", err)]
    ThreadPoolCreation { err: ThreadPoolBuildError },
    #[error(display = "Failed to create renderer: {}", err)]
    RendererCreation { err: RendererInitError },
}

pub struct Game<'a, 'b> {
    pub world: World,
    pub dispatcher: Dispatcher<'a, 'b>,
    pub events_loop: EventsLoop,
    pub event_send: Sender<Event>,
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
            .arg(
                Arg::with_name("dev")
                    .long("dev")
                    .help("Enable Development mode"),
            )
            .arg(
                Arg::with_name("validation")
                    .long("validation")
                    .help("Enable Vulkan validation layers"),
            )
            .arg(
                Arg::with_name("color")
                    .long("color")
                    .short("c")
                    .help("Enable console coloring"),
            )
            .get_matches();

        let color = clap.is_present("color");
        let is_dev = clap.is_present("dev");
        let debug_callback = clap.is_present("validation");

        if let Err(err) = Logger::init(color) {
            eprintln!("Failed to init logger: {}", err);
        }

        let mut world = World::new();

        // register components
        components::register(&mut world);
        world_access(&mut world);

        // Thread Pool
        let thread_pool = Arc::new(
            ThreadPoolBuilder::new()
                .build()
                .map_err(|err| GameInitError::ThreadPoolCreation { err })?,
        );

        // event channel
        let (send, recv) = channel::unbounded();

        // Resources
        let resources = resources(ResourceBuilder {
            res: Arc::new(RwLock::new(ResourcesData::new())),
            is_dev,
            names: FnvHashMap::default(),
        });

        // Renderer
        let events_loop = EventsLoop::new();
        let window = window_builder(WindowBuilder::new())
            .build(&events_loop)
            .map_err(|err| GameInitError::WindowCreation { err })?;

        let renderer = Renderer::new(
            window,
            debug_callback,
            resources.res.clone(),
            resources.names,
        )
        .map_err(|err| GameInitError::RendererCreation { err })?;

        // Dispatcher
        let dispatcher =
            dispatcher_builder(DispatcherBuilder::new().with_pool(thread_pool.clone()))
                .with(EventHandler, "event_handler", &[])
                .with(renderer, "renderer", &["event_handler"])
                .build();

        world.add_resource(recv);
        world.add_resource(clap);
        world.add_resource(thread_pool);
        world.add_resource(resources.res);
        world.add_resource(Running(true));

        info!("Game initialized");

        Ok(Game {
            world,
            dispatcher,
            events_loop,
            event_send: send,
        })
    }

    pub fn run(&mut self) {
        let event_send = self.event_send.clone();
        while self.world.read_resource::<Running>().0 {
            self.events_loop.poll_events(|event| {
                event_send.send(event).unwrap();
            });

            self.dispatcher.dispatch(&mut self.world.res);

            self.world.maintain();
        }

        info!("Exiting...");
    }
}
