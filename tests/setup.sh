#!/bin/sh

set -x

sudo ip tuntap add dev tun10 mode tun user $(whoami)
sudo ip address add 10.10.10.1/24 dev tun10
sudo ip link set tun10 up
