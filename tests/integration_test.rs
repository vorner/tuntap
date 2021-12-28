extern crate etherparse;
extern crate serial_test;
extern crate tun_tap;

use etherparse::{IpHeader, PacketBuilder, PacketHeaders, TransportHeader};
use serial_test::serial;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use tun_tap::{Iface, Mode};

#[test]
#[serial]
fn it_sents_packets() {
    let iface =
        Iface::without_packet_info("tun10", Mode::Tun).expect("failed to create a TUN device");
    let data = [1; 10];
    let socket = UdpSocket::bind("10.10.10.1:2424").expect("failed to bind to address");
    socket
        .send_to(&data, "10.10.10.2:4242")
        .expect("failed to send data");
    let mut buf = [0; 50];
    let num = iface.recv(&mut buf).expect("failed to receive data");
    assert_eq!(num, 38);
    let packet = &buf[..num];
    if let PacketHeaders {
        ip: Some(IpHeader::Version4(ip_header)),
        transport: Some(TransportHeader::Udp(udp_header)),
        payload,
        ..
    } = PacketHeaders::from_ip_slice(&packet).expect("failed to parse packet")
    {
        assert_eq!(ip_header.source, [10, 10, 10, 1]);
        assert_eq!(ip_header.destination, [10, 10, 10, 2]);
        assert_eq!(udp_header.source_port, 2424);
        assert_eq!(udp_header.destination_port, 4242);
        assert_eq!(payload, data);
    } else {
        assert!(false, "incorrect packet");
    }
}

#[test]
#[serial]
fn it_receives_packets() {
    let iface =
        Iface::without_packet_info("tun10", Mode::Tun).expect("failed to create a TUN device");
    let data = [1; 10];
    let socket = UdpSocket::bind("10.10.10.1:2424").expect("failed to bind to address");
    let builder = PacketBuilder::ipv4([10, 10, 10, 2], [10, 10, 10, 1], 20).udp(4242, 2424);
    let packet = {
        let mut packet = Vec::<u8>::with_capacity(builder.size(data.len()));
        builder
            .write(&mut packet, &data)
            .expect("failed to build packet");
        packet
    };
    iface.send(&packet).expect("failed to send packet");
    let mut buf = [0; 50];
    let (num, source) = socket
        .recv_from(&mut buf)
        .expect("failed to receive packet");
    assert_eq!(num, 10);
    assert_eq!(source.ip(), IpAddr::V4(Ipv4Addr::new(10, 10, 10, 2)));
    assert_eq!(source.port(), 4242);
    assert_eq!(data, &buf[..num]);
}

#[test]
#[serial]
fn it_receives_packets_on_cloned_interface() {
    let iface =
        Iface::without_packet_info("tun10", Mode::Tun).expect("failed to create a TUN device");
    let data = [5; 10];
    let socket = UdpSocket::bind("10.10.10.1:2424").expect("failed to bind to address");
    let builder = PacketBuilder::ipv4([10, 10, 10, 3], [10, 10, 10, 1], 20).udp(4242, 2424);
    let packet = {
        let mut packet = Vec::<u8>::with_capacity(builder.size(data.len()));
        builder
            .write(&mut packet, &data)
            .expect("failed to build packet");
        packet
    };
    iface
        .try_clone()
        .expect("failed to clone interface")
        .send(&packet)
        .expect("failed to send packet");
    let mut buf = [0; 50];
    let (num, source) = socket
        .recv_from(&mut buf)
        .expect("failed to receive packet");
    assert_eq!(num, 10);
    assert_eq!(source.ip(), IpAddr::V4(Ipv4Addr::new(10, 10, 10, 3)));
    assert_eq!(source.port(), 4242);
    assert_eq!(data, &buf[..num]);
}
