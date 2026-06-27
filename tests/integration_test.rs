use etherparse::{NetHeaders, PacketBuilder, PacketHeaders, TransportHeader};
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
        net: Some(NetHeaders::Ipv4(ip_header, _ext)),
        transport: Some(TransportHeader::Udp(udp_header)),
        payload,
        ..
    } = PacketHeaders::from_ip_slice(packet).expect("failed to parse packet")
    {
        assert_eq!(ip_header.source, [10, 10, 10, 1]);
        assert_eq!(ip_header.destination, [10, 10, 10, 2]);
        assert_eq!(udp_header.source_port, 2424);
        assert_eq!(udp_header.destination_port, 4242);
        assert_eq!(payload.slice(), data);
    } else {
        panic!("incorrect packet");
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

#[cfg(feature = "tokio")]
mod aio_tests {
    use super::*;
    use std::sync::Arc;
    use tun_tap::aio::Async;

    #[tokio::test]
    #[serial]
    async fn it_sents_packets_async() {
        let iface =
            Iface::without_packet_info("tun10", Mode::Tun).expect("failed to create a TUN device");
        let aio = Async::new(iface).expect("failed to create Async wrapper");

        let data = [99; 8];
        let socket = UdpSocket::bind("10.10.10.1:2426").expect("failed to bind");
        socket
            .send_to(&data, "10.10.10.2:4244")
            .expect("socket send failed");

        let mut buf = [0; 1500];
        let n = aio.recv(&mut buf).await.expect("async recv failed");

        let packet = &buf[..n];
        let headers =
            PacketHeaders::from_ip_slice(packet).expect("failed to parse received packet");
        if let PacketHeaders {
            net: Some(NetHeaders::Ipv4(ip, _ext)),
            transport: Some(TransportHeader::Udp(udp)),
            payload,
            ..
        } = headers
        {
            assert_eq!(ip.source, [10, 10, 10, 1]);
            assert_eq!(ip.destination, [10, 10, 10, 2]);
            assert_eq!(udp.source_port, 2426);
            assert_eq!(udp.destination_port, 4244);
            assert_eq!(payload.slice(), data);
        } else {
            panic!(
                "unexpected packet structure: {:?}",
                std::any::type_name_of_val(&headers)
            );
        }
    }

    #[tokio::test]
    #[serial]
    async fn it_receives_packets_async() {
        let iface =
            Iface::without_packet_info("tun10", Mode::Tun).expect("failed to create a TUN device");
        let aio = Async::new(iface).expect("failed to create Async wrapper");

        let data = [42; 12];
        let socket = UdpSocket::bind("10.10.10.1:2425").expect("failed to bind");

        let builder = PacketBuilder::ipv4([10, 10, 10, 2], [10, 10, 10, 1], 20).udp(4243, 2425);
        let packet = {
            let mut p = Vec::with_capacity(builder.size(data.len()));
            builder
                .write(&mut p, &data)
                .expect("failed to build packet");
            p
        };

        // Send the packet asynchronously into the TUN device.
        aio.send(&packet).await.expect("async send failed");

        let mut buf = [0; 50];
        let (n, src) = socket.recv_from(&mut buf).expect("socket recv failed");
        assert_eq!(n, data.len());
        assert_eq!(&buf[..n], &data);
        assert_eq!(src.ip(), IpAddr::V4(Ipv4Addr::new(10, 10, 10, 2)));
        assert_eq!(src.port(), 4243);
    }

    #[tokio::test(flavor = "multi_thread")]
    #[serial]
    async fn it_bidirectional_async() {
        let iface =
            Iface::without_packet_info("tun10", Mode::Tun).expect("failed to create a TUN device");
        // Drain any pending system traffic from the TUN buffer before testing.
        iface.set_non_blocking().expect("set_non_blocking");
        let mut drain = [0u8; 4096];
        while iface.recv(&mut drain).is_ok() {}
        let aio = Arc::new(Async::new(iface).expect("failed to create Async wrapper"));

        let data = [0xAB; 16];
        let socket = UdpSocket::bind("10.10.10.1:2427").expect("failed to bind");

        // Spawn a task that sends a packet into TUN via the async sender.
        let write_task = {
            let aio = aio.clone();
            tokio::spawn(async move {
                let builder =
                    PacketBuilder::ipv4([10, 10, 10, 2], [10, 10, 10, 1], 20).udp(4245, 2427);
                let mut packet = Vec::with_capacity(builder.size(data.len()));
                builder.write(&mut packet, &data).unwrap();
                aio.send(&packet).await.unwrap();
            })
        };

        // Meanwhile, the socket should receive the packet.
        let mut buf = [0; 64];
        let (n, src) = socket.recv_from(&mut buf).expect("socket recv failed");
        assert_eq!(&buf[..n], &data);
        assert_eq!(src.port(), 4245);

        write_task.await.unwrap();

        // Now send a packet from the socket and read it via async recv.
        // The TUN buffer may contain background kernel traffic (IGMP, MLDv2,
        // LLMNR, etc.), so loop until we find our packet by port numbers.
        let read_data = [0xCD; 8];
        socket
            .send_to(&read_data, "10.10.10.2:4246")
            .expect("socket send failed");

        let mut read_buf = [0; 1500];
        loop {
            let n = aio.recv(&mut read_buf).await.expect("async recv failed");
            let headers = PacketHeaders::from_ip_slice(&read_buf[..n]).expect("failed to parse");
            let PacketHeaders {
                net: Some(NetHeaders::Ipv4(_, _)),
                transport: Some(TransportHeader::Udp(udp)),
                ..
            } = &headers
            else {
                continue; // skip IPv6, IGMP, etc.
            };
            if udp.source_port != 2427 || udp.destination_port != 4246 {
                continue; // not our packet
            }
            // Found it — extract and assert.
            if let PacketHeaders {
                net: Some(NetHeaders::Ipv4(ip, _ext)),
                transport: Some(TransportHeader::Udp(udp)),
                payload,
                ..
            } = headers
            {
                assert_eq!(ip.source, [10, 10, 10, 1]);
                assert_eq!(ip.destination, [10, 10, 10, 2]);
                assert_eq!(udp.source_port, 2427);
                assert_eq!(udp.destination_port, 4246);
                assert_eq!(payload.slice(), read_data);
            }
            break;
        }
    }
}
