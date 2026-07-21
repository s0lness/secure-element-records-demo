"""Minimal Ledger HID transport, written against the documented framing
(64-byte reports, channel 0x0101, tag 0x05) so we can hold TWO devices open
at once: the stock clients all assume a single dongle.

Exposes the same .apdu(hex) -> hex interface as tests/conftest.SpeculosDevice,
so presse_client.Presse drives real hardware unchanged. UI-gated commands
simply block until the human taps the actual screen."""

import struct

import hid

LEDGER_VID = 0x2C97
CHANNEL = 0x0101
TAG = 0x05
PACKET_SIZE = 64


def enumerate_ledgers():
    """Return HID paths for connected Ledger admin interfaces."""
    paths = []
    for info in hid.enumerate(LEDGER_VID, 0):
        # The APDU interface: usage_page 0xffa0 on Windows/macOS backends,
        # interface 0 on Linux hidraw.
        if info.get("usage_page") == 0xFFA0 or info.get("interface_number") == 0:
            paths.append(info["path"])
    return sorted(set(paths))


class HidDevice:
    def __init__(self, name: str, path: bytes):
        self.name = name
        self.dev = hid.device()
        self.dev.open_path(path)
        self.dev.set_nonblocking(False)

    def apdu(self, hexstr: str) -> str:
        data = bytes.fromhex(hexstr)
        self._write(data)
        resp = self._read()
        return resp.hex()

    def _write(self, apdu: bytes):
        payload = struct.pack(">H", len(apdu)) + apdu
        seq = 0
        offset = 0
        while offset < len(payload):
            header = struct.pack(">HBH", CHANNEL, TAG, seq)
            chunk = payload[offset : offset + PACKET_SIZE - len(header)]
            packet = header + chunk
            packet += b"\x00" * (PACKET_SIZE - len(packet))
            self.dev.write(b"\x00" + packet)
            offset += len(chunk)
            seq += 1

    def _read(self) -> bytes:
        expected_len = None
        buf = b""
        seq = 0
        while expected_len is None or len(buf) < expected_len:
            packet = bytes(self.dev.read(PACKET_SIZE, timeout_ms=0))
            if not packet:
                continue
            channel, tag, rseq = struct.unpack(">HBH", packet[:5])
            if channel != CHANNEL or tag != TAG or rseq != seq:
                raise IOError(f"{self.name}: bad HID frame (ch={channel:#x} tag={tag} seq={rseq})")
            body = packet[5:]
            if seq == 0:
                expected_len = struct.unpack(">H", body[:2])[0]
                body = body[2:]
            buf += body
            seq += 1
        return buf[:expected_len]

    def close(self):
        self.dev.close()
