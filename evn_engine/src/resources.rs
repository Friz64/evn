use crate::{config::Config, rendering::Shader};
use fnv::{FnvBuildHasher, FnvHashMap};
use log::{info, warn};
use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    io::{Cursor, Error as IoError, Read},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
    thread,
};

pub type Resources = Arc<RwLock<ResourcesData>>;

#[derive(Debug)]
pub enum Resource {
    Config(Config),
    Shader(Shader),
}

#[derive(Debug)]
pub enum ResourceState {
    Loaded(Resource),
    Loading,
    Failed,
}

impl ResourceState {
    pub fn is_loaded(&self) -> bool {
        if let ResourceState::Loaded(_) = self {
            true
        } else {
            false
        }
    }
    pub fn is_loading(&self) -> bool {
        if let ResourceState::Loading = self {
            true
        } else {
            false
        }
    }
    pub fn is_failed(&self) -> bool {
        if let ResourceState::Failed = self {
            true
        } else {
            false
        }
    }
}

#[derive(Debug)]
pub struct ResourcesData {
    resources: Arc<Mutex<HashMap<String, Arc<ResourceState>, FnvBuildHasher>>>,
}

impl ResourcesData {
    pub fn new() -> Self {
        ResourcesData {
            resources: Arc::new(Mutex::new(FnvHashMap::default())),
        }
    }

    pub fn add_resource<F: 'static, E>(&self, name: impl AsRef<str>, load: F)
    where
        F: FnOnce() -> Result<Resource, E> + Send + Sync,
        E: Display,
    {
        let name = name.as_ref().to_owned();

        {
            let mut res = self.resources.lock().unwrap();
            (*res).insert(name.clone(), Arc::new(ResourceState::Loading));
        }

        thread::spawn({
            let res = self.resources.clone();
            move || {
                match load() {
                    Ok(loaded) => {
                        let mut res = res.lock().unwrap();
                        if let Some(val) = (*res).get_mut(&name) {
                            *(val) = Arc::new(ResourceState::Loaded(loaded));
                            info!("Resource \"{}\" loaded!", name);
                        }
                    }
                    Err(err) => {
                        warn!("Failed to load resource {}: {}", name, err);

                        let mut res = res.lock().unwrap();
                        if let Some(val) = (*res).get_mut(&name) {
                            *(val) = Arc::new(ResourceState::Failed);
                        }
                    }
                };
            }
        });
    }

    pub fn get_resource(&self, name: impl AsRef<str>) -> Arc<ResourceState> {
        {
            let res = self.resources.lock().unwrap();
            (*res)
                .get(name.as_ref())
                .expect("Called Resources::get_resource on nonexistent resource")
                .clone()
        }
    }
}

pub struct ResourceBuilder {
    pub res: Resources,
    pub is_dev: bool,
    pub names: HashMap<String, Vec<String>, FnvBuildHasher>,
}

impl ResourceBuilder {
    pub fn with_config<P: AsRef<Path> + Send + Sync + 'static>(
        mut self,
        name: impl AsRef<str>,
        path: P,
        template: &'static [u8],
    ) -> ResourceBuilder {
        let names = self.names.entry("configs".into()).or_insert(Vec::new());
        (*names).push(name.as_ref().to_owned());

        let is_dev = self.is_dev;
        {
            let resources = self.res.read().unwrap();
            (*resources).add_resource(name, {
                move || {
                    let path = path.as_ref();

                    let config = Config::new(
                        resource_path(path, is_dev, true),
                        &String::from_utf8_lossy(template),
                    );

                    config.map(Resource::Config)
                }
            });
        }

        self
    }

    pub fn with_shader<P: AsRef<Path> + Send + Sync + 'static>(
        mut self,
        name: impl AsRef<str>,
        vert_path: P,
        frag_path: P,
    ) -> ResourceBuilder {
        let names = self.names.entry("shaders".into()).or_insert(Vec::new());
        (*names).push(name.as_ref().to_owned());

        let is_dev = self.is_dev;
        {
            let resources = self.res.read().unwrap();
            (*resources).add_resource(name, {
                move || -> Result<_, IoError> {
                    {
                        Ok(Resource::Shader({
                            let vert = {
                                let path = resource_path(vert_path.as_ref(), is_dev, false);
                                let mut buf = Vec::new();
                                let mut file = File::open(path)?;
                                file.read_to_end(&mut buf)?;
                                buf
                            };

                            let frag = {
                                let path = resource_path(frag_path.as_ref(), is_dev, false);
                                let mut buf = Vec::new();
                                let mut file = File::open(path)?;
                                file.read_to_end(&mut buf)?;
                                buf
                            };

                            Shader {
                                vert: ash::util::read_spv(&mut Cursor::new(&vert))?,
                                frag: ash::util::read_spv(&mut Cursor::new(&frag))?,
                            }
                        }))
                    }
                }
            });
        }

        self
    }
}

fn resource_path(path: impl AsRef<Path>, is_dev: bool, open: bool) -> PathBuf {
    let mut res_path = PathBuf::from(match (is_dev, open) {
        (true, true) => "./resources/open/",
        (true, false) => "./resources/closed/",
        (false, true) => "./",
        (false, false) => "./res/",
    });

    res_path.push(path);

    res_path
}
