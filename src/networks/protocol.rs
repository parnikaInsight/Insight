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
use libp2p::{
    development_transport, identity,
    mdns::{Mdns, MdnsConfig},
    swarm::SwarmEvent,
    PeerId, Swarm,
};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::time::Duration;

pub async fn start_protocol(
    local_key: identity::Keypair,
    local_peer_id: PeerId,
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
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    //
    //swarm.listen_on("/ip4/10.150.108.167/tcp/51736".parse()?)?;
    swarm = zkademlia::boot(swarm);
    // Kick it off.
    loop {
        select! {
            line = stdin.select_next_some() => input::handle_input_line(&mut swarm.behaviour_mut(), line.expect("Stdin not to close"), topic.clone()),
            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening in {:?}", address);
                },
                _ => {}
            }
        }
        swarm
            .behaviour_mut()
            .kademlia
            .store_mut()
            .retain(validate::validate);
    }
}

//Split Gossip and kademlia
