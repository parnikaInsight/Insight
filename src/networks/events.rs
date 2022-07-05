// Copyright 20l9 Parity Technologies (UK) Ltd.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

//! A basic key value store demonstrating libp2p and the mDNS and Kademlia protocols.
//!
//! 1. Using two terminal windows, start two instances. If you local network
//!    allows mDNS, they will automatically connect.
//!
//! 2. Type `PUT my-key my-value` in terminal one and hit return.
//!
//! 3. Type `GET my-key` in terminal two and hit return.
//!
//! 4. Close with Ctrl-c.
//!
//! You can also store provider records instead of key value records.
//!
//! 1. Using two terminal windows, start two instances. If you local network
//!    allows mDNS, they will automatically connect.
//!
//! 2. Type `PUT_PROVIDER my-key` in terminal one and hit return.
//!
//! 3. Type `GET_PROVIDERS my-key` in terminal two and hit return.
//!
//! 4. Close with Ctrl-c.

use libp2p::gossipsub;
use libp2p::gossipsub::GossipsubEvent;
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::record::store::RecordStore;
use libp2p::kad::{
    AddProviderOk, InboundRequest, Kademlia, KademliaEvent, PeerRecord, PutRecordOk, QueryResult,
    Record,
};
use libp2p::{
    mdns::{Mdns, MdnsEvent},
    swarm::NetworkBehaviourEventProcess,
    NetworkBehaviour,
};
use crossbeam_channel::{bounded, Sender};

#[derive(NetworkBehaviour)]
#[behaviour(event_process = true)]
pub struct MyBehaviour {
    pub gossipsub: libp2p::gossipsub::Gossipsub,
    pub kademlia: Kademlia<MemoryStore>,
    pub mdns: Mdns,
}

//Gossipsub
impl NetworkBehaviourEventProcess<GossipsubEvent> for MyBehaviour {
    // Called when `gossipsub` produces an event.
    fn inject_event(&mut self, event: GossipsubEvent) {
        if let gossipsub::GossipsubEvent::Message {
            propagation_source: peer_id,
            message_id: id,
            message,
        } = event
        {
            //When recieved
            println!(
                "Got message: {} with id: {} from peer: {:?}",
                String::from_utf8_lossy(&message.data),
                id,
                peer_id
            )
        }
    }
}
impl NetworkBehaviourEventProcess<MdnsEvent> for MyBehaviour {
    // Called when `mdns` produces an event.
    fn inject_event(&mut self, event: MdnsEvent) {
        if let MdnsEvent::Discovered(list) = event {
            for (peer_id, multiaddr) in list {
                println!("Discovered {}", peer_id);
                self.kademlia.add_address(&peer_id, multiaddr); //send address
                self.gossipsub.add_explicit_peer(&peer_id);
            }
        }
    }
}
// fn send_peers(event: MdnsEvent, sender: Sender<String>,) {
//     if let MdnsEvent::Discovered(list) = event {
//         for (peer_id, multiaddr) in list {
//             println!("Sending {}", peer_id);
//             sender.send(multiaddr.to_string()); //send address
//         }
//     }
//     else{
//         println!("No peers to send address of");
//     }
// }

impl NetworkBehaviourEventProcess<KademliaEvent> for MyBehaviour {
    // Called when `kademlia` produces an event.
    fn inject_event(&mut self, message: KademliaEvent) {
        match message {
            KademliaEvent::OutboundQueryCompleted { result, .. } => match result {
                QueryResult::GetProviders(Ok(ok)) => {
                    for peer in ok.providers {
                        println!(
                            "Peer {:?} provides key {:?}",
                            peer,
                            std::str::from_utf8(ok.key.as_ref()).unwrap()
                        );
                        println!("Peers: {:?}", ok.closest_peers);
                    }
                }
                QueryResult::GetProviders(Err(err)) => {
                    eprintln!("Failed to get providers: {:?}", err);
                }
                QueryResult::GetRecord(Ok(ok)) => {
                    for PeerRecord {
                        record: Record { key, value, .. },
                        ..
                    } in ok.records
                    {
                        println!(
                            "Got record {:?} {:?}",
                            std::str::from_utf8(key.as_ref()).unwrap(),
                            std::str::from_utf8(&value).unwrap(),
                        );
                    }
                }
                QueryResult::GetRecord(Err(err)) => {
                    eprintln!("Failed to get record: {:?}", err);
                }
                QueryResult::PutRecord(Ok(PutRecordOk { key })) => {
                    println!(
                        "Successfully put record {:?}",
                        std::str::from_utf8(key.as_ref()).unwrap()
                    );
                }
                QueryResult::PutRecord(Err(err)) => {
                    eprintln!("Failed to put record: {:?}", err);
                }
                QueryResult::StartProviding(Ok(AddProviderOk { key })) => {
                    println!(
                        "Successfully put provider record {:?}",
                        std::str::from_utf8(key.as_ref()).unwrap()
                    );
                }
                QueryResult::StartProviding(Err(err)) => {
                    eprintln!("Failed to put provider record: {:?}", err);
                }
                _ => {}
            },
            KademliaEvent::RoutablePeer { peer, .. } => {
                self.gossipsub.add_explicit_peer(&peer);
            }
            KademliaEvent::InboundRequest { request } => match request {
                InboundRequest::AddProvider { record } => {
                    self.kademlia.store_mut().add_provider(record.unwrap());
                }
                InboundRequest::PutRecord {
                    source,
                    connection,
                    record,
                } => {
                    self.kademlia.store_mut().put(record.unwrap());
                }
                _ => println!("{:?}", request),
            },
            _ => (),
        }
    }
}
