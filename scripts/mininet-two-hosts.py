#!/usr/bin/env python3

from mininet.net import Mininet
from mininet.node import Controller
from mininet.cli import CLI
from mininet.log import setLogLevel, info


def main():
    net = Mininet(controller=Controller)

    info("*** Adding controller\n")
    net.addController("c0")

    info("*** Adding switch\n")
    s1 = net.addSwitch("s1")

    info("*** Adding hosts\n")
    h1 = net.addHost("h1", ip="10.0.0.1/24")
    h2 = net.addHost("h2", ip="10.0.0.2/24")

    info("*** Creating links\n")
    net.addLink(h1, s1)
    net.addLink(h2, s1)

    info("*** Starting network\n")
    net.start()

    info("*** Testing connectivity\n")
    net.pingAll()

    info("*** Hosts:\n")
    info(f"h1 IP: {h1.IP()}\n")
    info(f"h2 IP: {h2.IP()}\n")

    info("*** Starting CLI\n")
    CLI(net)

    info("*** Stopping network\n")
    net.stop()


if __name__ == "__main__":
    setLogLevel("info")
    main()
