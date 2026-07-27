#!/usr/bin/env python3

from mininet.cli import CLI
from mininet.log import info, setLogLevel
from mininet.net import Mininet
from mininet.node import Node
from mininet.link import TCLink


class LinuxRouter(Node):
    """Mininet node with IPv4 forwarding enabled."""

    def config(self, **params):
        super().config(**params)
        self.cmd("sysctl -w net.ipv4.ip_forward=1")

    def terminate(self):
        self.cmd("sysctl -w net.ipv4.ip_forward=0")
        super().terminate()


def main():
    net = Mininet(
        controller=None,
        link=TCLink,
        build=False,
    )

    info("*** Adding client and server\n")

    client = net.addHost("client")
    server = net.addHost("server")

    info("*** Adding path routers\n")

    r1 = net.addHost("r1", cls=LinuxRouter)
    r2 = net.addHost("r2", cls=LinuxRouter)

    info("*** Adding links\n")

    # Client path A.
    net.addLink(
        client,
        r1,
        intfName1="client-eth0",
        intfName2="r1-eth0",
    )

    # Client path B.
    net.addLink(
        client,
        r2,
        intfName1="client-eth1",
        intfName2="r2-eth0",
    )

    # Both routers reach the same server.
    net.addLink(
        r1,
        server,
        intfName1="r1-eth1",
        intfName2="server-eth0",
    )

    net.addLink(
        r2,
        server,
        intfName1="r2-eth1",
        intfName2="server-eth1",
    )

    info("*** Building network\n")
    net.build()

    info("*** Configuring addresses\n")

    client.setIP("10.0.1.2/24", intf="client-eth0")
    client.setIP("10.0.2.2/24", intf="client-eth1")

    r1.setIP("10.0.1.1/24", intf="r1-eth0")
    r1.setIP("10.0.3.1/24", intf="r1-eth1")

    r2.setIP("10.0.2.1/24", intf="r2-eth0")
    r2.setIP("10.0.4.1/24", intf="r2-eth1")

    server.setIP("10.0.3.2/24", intf="server-eth0")
    server.setIP("10.0.4.2/24", intf="server-eth1")

    info("*** Network ready\n")
    CLI(net)

    info("*** Stopping network\n")
    net.stop()


if __name__ == "__main__":
    setLogLevel("info")
    main()
