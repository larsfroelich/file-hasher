import argparse


def parse_cmd_args():
    parser = argparse.ArgumentParser(description="Toolbox for hashing files and verifying hashed files")

    parser.add_argument("path", help="The folder- or filepath to perform operations in")

    action_argument = parser.add_mutually_exclusive_group(required=True)
    action_argument.add_argument("--hash", action='store_true')
    action_argument.add_argument("--verify", action='store_true')

    parser.add_argument("--hash-length", type=int, default=12)
    parser.add_argument("--hash-algorithm", choices=["sha256"], default="sha256")

    return parser.parse_args()
