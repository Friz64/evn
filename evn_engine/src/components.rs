use nalgebra::{geometry, Real};
use specs::{storage::*, Component, World};
use specs_derive::Component;

pub fn register(world: &mut World) {
    world.register::<Translation3<f32>>();
    world.register::<Translation3<f64>>();

    world.register::<Translation2<f32>>();
    world.register::<Translation2<f64>>();

    world.register::<Rotation3<f32>>();
    world.register::<Rotation3<f64>>();

    world.register::<Rotation2<f32>>();
    world.register::<Rotation2<f64>>();
}

#[derive(Component)]
pub struct Translation2<T: Real>(pub geometry::Translation2<T>);
#[derive(Component)]
pub struct Translation3<T: Real>(pub geometry::Translation3<T>);

#[derive(Component)]
pub struct Rotation2<T: Real>(pub geometry::Rotation2<T>);
#[derive(Component)]
pub struct Rotation3<T: Real>(pub geometry::Rotation3<T>);
