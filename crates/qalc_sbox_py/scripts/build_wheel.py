#!/usr/bin/env python3
"""Assemble the qalc_sbox_py `dist/` tree into a single installable wheel.

Run inside the builder container (see `xtask package qalc-py`) so the ABI and
platform tags match the interpreter the `.so` was compiled against. Uses only
the Python standard library.

`dist/` layout (produced by `xtask build qalc-py`):

    dist/qalc_sbox_py.so        compiled extension (module `qalc_sbox_py`)
    dist/qalc_sbox_py/          stub package tree (.pyi + generated .py)

The extension is placed into the wheel as the package initialiser
(`qalc_sbox_py/__init__<EXT_SUFFIX>`), so `import qalc_sbox_py` loads the
compiled module while the sibling `.pyi` files describe it (mixed layout).
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import sysconfig
import zipfile
from pathlib import Path


def wheel_tags() -> tuple[str, str, str]:
    """Return (python_tag, abi_tag, platform_tag) for the running interpreter."""
    import sys

    py = f"cp{sys.version_info.major}{sys.version_info.minor}"
    soabi = sysconfig.get_config_var("SOABI") or ""
    # SOABI is e.g. "cpython-313-x86_64-linux-gnu"; the wheel ABI tag is "cp313".
    parts = soabi.split("-")
    abi = f"cp{parts[1]}" if len(parts) >= 2 and parts[0] == "cpython" else py
    plat = sysconfig.get_platform().replace("-", "_").replace(".", "_")
    return py, abi, plat


def record_line(arcname: str, data: bytes) -> str:
    digest = base64.urlsafe_b64encode(hashlib.sha256(data).digest())
    digest = digest.rstrip(b"=").decode("ascii")
    return f"{arcname},sha256={digest},{len(data)}"


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--dist", required=True, type=Path, help="dist/ directory")
    ap.add_argument("--out", required=True, type=Path, help="output directory")
    ap.add_argument("--name", required=True, help="distribution name")
    ap.add_argument("--version", required=True, help="distribution version")
    args = ap.parse_args()

    ext_suffix = sysconfig.get_config_var("EXT_SUFFIX")  # e.g. .cpython-313-...so
    py_tag, abi_tag, plat_tag = wheel_tags()

    so_src = args.dist / "qalc_sbox_py.so"
    stub_src = args.dist / "qalc_sbox_py"
    if not so_src.is_file():
        raise SystemExit(f"missing compiled module: {so_src}")
    if not stub_src.is_dir():
        raise SystemExit(f"missing stub package: {stub_src}")

    # (arcname, bytes) for every payload file, in a stable order. Only the
    # `.pyi` type stubs are shipped: the sibling `.py` files generated for the
    # `tracing_subscriber` submodule would shadow the real module that the
    # extension registers at runtime via `add_submodule` (and don't re-export
    # its members), so importing them breaks `Tracing` et al.
    payload: list[tuple[str, bytes]] = []
    for path in sorted(stub_src.rglob("*.pyi")):
        rel = path.relative_to(stub_src).as_posix()
        payload.append((f"{args.name}/{rel}", path.read_bytes()))
    # The extension, loaded as the package initialiser.
    payload.append((f"{args.name}/__init__{ext_suffix}", so_src.read_bytes()))
    # PEP 561 marker so type checkers pick up the shipped stubs.
    payload.append((f"{args.name}/py.typed", b""))

    dist_info = f"{args.name}-{args.version}.dist-info"
    metadata = (
        "Metadata-Version: 2.1\n"
        f"Name: {args.name}\n"
        f"Version: {args.version}\n"
    ).encode()
    wheel_meta = (
        "Wheel-Version: 1.0\n"
        "Generator: sadanzip-xtask\n"
        "Root-Is-Purelib: false\n"
        f"Tag: {py_tag}-{abi_tag}-{plat_tag}\n"
    ).encode()
    payload.append((f"{dist_info}/METADATA", metadata))
    payload.append((f"{dist_info}/WHEEL", wheel_meta))

    record_path = f"{dist_info}/RECORD"
    lines = [record_line(name, data) for name, data in payload]
    lines.append(f"{record_path},,")  # RECORD itself is unhashed.
    record = ("\n".join(lines) + "\n").encode()

    args.out.mkdir(parents=True, exist_ok=True)
    wheel_name = f"{args.name}-{args.version}-{py_tag}-{abi_tag}-{plat_tag}.whl"
    wheel_path = args.out / wheel_name
    if wheel_path.exists():
        wheel_path.unlink()
    with zipfile.ZipFile(wheel_path, "w", zipfile.ZIP_DEFLATED) as zf:
        for name, data in payload:
            zf.writestr(name, data)
        zf.writestr(record_path, record)

    print(wheel_path)


if __name__ == "__main__":
    main()
