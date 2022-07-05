use crate::networks::events::MyBehaviour;
use crate::networks::input;
use crate::networks::validate;
use crate::networks::zgossipsub;
use crate::networks::zkademlia;
use crate::networks::zmdns;
use async_std::{io, task};
use futures::{prelude::*, select};
use libp2p::gossipsub;
use libp2p::gossipsub::{
    GossipsubMessage, IdentTopic as Topic, MessageAuthenticity, MessageId, ValidationMode,
};
use libp2p::kad;
use libp2p::kad::record::store::{MemoryStore, MemoryStoreConfig};
use libp2p::kad::Kademlia;
use libp2p::kad::KademliaStoreInserts;
use libp2p::kad::QueryInfo;
use libp2p::multiaddr::Multiaddr;
use libp2p::swarm::handler::multi;
use libp2p::{
    development_transport, identity,
    mdns::{Mdns, MdnsConfig},
    swarm::SwarmEvent,
    PeerId, Swarm,
};
use serde_json::to_string;
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use crossbeam_channel::{bounded, Sender, Receiver};
use std::net::Ipv4Addr;
use multiaddr::{Protocol};
use bevy::ecs::event::EventReader;

// #[derive(Deref)]
// struct StreamReceiver(Receiver<String>);
// struct StreamEvent(String);

// This system reads from the receiver and sends events to Bevy
// fn read_stream(receiver: ResMut<StreamReceiver>, mut events: EventWriter<StreamEvent>) { //access custom receiver
//     for from_stream in receiver.try_iter() {
//         println!("from stream: {}", from_stream);
//         events.send(StreamEvent(from_stream)); //send custom streamevent from other channel to bevy
//     }
// }

pub async fn start_protocol(
    local_key: identity::Keypair,
    local_peer_id: PeerId,
    sender: Sender<String>,
   // mut reader: EventReader<StreamEvent>,
    //sender: Sender<Protocol<'_>>,
) -> Result<(), Box<dyn Error>> {
    //env_logger::init(); this messes up w/ bevy: thread '<unnamed>' panicked at 'env_logger::init should not be called after logger initialized: SetLoggerError(())'

    println!("{:?}", local_peer_id);

    let mut swarm = {
        let transport = development_transport(local_key.clone()).await?;
        let gossipsub: gossipsub::Gossipsub = zgossipsub::create_gossip(local_key.clone());
        let kademlia: Kademlia<MemoryStore> = zkademlia::create_kademlia(local_key.clone());
        let mdns = task::block_on(Mdns::new(MdnsConfig::default()))?;
        let behaviour = MyBehaviour {
            gossipsub,
            kademlia,
            mdns,
        };
        Swarm::new(transport, behaviour, local_peer_id)
    };

    let topic = Topic::new("test-net");
    swarm.behaviour_mut().gossipsub.subscribe(&topic);

    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();

    // Listen on all interfaces and whatever port the OS assigns.
    let multi_addr: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
    //let multi_addr: Multiaddr = "/ip4/192.168.0.25/tcp/56986".parse().unwrap();
    //let m_clone = multi_addr.clone();
    //let components = m_clone.iter().collect::<Vec<_>>();
    // assert_eq!(components[0], Protocol::Ip4(Ipv4Addr::new(127, 0, 0, 1)));
    // assert_eq!(components[1], Protocol::Udt);
    //assert_eq!(components[1], Protocol::Tcp(56986));
    swarm.listen_on(multi_addr)?;
    //sender.send(components[1].clone().acquire());
    //sender.send(components[1].clone().acquire().to_string());

    //
    //swarm.listen_on("/ip4/192.168.0.25/tcp/56986".parse()?)?;
    swarm = zkademlia::boot(swarm);
    // Kick it off.
    loop {
        select! {
            line = stdin.select_next_some() => input::handle_input_line(&mut swarm.behaviour_mut(), line.expect("Stdin not to close"), topic.clone()),
            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening in {:?}", address);
                    let components = address.iter().collect::<Vec<_>>();
                    sender.send(components[1].to_string());
                },
                // MdnsEvent => {
                //     for (per_frame, event) in reader.iter().enumerate() {
                //         let address = &event.0;
                //         println!("Address of Peer: {}", address.parse()?);
                //        // let sock_handle: Handle<UdpNonBlockingSocket> = new(Handle);
                //        // commands.insert_resource(socket);
                //         //let sess = sess_build.start_p2p_session(socket)?;
                //     }
                // },
                _ => {}
            }
        }
        swarm
            .behaviour_mut()
            .kademlia
            .store_mut()
            .retain(validate::validate);
        
        //sender.send(swarm.behaviour_mut().kademlia.get_closest_local_peers(local_key.clone()).to_string());
    }
}

//Split Gossip and kademlia
