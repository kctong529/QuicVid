#!/usr/bin/env python3

import argparse
from pathlib import Path

from mininet.log import info, setLogLevel
from mininet.term import makeTerm

from dual_path import create_network


PRESETS = {
    "diagnostic": {
        "fps": 1,
        "duration_seconds": 2,
        "rebind": "10.0.2.2:0",
        "rebind_after_seconds": 0.5,
        "preview": False,
    },
    "preview": {
        "fps": 30,
        "duration_seconds": 5,
        "rebind": "10.0.2.2:0",
        "rebind_after_seconds": 2.5,
        "preview": True,
    },
}

REPO_ROOT = Path(__file__).resolve().parents[2]
CARGO_ROOT = REPO_ROOT / "quic-vid"
BINARY = CARGO_ROOT / "target" / "release" / "quic-vid"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run the QuicVid dual-path migration demo."
    )

    parser.add_argument(
        "--preset",
        choices=PRESETS.keys(),
        default="diagnostic",
        help="Demo preset to use (default: diagnostic)",
    )

    parser.add_argument(
        "--fps",
        type=int,
        default=None,
        help="Override video frame rate",
    )

    parser.add_argument(
        "--duration-seconds",
        type=int,
        default=None,
        help="Override video duration",
    )

    parser.add_argument(
        "--rebind",
        default=None,
        help="Override target local address for endpoint rebind",
    )

    parser.add_argument(
        "--rebind-after-seconds",
        type=float,
        default=None,
        help="Override migration trigger time",
    )

    parser.add_argument(
        "--preview",
        action=argparse.BooleanOptionalAction,
        default=None,
        help="Enable or disable receiver preview",
    )

    return parser.parse_args()


def apply_preset(args: argparse.Namespace) -> argparse.Namespace:
    preset = PRESETS[args.preset]

    if args.fps is None:
        args.fps = preset["fps"]

    if args.duration_seconds is None:
        args.duration_seconds = preset["duration_seconds"]

    if args.rebind is None:
        args.rebind = preset["rebind"]

    if args.rebind_after_seconds is None:
        args.rebind_after_seconds = preset["rebind_after_seconds"]

    if args.preview is None:
        args.preview = preset["preview"]

    return args


def validate_args(args: argparse.Namespace) -> None:
    if args.fps <= 0:
        raise ValueError("--fps must be greater than zero")

    if args.duration_seconds <= 0:
        raise ValueError("--duration-seconds must be greater than zero")

    if args.rebind_after_seconds <= 0:
        raise ValueError("--rebind-after-seconds must be greater than zero")

    if args.rebind_after_seconds >= args.duration_seconds:
        raise ValueError(
            "--rebind-after-seconds must be smaller than "
            "--duration-seconds"
        )


def server_command(args: argparse.Namespace) -> str:
    command = (
        f"{BINARY} server "
        "--listen 10.0.0.1:4433"
    )

    if args.preview:
        command += " --preview"

    return command


def client_command(args: argparse.Namespace) -> str:
    return (
        f"{BINARY} client "
        "--connect 10.0.0.1:4433 "
        "--bind 10.0.1.2:0 "
        f"--rebind {args.rebind} "
        f"--rebind-after-seconds {args.rebind_after_seconds} "
        f"--fps {args.fps} "
        f"--duration-seconds {args.duration_seconds}"
    )


def print_configuration(args: argparse.Namespace) -> None:
    info("\n*** QuicVid migration demo configuration\n")
    info(f"*** Preset:              {args.preset}\n")
    info(f"*** FPS:                 {args.fps}\n")
    info(f"*** Duration:            {args.duration_seconds} s\n")
    info(f"*** Rebind target:       {args.rebind}\n")
    info(
        f"*** Rebind after:        "
        f"{args.rebind_after_seconds} s\n"
    )
    info(
        f"*** Preview:             "
        f"{'yes' if args.preview else 'no'}\n"
    )


def launch_terminals(net, args: argparse.Namespace) -> None:
    client = net.get("client")
    server = net.get("server")
    r1 = net.get("r1")
    r2 = net.get("r2")

    server_cmd = server_command(args)
    client_cmd = client_command(args)

    info("*** Launching server terminal\n")
    makeTerm(
        server,
        title="QuicVid Server",
        cmd=(
            "bash -lc '"
            f"{server_cmd}; "
            "exec bash'"
        ),
    )

    info("*** Launching Path A capture\n")
    makeTerm(
        r1,
        title="Path A - r1",
        cmd=(
            "bash -lc '"
            "tcpdump -ni r1-eth0 udp port 4433; "
            "exec bash'"
        ),
    )

    info("*** Launching Path B capture\n")
    makeTerm(
        r2,
        title="Path B - r2",
        cmd=(
            "bash -lc '"
            "tcpdump -ni r2-eth0 udp port 4433; "
            "exec bash'"
        ),
    )

    info("*** Launching client terminal\n")
    makeTerm(
        client,
        title="QuicVid Client",
        cmd=(
            "bash -lc '"
            "echo \"QuicVid migration demo\"; "
            "echo; "
            f"echo \"{client_cmd}\"; "
            "echo; "
            "read -p \"Press Enter to start...\"; "
            f"{client_cmd}; "
            "exec bash'"
        ),
    )


def main() -> None:
    args = parse_args()
    args = apply_preset(args)
    validate_args(args)

    if not BINARY.exists():
	    raise FileNotFoundError(
	        f"QuicVid release binary not found: {BINARY}\n"
	        "Run: cargo build --release --manifest-path quic-vid/Cargo.toml"
	    )

    print_configuration(args)

    info("*** Creating dual-path Mininet topology\n")
    net = create_network()

    try:
        launch_terminals(net, args)

        info("\n*** Migration demo ready\n")
        info("*** Press Enter in the client terminal to start\n")
        input("*** Press Enter here when finished to stop Mininet...\n")
    finally:
        info("*** Stopping network\n")
        net.stop()


if __name__ == "__main__":
    setLogLevel("info")
    main()
