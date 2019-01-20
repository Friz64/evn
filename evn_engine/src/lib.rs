pub mod config;
pub mod logger;
pub mod prelude;
pub mod rendering;

use crate::{
    config::{Config, ConfigError, ConfigMap},
    logger::{Logger, LoggerInitError, UnwrapOrLog},
    rendering::{shaders::ShaderMap, Renderer},
};
use clap::{App, Arg, ArgMatches};
use hashbrown::HashMap;
use log::info;
use specs::World;
use std::path::{Path, PathBuf};
use winit::{EventsLoop, WindowBuilder};

#[macro_export]
macro_rules! include_resource {
    (open: $file:expr) => {
        include_bytes!(concat!("../../resources/open/", $file))
    };
    (closed: $file:expr) => {
        include_bytes!(concat!("../../resources/closed/", $file))
    };
}

pub struct Game {
    pub world: World,
    pub events_loop: EventsLoop,
}

impl Game {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<RB, WB>(version: &str, resources: RB, window: WB) -> Result<Self, LoggerInitError>
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

        Logger::init(!clap.is_present("no-color"))?;

        let events_loop = EventsLoop::new();

        let renderer = Renderer::new(
            window(WindowBuilder::new())
                .build(&events_loop)
                .unwrap_or_log("Window Creation"),
        );

        let mut world = World::new();
        world.add_resource(renderer);
        world.add_resource(clap);
        world.add_resource(ConfigMap(HashMap::new()));
        world.add_resource(ShaderMap(HashMap::new()));

        resources(ResourceBuilder { world: &world });

        Ok(Game { world, events_loop })
    }

    pub fn run(&self) {
        let map = self.world.read_resource::<ConfigMap>();
        println!("ConfigMap: {:?}", (*map));

        std::thread::sleep(std::time::Duration::from_secs(10));

        info!("Exiting...");
    }
}

pub struct ResourceBuilder<'a> {
    world: &'a World,
}

impl<'a> ResourceBuilder<'a> {
    pub fn with_config(
        self,
        path: impl AsRef<Path>,
        template: &[u8],
    ) -> Result<ResourceBuilder<'a>, ConfigError> {
        {
            let mut config_map = self.world.write_resource::<ConfigMap>();
            let args = self.world.read_resource::<ArgMatches>();
            (*config_map).0.insert(
                path.as_ref().to_str().unwrap().into(),
                Config::new(
                    resource_path(path, &*args),
                    &String::from_utf8_lossy(template),
                )?,
            );
        }

        Ok(self)
    }

    // shouldn't fail?
    pub fn with_shader(
        self,
        name: &str,
        vert_src: &'static [u8],
        frag_src: &'static [u8],
    ) -> ResourceBuilder<'a> {
        {
            let mut shader_map = self.world.write_resource::<ShaderMap>();
            (*shader_map).0.insert(name.into(), (vert_src, frag_src));
        }

        self
    }
}

fn resource_path(path: impl AsRef<Path>, args: &ArgMatches) -> PathBuf {
    let mut res_path = PathBuf::from(if args.is_present("dev") {
        "./resources/open/"
    } else {
        "./"
    });

    res_path.push(path);

    res_path
}
