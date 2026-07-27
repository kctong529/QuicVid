#!/usr/bin/env python3

import argparse
import shlex
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
        "auto_migrate": False,
        "suspect_after_ms": 250,
        "challenge_after_ms": 250,
        "impair_after_seconds": None,
        "impair_duration_seconds": None,
    },
    "preview": {
        "fps": 30,
        "duration_seconds": 5,
        "rebind": "10.0.2.2:0",
        "rebind_after_seconds": 2.5,
        "preview": True,
        "auto_migrate": False,
        "suspect_after_ms": 250,
        "challenge_after_ms": 250,
        "impair_after_seconds": None,
        "impair_duration_seconds": None,
    },
    "health-transient": {
        "fps": 10,
        "duration_seconds": 4,
        "rebind": None,
        "rebind_after_seconds": None,
        "preview": False,
        "auto_migrate": True,
        "suspect_after_ms": 250,
        "challenge_after_ms": 1000,
        "impair_after_seconds": 2.0,
        "impair_duration_seconds": 0.35,
    },
    "health-sustained": {
        "fps": 10,
        "duration_seconds": 3,
        "rebind": None,
        "rebind_after_seconds": None,
        "preview": False,
        "auto_migrate": True,
        "suspect_after_ms": 250,
        "challenge_after_ms": 500,
        "impair_after_seconds": 2.0,
        "impair_duration_seconds": None,
    },
}

REPO_ROOT = Path(__file__).resolve().parents[2]
CARGO_ROOT = REPO_ROOT / "quic-vid"
BINARY = CARGO_ROOT / "target" / "release" / "quic-vid"

# Mininet hosts share the filesystem, so this marker provides a common
# reference point between the client and impairment-control namespaces.
HEALTH_START_MARKER = Path("/tmp/quicvid-health-start")


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

    parser.add_argument("--fps", type=int, default=None, help="Override video frame rate")
    parser.add_argument(
        "--duration-seconds", type=int, default=None, help="Override video duration"
    )
    parser.add_argument(
        "--rebind",
        default=None,
        help="Override target local address for controlled endpoint rebind",
    )
    parser.add_argument(
        "--rebind-after-seconds",
        type=float,
        default=None,
        help="Override controlled migration trigger time",
    )
    parser.add_argument(
        "--preview",
        action=argparse.BooleanOptionalAction,
        default=None,
        help="Enable or disable receiver preview",
    )
    parser.add_argument(
        "--auto-migrate",
        action=argparse.BooleanOptionalAction,
        default=None,
        help="Enable or disable automatic path-health monitoring",
    )
    parser.add_argument(
        "--suspect-after-ms",
        type=int,
        default=None,
        help="Override no-progress time before entering Suspect",
    )
    parser.add_argument(
        "--challenge-after-ms",
        type=int,
        default=None,
        help="Override additional Suspect time before requesting a challenge",
    )
    parser.add_argument(
        "--impair-after-seconds",
        type=float,
        default=None,
        help="Override time from client start until Path A is impaired",
    )
    parser.add_argument(
        "--impair-duration-seconds",
        type=float,
        default=None,
        help="Override Path A impairment duration; omit for sustained impairment",
    )

    return parser.parse_args()


def apply_preset(args: argparse.Namespace) -> argparse.Namespace:
    preset = PRESETS[args.preset]

    for key in (
        "fps",
        "duration_seconds",
        "rebind",
        "rebind_after_seconds",
        "preview",
        "auto_migrate",
        "suspect_after_ms",
        "challenge_after_ms",
        "impair_after_seconds",
        "impair_duration_seconds",
    ):
        if getattr(args, key) is None:
            setattr(args, key, preset[key])

    return args


def validate_args(args: argparse.Namespace) -> None:
    if args.fps <= 0:
        raise ValueError("--fps must be greater than zero")

    if args.duration_seconds <= 0:
        raise ValueError("--duration-seconds must be greater than zero")

    if args.auto_migrate:
        if args.rebind is not None or args.rebind_after_seconds is not None:
            raise ValueError(
                "--auto-migrate cannot be combined with controlled rebind options"
            )

        if args.suspect_after_ms <= 0:
            raise ValueError("--suspect-after-ms must be greater than zero")

        if args.challenge_after_ms <= 0:
            raise ValueError("--challenge-after-ms must be greater than zero")
    else:
        if args.rebind is None or args.rebind_after_seconds is None:
            raise ValueError(
                "controlled migration requires --rebind and --rebind-after-seconds"
            )

        if args.rebind_after_seconds <= 0:
            raise ValueError("--rebind-after-seconds must be greater than zero")

        if args.rebind_after_seconds >= args.duration_seconds:
            raise ValueError(
                "--rebind-after-seconds must be smaller than --duration-seconds"
            )

    if args.impair_after_seconds is not None:
        if not args.auto_migrate:
            raise ValueError("scheduled path impairment requires --auto-migrate")

        if args.impair_after_seconds <= 0:
            raise ValueError("--impair-after-seconds must be greater than zero")

        if args.impair_after_seconds >= args.duration_seconds:
            raise ValueError(
                "--impair-after-seconds must be smaller than --duration-seconds"
            )

    if args.impair_duration_seconds is not None:
        if args.impair_after_seconds is None:
            raise ValueError(
                "--impair-duration-seconds requires --impair-after-seconds"
            )

        if args.impair_duration_seconds <= 0:
            raise ValueError("--impair-duration-seconds must be greater than zero")


