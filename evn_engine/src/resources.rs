use crate::{config::Config, logger::UnwrapOrLog, rendering::shaders::Shader};
use clap::ArgMatches;
use hashbrown::HashMap;
use specs::World;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
    thread,
};

#[derive(Debug)]
pub enum Resource {
    Config(Config),
    Shader(Shader),
}

#[derive(Debug)]
pub enum ResourceState {
    Loaded(Resource),
    Loading,
}

impl ResourceState {
    pub fn is_loaded(&self) -> bool {
        if let ResourceState::Loaded(_) = self {
            true
        } else {
            false
        }
    }
}

#[derive(Debug)]
pub struct Resources {
    resources: Arc<Mutex<HashMap<String, Arc<ResourceState>>>>,
}

impl Resources {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_resource<F: 'static>(&self, name: impl AsRef<str>, load: F)
    where
        F: FnOnce() -> Resource + Send + Sync,
    {
        let name = name.as_ref().to_owned();

        {
            let mut res = self.resources.lock().unwrap();
            (*res).insert(name.clone(), Arc::new(ResourceState::Loading));
        }

        thread::spawn({
            let res = self.resources.clone();
            move || {
                let loaded = load();

                let mut res = res.lock().unwrap();
                if let Some(val) = (*res).get_mut(&name) {
                    *(val) = Arc::new(ResourceState::Loaded(loaded));
                }
            }
        });
    }

    pub fn get_resource(&self, name: impl AsRef<str>) -> Arc<ResourceState> {
        {
            let res = self.resources.lock().unwrap();
            (*res)
                .get(name.as_ref())
                .expect("Called get_resource() on nonexistent resource")
                .clone()
        }
    }
}

impl Default for Resources {
    fn default() -> Self {
        Resources {
            resources: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

pub struct ResourceBuilder {
    pub world: Arc<RwLock<World>>,
}

impl ResourceBuilder {
    pub fn with_config<P: AsRef<Path> + Send + Sync + 'static>(
        self,
        name: impl AsRef<str>,
        path: P,
        template: &'static [u8],
    ) -> ResourceBuilder {
        let world = self.world.clone();
        {
            let read_world = world.read().unwrap();
            let resources = read_world.read_resource::<Resources>();

            (*resources).add_resource(name, {
                let world = world.clone();
                move || {
                    let read_world = world.read().unwrap();
                    let args = read_world.read_resource::<ArgMatches>();

                    Resource::Config(
                        Config::new(
                            resource_path(path.as_ref(), &args, true),
                            &String::from_utf8_lossy(template),
                        )
                        .unwrap_or_log("Config"),
                    )
                }
            });
        }

        self
    }

    pub fn with_shader<P: AsRef<Path> + Send + Sync + 'static>(
        self,
        name: impl AsRef<str>,
        vert_path: P,
        frag_path: P,
    ) -> ResourceBuilder {
        let world = self.world.clone();
        {
            let read_world = world.read().unwrap();
            let resources = read_world.read_resource::<Resources>();

            (*resources).add_resource(name, {
                let world = world.clone();
                move || {
                    let read_world = world.read().unwrap();
                    let args = read_world.read_resource::<ArgMatches>();

                    Resource::Shader({
                        let vert = {
                            let mut buf = Vec::new();
                            let mut file =
                                File::open(resource_path(vert_path.as_ref(), &args, false))
                                    .unwrap_or_log("VertexShader");
                            file.read_to_end(&mut buf).unwrap_or_log("VertexShader");
                            buf
                        };

                        let frag = {
                            let mut buf = Vec::new();
                            let mut file =
                                File::open(resource_path(frag_path.as_ref(), &args, false))
                                    .unwrap_or_log("FragmentShader");
                            file.read_to_end(&mut buf).unwrap_or_log("FragmentShader");
                            buf
                        };

                        Shader { vert, frag }
                    })
                }
            });
        }

        self
    }
}

fn resource_path(path: impl AsRef<Path>, args: &ArgMatches, open: bool) -> PathBuf {
    let mut res_path = PathBuf::from(match (args.is_present("dev"), open) {
        (true, true) => "./resources/open/",
        (true, false) => "./resources/closed/",
        (false, true) => "./",
        (false, false) => "./res/"
    });

    res_path.push(path);

    res_path
}
