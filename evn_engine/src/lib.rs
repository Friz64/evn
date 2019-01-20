pub mod config;
pub mod logger;
pub mod prelude;
pub mod rendering;

use crate::{
    config::{Config, ConfigError, ConfigMap},
    logger::{Logger, LoggerInitError},
    rendering::shaders::ShaderMap,
};
use clap::{App, Arg, ArgMatches};
use hashbrown::HashMap;
use log::info;
use specs::World;
use std::path::{Path, PathBuf};

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
}

impl Game {
    pub fn run(&self) {
        let map = self.world.read_resource::<ConfigMap>();
        println!("ConfigMap: {:?}", (*map));

        info!("Exiting...");
    }
}

pub struct GameBuilder {
    world: World,
}

impl GameBuilder {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(version: &str) -> Result<Self, LoggerInitError> {
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

        let mut world = World::new();
        world.add_resource(clap);
        world.add_resource(ConfigMap(HashMap::new()));
        world.add_resource(ShaderMap(HashMap::new()));

        Ok(GameBuilder { world })
    }

    pub fn with_config(
        self,
        path: impl AsRef<Path>,
        template: &[u8],
    ) -> Result<GameBuilder, ConfigError> {
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
    ) -> GameBuilder {
        {
            let mut shader_map = self.world.write_resource::<ShaderMap>();
            (*shader_map).0.insert(name.into(), (vert_src, frag_src));
        }

        self
    }

    pub fn build(self) -> Game {
        Game { world: self.world }
    }
}

pub fn resource_path(path: impl AsRef<Path>, args: &ArgMatches) -> PathBuf {
    let mut res_path = PathBuf::from(if args.is_present("dev") {
        "./resources/open/"
    } else {
        "./"
    });

    res_path.push(path);

    res_path
}
