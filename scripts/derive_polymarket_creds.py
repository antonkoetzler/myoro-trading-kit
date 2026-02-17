#!/usr/bin/env python3
"""
Derive Polymarket CLOB API credentials from your private key.
Run from repo root with .env containing PRIVATE_KEY. Prints API_KEY, API_SECRET, API_PASSPHRASE.
Installs py-clob-client and python-dotenv if missing.
"""
import os
import subprocess
import sys

# Load .env from repo root (parent of scripts/)
_repo_root = os.path.join(os.path.dirname(__file__), "..")
try:
    from dotenv import load_dotenv
    load_dotenv(os.path.join(_repo_root, ".env"))
except ImportError:
    pass


def _ensure_deps():
    try:
        from py_clob_client.client import ClobClient  # noqa: F401
    except ImportError:
        subprocess.check_call(
            [sys.executable, "-m", "pip", "install", "py-clob-client", "python-dotenv"],
            cwd=os.path.abspath(_repo_root),
        )


def main():
    _ensure_deps()
    try:
        from dotenv import load_dotenv
        load_dotenv(os.path.join(_repo_root, ".env"))
    except ImportError:
        pass
    from py_clob_client.client import ClobClient

    pk = os.getenv("PRIVATE_KEY") or os.getenv("PK")
    if not pk:
        print("Set PRIVATE_KEY (or PK) in .env", file=sys.stderr)
        sys.exit(1)
    pk = pk.strip()
    if pk.startswith("0x"):
        pk = pk[2:]

    host = "https://clob.polymarket.com"
    chain_id = 137
    client = ClobClient(host, key=pk, chain_id=chain_id)
    creds = client.create_or_derive_api_creds()

    # ApiCreds: .api_key or .key, .secret, .passphrase
    api_key = getattr(creds, "api_key", None) or getattr(creds, "key", None)
    secret = getattr(creds, "secret", None) or getattr(creds, "api_secret", None)
    passphrase = getattr(creds, "passphrase", None) or getattr(creds, "api_passphrase", None)

    print("Add these to your .env:")
    print(f"API_KEY={api_key}")
    print(f"API_SECRET={secret}")
    print(f"API_PASSPHRASE={passphrase}")


if __name__ == "__main__":
    main()