def server_command(args: argparse.Namespace) -> str:
    command = f"{BINARY} server --listen 10.0.0.1:4433"
    if args.preview:
        command += " --preview"
    return command


def client_command(args: argparse.Namespace) -> str:
    parts = [
        str(BINARY),
        "client",
        "--connect",
        "10.0.0.1:4433",
        "--bind",
        "10.0.1.2:0",
        "--fps",
        str(args.fps),
        "--duration-seconds",
        str(args.duration_seconds),
    ]

    if args.auto_migrate:
        parts.extend(
            [
                "--auto-migrate",
                "--suspect-after-ms",
                str(args.suspect_after_ms),
                "--challenge-after-ms",
                str(args.challenge_after_ms),
            ]
        )
    else:
        parts.extend(
            [
                "--rebind",
                args.rebind,
                "--rebind-after-seconds",
                str(args.rebind_after_seconds),
            ]
        )

    return shlex.join(parts)


def terminal_command(command: str) -> str:
    return f"bash -lc {shlex.quote(command + '; exec bash')}"


def impairment_command(args: argparse.Namespace) -> str | None:
    if args.impair_after_seconds is None:
        return None

    marker = shlex.quote(str(HEALTH_START_MARKER))
    commands = [
        f"echo 'Waiting for client start marker: {HEALTH_START_MARKER}'",
        f"while [ ! -e {marker} ]; do sleep 0.01; done",
        "echo 'Client started'",
        f"echo 'Waiting {args.impair_after_seconds:.3f} s before impairing Path A'",
        f"sleep {args.impair_after_seconds}",
        "echo 'Path A DOWN: r1-eth0'",
        "ip link set r1-eth0 down",
    ]

    if args.impair_duration_seconds is None:
        commands.append("echo 'Path A remains unavailable'")
    else:
        commands.extend(
            [
                f"echo 'Keeping Path A down for {args.impair_duration_seconds:.3f} s'",
                f"sleep {args.impair_duration_seconds}",
                "echo 'Path A UP: r1-eth0'",
                "ip link set r1-eth0 up",
                "echo 'Transient impairment complete'",
            ]
        )

    return "; ".join(commands)


def print_configuration(args: argparse.Namespace) -> None:
    info("\n*** QuicVid migration demo configuration\n")
    info(f"*** Preset:              {args.preset}\n")
    info(f"*** FPS:                 {args.fps}\n")
    info(f"*** Duration:            {args.duration_seconds} s\n")
    info(f"*** Preview:             {'yes' if args.preview else 'no'}\n")

    if args.auto_migrate:
        info("*** Mode:                automatic path health\n")
        info(f"*** Suspect after:       {args.suspect_after_ms} ms\n")
        info(f"*** Challenge after:     {args.challenge_after_ms} ms\n")

        if args.impair_after_seconds is None:
            info("*** Path impairment:     none\n")
        elif args.impair_duration_seconds is None:
            info(
                f"*** Path impairment:     sustained after {args.impair_after_seconds} s\n"
            )
        else:
            info(
                f"*** Path impairment:     after {args.impair_after_seconds} s "
                f"for {args.impair_duration_seconds} s\n"
            )
    else:
        info("*** Mode:                controlled rebind\n")
        info(f"*** Rebind target:       {args.rebind}\n")
        info(f"*** Rebind after:        {args.rebind_after_seconds} s\n")


def launch_terminals(net, args: argparse.Namespace) -> None:
    client = net.get("client")
    server = net.get("server")
    r1 = net.get("r1")
    r2 = net.get("r2")

    server_cmd = server_command(args)
    client_cmd = client_command(args)
    impair_cmd = impairment_command(args)

    if impair_cmd is not None:
        client.cmd(f"rm -f {shlex.quote(str(HEALTH_START_MARKER))}")

    info("*** Launching server terminal\n")
    makeTerm(server, title="QuicVid Server", cmd=terminal_command(server_cmd))

    info("*** Launching Path A capture\n")
    makeTerm(
        r1,
        title="Path A - r1",
        cmd=terminal_command("tcpdump -ni r1-eth0 udp port 4433"),
    )

    info("*** Launching Path B capture\n")
    makeTerm(
        r2,
        title="Path B - r2",
        cmd=terminal_command("tcpdump -ni r2-eth0 udp port 4433"),
    )

    if impair_cmd is not None:
        info("*** Launching Path A impairment controller\n")
        makeTerm(
            r1,
            title="Path A Impairment",
            cmd=terminal_command(impair_cmd),
        )

    if impair_cmd is not None:
        start_client_cmd = (
            f"touch {shlex.quote(str(HEALTH_START_MARKER))}; {client_cmd}"
        )
    else:
        start_client_cmd = client_cmd

    client_shell = (
        "echo 'QuicVid migration demo'; "
        "echo; "
        f"echo {shlex.quote(client_cmd)}; "
        "echo; "
        "read -p 'Press Enter to start...'; "
        f"{start_client_cmd}"
    )

    info("*** Launching client terminal\n")
    makeTerm(
        client,
        title="QuicVid Client",
        cmd=terminal_command(client_shell),
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
        if args.impair_after_seconds is not None:
            info("*** Path A impairment will be scheduled relative to client start\n")
        input("*** Press Enter here when finished to stop Mininet...\n")
    finally:
        info("*** Stopping network\n")
        net.stop()
        HEALTH_START_MARKER.unlink(missing_ok=True)


if __name__ == "__main__":
    setLogLevel("info")
    main()
