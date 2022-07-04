#![windows_subsystem = "windows"]

// Using game as a separate crate

pub mod blockchain;
pub mod db;
pub mod game;
pub mod networks;
pub mod validation;
use futures::executor::block_on;
use libp2p::{identity, PeerId};
use std::thread;
use libp2p::gossipsub::IdentTopic as Topic;
use crate::game::simulation;

use bevy::prelude::*;
use bevy::winit;
use bevy_rapier2d::prelude::*;
use crossbeam_channel::{bounded, Receiver};
use rand::Rng;
use std::time::{Duration, Instant};

// fn main() {
//     //random key
//     let priva = identity::Keypair::generate_ed25519();
//     // for boot nodes. Create by above^^
//     // let x: [u8; 68] = [
//     //     8, 1, 18, 64, 236, 219, 78, 215, 40, 219, 195, 32, 155, 130, 105, 2, 31, 197, 107, 68, 180,
//     //     113, 242, 11, 55, 254, 89, 219, 224, 73, 147, 124, 229, 211, 138, 11, 38, 25, 174, 72, 28,
//     //     220, 126, 249, 123, 12, 164, 200, 89, 111, 56, 135, 128, 88, 250, 164, 86, 74, 172, 121,
//     //     106, 120, 35, 196, 229, 115, 199, 174,
//     // ];
//     //let priva = identity::Keypair::from_protobuf_encoding(&x).unwrap();
//     let peerid = PeerId::from(priva.public());
//     let my_future = networks::protocol::start_protocol(priva, peerid);
    
//     block_on(my_future).expect("error");

//     // App::new()
//     //     .insert_resource(ClearColor(Color::rgb(0.53, 0.53, 0.53)))
//     //     .add_plugins(DefaultPlugins)
//     //     .add_startup_system(my_future)
//     //     .run();
    
//     // loop{
//     //     thread::spawn(|| {
//     //         networks::protocol::get_msgs(swarm, Topic::new("test-net"));
//     //     });
//     //     thread::spawn(|| {
//     //         simulation::create_app();
//     //     });
//     // }pu


// }

fn main() {
    App::new()
        .add_event::<StreamEvent>()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(read_stream)
        .add_system(spawn_text)
        .add_system(move_text)
        .run();
}

#[derive(Deref)]
struct StreamReceiver(Receiver<u32>);
struct StreamEvent(u32);

#[derive(Deref)]
struct LoadedFont(Handle<Font>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    //commands.spawn_bundle(Camera2dBundle::default());
    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    camera_bundle.orthographic_projection.scale = 1. / 50.;
    commands.spawn_bundle(camera_bundle);

    let (tx, rx) = bounded::<u32>(10); //10 msg cap, sender/receiver from crossbeam_channel
    std::thread::spawn(move || loop { //infinite loop in thread where rand # is created
        // Everything here happens in another thread
        // This is where you could connect to an external data source
        println!("thread");
        let priva = identity::Keypair::generate_ed25519();
        let peerid = PeerId::from(priva.public());
        let my_future = networks::protocol::start_protocol(priva, peerid);
        block_on(my_future).expect("error");
        println!("after");
        // let mut rng = rand::thread_rng();
        // let start_time = Instant::now();
        // let duration = Duration::from_secs_f32(rng.gen_range(0.0..0.2));
        // while start_time.elapsed() < duration {
        //     // Spinning for 'duration', simulating doing hard work!
        // }

        // tx.send(rng.gen_range(0..2000)).unwrap(); //sender sends rand # to receiver
    });

    commands.insert_resource(StreamReceiver(rx)); //custom receiver is inserted as a resource to receive from one channel and send to another
    commands.insert_resource(LoadedFont(asset_server.load("fonts/FiraSans-Bold.ttf")));
}

// This system reads from the receiver and sends events to Bevy
fn read_stream(receiver: ResMut<StreamReceiver>, mut events: EventWriter<StreamEvent>) { //access custom receiver
    for from_stream in receiver.try_iter() {
        events.send(StreamEvent(from_stream)); //send custom streamevent from other channel to bevy
    }
}

fn spawn_text(
    mut commands: Commands,
    mut reader: EventReader<StreamEvent>,
    loaded_font: Res<LoadedFont>,
) {
    let text_style = TextStyle {
        font: loaded_font.clone(),
        font_size: 20.0,
        color: Color::WHITE,
    };
    let text_alignment = TextAlignment {
        vertical: VerticalAlign::Center,
        horizontal: HorizontalAlign::Center,
    }; //access custom streamevent in bevy to spawn an entity with text component dependent on that event
    for (per_frame, event) in reader.iter().enumerate() {
        commands.spawn_bundle(Text2dBundle {
            text: Text::with_section(format!("{}", event.0), text_style.clone(), text_alignment),
            transform: Transform::from_xyz(
                per_frame as f32 * 100.0 + rand::thread_rng().gen_range(-40.0..40.0),
                300.0,
                0.0,
            ),
            ..default()
        });
    }
}
//despawn entity
fn move_text(
    mut commands: Commands,
    mut texts: Query<(Entity, &mut Transform), With<Text>>,
    time: Res<Time>,
) {
    for (entity, mut position) in texts.iter_mut() {
        position.translation -= Vec3::new(0.0, 100.0 * time.delta_seconds(), 0.0);
        if position.translation.y < -300.0 {
            commands.entity(entity).despawn();
        }
    }
}