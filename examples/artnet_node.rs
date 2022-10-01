use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

fn main() {
    // Use the default ArtNet Port
    let port = tiny_artnet::PORT;

    // // Lookup the local IP Address
    // let ip_address: [u8; 4] = match local_ip_address::local_ip().unwrap() {
    //     IpAddr::V4(ip) => ip.octets(),
    //     IpAddr::V6(_ip) => unimplemented!("IPV6 support"),
    // };

    // Or hard code the loopback address
    let ip_address = Ipv4Addr::new(127, 0, 0, 1).octets();

    // Lookup the mac address
    let mac_address_bytes = mac_address::get_mac_address().unwrap().unwrap().bytes();

    // Open the UDP socket
    let socket = UdpSocket::bind(SocketAddr::from((ip_address, port))).unwrap();

    println!(
        "\n\nServer Started, listening on {}:{}",
        IpAddr::from(ip_address),
        port
    );

    // Receives a single datagram message on the socket. If `buf` is too small to hold
    // the message, it will be cut off.
    let mut buf = [0; 65_507];
    use tiny_artnet::Art;

    loop {
        let (len, from_addr) = socket.recv_from(&mut buf).unwrap();

        // println!("{:?}", buf);
        match tiny_artnet::from_slice(&buf[..len]) {
            Ok(Art::Dmx(dmx)) => {
                println!(
                    "RX: ArtDMX - These packets contain data for one DMX512 universe - use them to control your node's lighting, etc. Seq: {:?} Data: {:?}...",
                    dmx.sequence,
                    &dmx.data[0..10],
                );
            }
            Ok(Art::Sync) => {
                println!("RX: ArtSync - Use these to buffer DMX packets and then synchronize the rendering of multiple DMX universes.");
            }
            Ok(Art::Poll(poll)) => {
                println!("RX: ArtPoll - Someone is looking for ArtNet nodes. Let's respond to them to make this node discoverable! {:?}", poll);

                let poll_reply = tiny_artnet::PollReply {
                    ip_address: &ip_address,
                    port,
                    firmware_version: 0x0001,
                    short_name: "Example Node",
                    long_name: "Tiny Artnet Example Node",
                    mac_address: &mac_address_bytes,
                    // This Node has one port
                    num_ports: 1,
                    // This node has one output channel
                    port_types: &[0b10000000, 0, 0, 0],
                    // Report that data is being output correctly
                    good_output_a: &[0b10000000, 0, 0, 0],
                    ..Default::default()
                };

                let msg_len = poll_reply.serialize(&mut buf);
                socket.send_to(&buf[..msg_len], &from_addr).unwrap();
                // let broadcast: UdpSocket = UdpSocket::bind("0.0.0.0:0").unwrap();
                // broadcast
                //     .set_read_timeout(Some(Duration::new(5, 0)))
                //     .unwrap();
                // broadcast.set_broadcast(true).unwrap();
                // broadcast
                //     .send_to(&buf[..msg_len], "255.255.255.255")
                //     .unwrap();

                println!("TX: Sent ArtPollReply to {:?}: {:?}", from_addr, poll_reply);
            }
            Err(err) => {
                println!("Error: {:?}", err);
            }
            msg => {
                println!("Something else! {:?}", msg);
            }
        };
    }
}
