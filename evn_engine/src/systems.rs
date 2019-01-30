// only for most relevant engine systems

use crate::Running;
use crossbeam::channel::Receiver;
use specs::{ReadExpect, System, WriteExpect};
use winit::{/* DeviceEvent,*/ Event, WindowEvent};

pub struct EventHandler;

impl<'a> System<'a> for EventHandler {
    type SystemData = (ReadExpect<'a, Receiver<Event>>, WriteExpect<'a, Running>);

    fn run(&mut self, (event_receiver, mut running): Self::SystemData) {
        for event in event_receiver.try_iter() {
            //println!("Event: {:?}", event);

            match event {
                Event::DeviceEvent { event, .. } => match event {
                    _ => (),
                },
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => running.0 = false,
                    _ => (),
                },
                _ => (),
            }
        }
    }
}
