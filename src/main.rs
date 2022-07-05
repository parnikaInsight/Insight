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
use multiaddr::{Protocol};
use ggrs::{P2PSession, PlayerType, SessionBuilder, UdpNonBlockingSocket};
use structopt::StructOpt;
use bevy_ggrs::{GGRSPlugin, SessionType};
use std::net::SocketAddr;

mod box_game;
use box_game::box_logic;

// cargo run -- --local-port 7000 --players localhost 127.0.0.1:7001
// cargo run -- --local-port 7001 --players 127.0.0.1:7000 localhost

const FPS: usize = 60;
const ROLLBACK_DEFAULT: &str = "rollback_default";

// structopt will read command line parameters for u
#[derive(StructOpt)]
struct Opt {
    #[structopt(short, long)]
    local_port: u16,
    #[structopt(short, long)]
    players: Vec<String>,
    #[structopt(short, long)]
    spectators: Vec<SocketAddr>,
}
fn main() {
    // // read cmd line arguments
    // let opt = Opt::from_args();
    // let num_players = opt.players.len(); //number of discovered peers
    // assert!(num_players > 0);

    // // create a GGRS session
    // let mut sess_build = SessionBuilder::<box_logic::GGRSConfig>::new()
    //     .with_num_players(num_players)
    //     .with_max_prediction_window(12) // (optional) set max prediction window
    //     .with_input_delay(2); // (optional) set input delay for the local player

    // add players
    // for (i, player_addr) in opt.players.iter().enumerate() {
    //     // local player
    //     if player_addr == "localhost" { //receive my listening on address
    //         sess_build = sess_build.add_player(PlayerType::Local, i)?;
    //     } else {
    //         // remote players
    //         let remote_addr: SocketAddr = player_addr.parse()?; //receive addr of discovered peers
    //         sess_build = sess_build.add_player(PlayerType::Remote(remote_addr), i)?;
    //     }
    // }

    //let socket = UdpNonBlockingSocket::bind_to_port(opt.local_port)?;
    //let sess = sess_build.start_p2p_session(socket.from_world())?;

    App::new()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor { //must come before default plugin
            width: 720.,
            height: 720.,
            title: "GGRS Box Game".to_owned(),
            ..Default::default()
        })
        .add_event::<StreamEvent>()
        .add_plugins(DefaultPlugins)
        //.add_startup_system(box_logic::setup_system)
        .add_startup_system(setup)
        .add_system(read_stream)
        .add_system(spawn_text)
        .run();
}

#[derive(Deref)]
struct StreamReceiver(Receiver<String>);
struct StreamEvent(String);

#[derive(Deref)]
struct PeerAddrReceiver(Receiver<String>);
struct PeerAddrEvent(String);

#[derive(Deref)]
struct LoadedFont(Handle<Font>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    //commands.spawn_bundle(Camera2dBundle::default());
    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    camera_bundle.orthographic_projection.scale = 1. / 50.;
    commands.spawn_bundle(camera_bundle);

    let (tx, rx) = bounded(10); //10 msg cap, sender/receiver from crossbeam_channel
    std::thread::spawn(move || loop { //infinite loop where swarm is created --only runs once since loop in startprotocol
        // Everything here happens in another thread
        // This is where you could connect to an external data source
        println!("thread");
        let priva = identity::Keypair::generate_ed25519();
        let peerid = PeerId::from(priva.public());
        let my_future = networks::protocol::start_protocol(priva, peerid, tx.clone());
        block_on(my_future).expect("error"); //send port that swarm chooses to be used by ggrs!!!!!!!
        println!("after");
    });

    commands.insert_resource(StreamReceiver(rx)); //custom receiver is inserted as a resource to receive from one channel and send to another
    commands.insert_resource(LoadedFont(asset_server.load("fonts/FiraSans-Bold.ttf")));

    // let (tx1, rx1) = bounded(10); //10 msg cap, sender/receiver from crossbeam_channel
    // std::thread::spawn(move || loop { //infinite loop where swarm is created --only runs once since loop in startprotocol
    //     // Everything here happens in another thread
    //     // This is where you could connect to an external data source
    //     println!("thread2");
    //     let my_future = networks::events::send_peers(priva, peerid, tx.clone());
    //     block_on(my_future).expect("error"); //send port that swarm chooses to be used by ggrs!!!!!!!
    //     println!("after");
    // });
}

// This system reads from the receiver and sends events to Bevy
fn read_stream(receiver: ResMut<StreamReceiver>, mut events: EventWriter<StreamEvent>) { //access custom receiver
    for from_stream in receiver.try_iter() {
        println!("from stream: {}", from_stream);
        events.send(StreamEvent(from_stream)); //send custom streamevent from other channel to bevy
    }
}

fn spawn_text(
    mut commands: Commands,
    mut reader: EventReader<StreamEvent>,
   // mut app: App,
) {//-> Result<(), Box<dyn std::error::Error>>{
    // // read cmd line arguments
    // let opt = Opt::from_args();
    // let num_players = opt.players.len(); //number of discovered peers
    // assert!(num_players > 0);

    // // create a GGRS session
    // let mut sess_build = SessionBuilder::<box_logic::GGRSConfig>::new()
    //     .with_num_players(num_players)
    //     .with_max_prediction_window(12) // (optional) set max prediction window
    //     .with_input_delay(2); // (optional) set input delay for the local player

    // // add players
    // for (i, player_addr) in opt.players.iter().enumerate() {
    //     // local player
    //     if player_addr == "localhost" { //receive my listening on address
    //         sess_build = sess_build.add_player(PlayerType::Local, i)?;
    //     } else {
    //         // remote players
    //         let remote_addr: SocketAddr = player_addr.parse()?; //receive addr of discovered peers
    //         sess_build = sess_build.add_player(PlayerType::Remote(remote_addr), i)?;
    //     }
    // }

    //access custom streamevent in bevy to spawn an entity with text component dependent on that event
    for (per_frame, event) in reader.iter().enumerate() {
        let port_num = &event.0[5..];
        let socket = UdpNonBlockingSocket::bind_to_port(port_num.parse::<u16>().unwrap());
        println!("Port: {}", port_num.parse::<u16>().unwrap());
       // let sess = sess_build.start_p2p_session(socket)?;
       // let sock_handle: Handle<UdpNonBlockingSocket> = new(Handle);
       // commands.insert_resource(socket);
        //let sess = sess_build.start_p2p_session(socket)?;
    }

    // GGRSPlugin::<box_logic::GGRSConfig>::new()
    //     // define frequency of rollback game logic update
    //     .with_update_frequency(FPS)
    //     // define system that returns inputs given a player handle, so GGRS can send the inputs around
    //     .with_input_system(box_logic::input)
    //     // register types of components AND resources you want to be rolled back
    //     .register_rollback_type::<Transform>()
    //     .register_rollback_type::<box_logic::Velocity>()
    //     .register_rollback_type::<box_logic::FrameCount>()
    //     // these systems will be executed as part of the advance frame update
    //     .with_rollback_schedule(
    //         Schedule::default().with_stage(
    //             ROLLBACK_DEFAULT,
    //             SystemStage::parallel()
    //                 .with_system(box_logic::move_cube_system)
    //                 .with_system(box_logic::increase_frame_system),
    //         ),
    //     )
    //     // make it happen in the bevy app
    //     .build(&mut app);
    
        // // add your GGRS session
        // app.insert_resource(sess)
        // .insert_resource(SessionType::P2PSession)

    //Ok(())
}