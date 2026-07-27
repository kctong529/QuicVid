#!/usr/bin/env python3

import argparse
from pathlib import Path


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
        "duration_seconds": 10,
        "rebind": "10.0.2.2:0",
        "rebind_after_seconds": 5.0,
        "preview": True,
    },
}

REPO_ROOT = Path(__file__).resolve().parents[2]
BINARY = REPO_ROOT / "target" / "release" / "quic-vid"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Prepare a QuicVid dual-path migration demo."
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
    print("QuicVid migration demo configuration")
    print()
    print(f"Preset:              {args.preset}")
    print(f"FPS:                 {args.fps}")
    print(f"Duration:            {args.duration_seconds} s")
    print(f"Rebind target:       {args.rebind}")
    print(f"Rebind after:        {args.rebind_after_seconds} s")
    print(f"Preview:             {'yes' if args.preview else 'no'}")
    print()


def print_commands(args: argparse.Namespace) -> None:
    print("Server command:")
    print(server_command(args))
    print()

    print("Client command:")
    print(client_command(args))
    print()


def main() -> None:
    args = parse_args()
    args = apply_preset(args)
    validate_args(args)

    print_configuration(args)
    print_commands(args)


if __name__ == "__main__":
    main()
