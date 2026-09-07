#!/usr/bin/env python3
"""Generate the Ed25519 key pair used by MSC application updates.

The key material is printed once and never written to the repository. Put the
private value in the GitHub Actions secret and the public value in the GitHub
Actions repository variable described in the release documentation.
"""

from __future__ import annotations

import secrets


Q = 2**255 - 19


def inv(value: int) -> int:
    return pow(value, Q - 2, Q)


def xrecover(y: int) -> int:
    xx = (y * y - 1) * inv(121665 * y * y + 1)
    x = pow(xx, (Q + 3) // 8, Q)
    if (x * x - xx) % Q:
        x = (x * 19681161315388985) % Q
    if x % 2:
        x = Q - x
    return x


BY = 4 * inv(5) % Q
BX = xrecover(BY)
B = (BX, BY)


def edwards_add(point_a: tuple[int, int], point_b: tuple[int, int]) -> tuple[int, int]:
    x1, y1 = point_a
    x2, y2 = point_b
    d = -121665 * inv(121666) % Q
    denominator_x = inv(1 + d * x1 * x2 * y1 * y2)
    denominator_y = inv(1 - d * x1 * x2 * y1 * y2)
    return (
        (x1 * y2 + x2 * y1) * denominator_x % Q,
        (y1 * y2 + x1 * x2) * denominator_y % Q,
    )


def scalar_mult(point: tuple[int, int], scalar: int) -> tuple[int, int]:
    result = (0, 1)
    addend = point
    while scalar:
        if scalar & 1:
            result = edwards_add(result, addend)
        addend = edwards_add(addend, addend)
        scalar >>= 1
    return result


def encode_point(point: tuple[int, int]) -> bytes:
    x, y = point
    return (y | ((x & 1) << 255)).to_bytes(32, "little")


def public_key(seed: bytes) -> bytes:
    import hashlib

    digest = hashlib.sha512(seed).digest()
    scalar = int.from_bytes(digest[:32], "little")
    scalar &= (1 << 254) - 8
    scalar |= 1 << 254
    return encode_point(scalar_mult(B, scalar))


def main() -> int:
    seed = secrets.token_bytes(32)
    print("Copy these values into GitHub Actions. They are not saved to disk:")
    print(f"MSC2_RELEASE_SIGNING_KEY_HEX={seed.hex()}")
    print(f"MSC2_RELEASE_PUBLIC_KEY_HEX={public_key(seed).hex()}")
    print()
    print("Keep MSC2_RELEASE_SIGNING_KEY_HEX private. The public value is safe to ship.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
