use std::{net::{IpAddr, Ipv4Addr, UdpSocket}, sync::{Arc, Mutex}, time::{Duration, Instant}};

use log::{debug, warn};
use serde::{Serialize, Deserialize};

use crate::{keypair::Cert, ErrorIdentity};

const BROADCAST_PORT: u16 = 9872;
const BROADCAST_POLL_SECS: u64 = 1;
const BROADCAST_POLL: Duration = Duration::from_secs(BROADCAST_POLL_SECS);
const INACTIVE_TIMEOUT: Duration = Duration::from_secs(BROADCAST_POLL_SECS * 10);

pub trait IpEvent: Send {
    fn ip_added(&mut self, addr: IpAddr, cert: &[u8]);
    fn ip_removed(&mut self, addr: IpAddr);
}

struct IpInfo {
    ip: IpAddr,
    cert: Vec<u8>,
    recved: Instant
}

pub struct PeerList {
    ips: Vec<IpInfo>,
    myip: IpAddr,
    subscribers: Vec<Box<dyn IpEvent>>
}

impl PeerList {
    pub fn new(subscribers: Vec<Box<dyn IpEvent>>, mycert: Vec<u8>) -> Result<Arc<Mutex<Self>>, ErrorIdentity> {
        let broadcast_ip = IpAddr::from(Ipv4Addr::UNSPECIFIED);
        let broadcast_sock = setup_socket(broadcast_ip, 0)?;

        let recv_sock  = setup_socket(broadcast_ip, BROADCAST_PORT)?;
        let local_ip = local_ip_address::local_ip()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e))?;
        
        let discovery_msg = DiscoveryMessage{ip: local_ip, cert: mycert.clone()};

        let pl = Arc::new(Mutex::new(
            PeerList {
                ips: vec![],
                myip: local_ip,
                subscribers: subscribers
            }
        ));

        let cpl = pl.clone();
        std::thread::spawn(move|| {
            let mut buf = [0u8; 65536];
            loop {
                match recv_broadcast(&recv_sock, &mut buf) {
                    Ok(msg) => cpl.lock().unwrap().process(msg),
                    Err(e) => warn!("Error receiving discovery message: {}", e)
                }
            }
        });

        let cpl = pl.clone();
        std::thread::spawn(move|| {
            loop {
                if let Err(e) = send_broadcast(&broadcast_sock, &discovery_msg) {
                    warn!("Error sending broadcast: {}", e);
                }
                cpl.lock().unwrap().cleanup();
                std::thread::sleep(BROADCAST_POLL);
            }
        });

        Ok(pl)
    }

    pub fn list(&self) -> Result<Vec<(IpAddr, Cert)>, ErrorIdentity> {
        let mut names = vec![];
        for ipinfo in &self.ips {
            let cert = Cert::from_bytes(&ipinfo.cert)?;
            names.push((ipinfo.ip, cert));
        }
        Ok(names)
    }

    fn process(&mut self, msg: DiscoveryMessage) {
        if msg.ip == self.myip {
            return
        }
        let now = Instant::now();
        let mut found = false;
        let mut fidx = 0;

        for (idx, ipinfo) in self.ips.iter().enumerate() {
            if msg.ip == ipinfo.ip {
                found = true;
                fidx = idx;
            }
        }
        if found {
            self.ips[fidx].recved = now;
        } else {
            self.ips.push(IpInfo{ip: msg.ip, cert: msg.cert, recved: now});
            for subscriber in &mut self.subscribers {
                let lastip = &self.ips[self.ips.len() - 1];
                subscriber.ip_added(lastip.ip, &lastip.cert);
            }
        }
    }

    fn cleanup(&mut self) {
        let now = Instant::now();
        self.ips = self.ips.drain(..).filter(|ipinfo| {
            let keep = (now - ipinfo.recved) <= INACTIVE_TIMEOUT;
            if !keep {
                for subscriber in &mut self.subscribers {
                    subscriber.ip_removed(ipinfo.ip);
                }
            }
            keep
        }).collect();
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DiscoveryMessage {
    ip: IpAddr,
    cert: Vec<u8>
}

fn setup_socket(ip: std::net::IpAddr, port: u16) -> std::io::Result<UdpSocket> {
    let sock = UdpSocket::bind((ip, port))?;
    sock.set_broadcast(true)?;
    Ok(sock)
}

fn recv_broadcast(sock: &UdpSocket, buf: &mut [u8]) -> std::io::Result<DiscoveryMessage> {
    let (_, addr) = sock.recv_from(buf)?;

    let msg = bincode::deserialize::<DiscoveryMessage>(&buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    debug!("Recieved msg: {:?}: {}", msg, addr);
    Ok(msg)
}

fn send_broadcast(sock: &UdpSocket, msg: &DiscoveryMessage) -> std::io::Result<()> {
    let sent = sock.send_to(&bincode::serialize(msg).unwrap(), ("255.255.255.255", BROADCAST_PORT))?;
    debug!("Sent: {}", sent);
    Ok(())
}