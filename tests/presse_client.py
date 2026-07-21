"""APDU client + ceremony driver for the presse app.

Independent of the device code on purpose: certificate parsing and signature
verification here re-implement docs/protocol.md (python-ecdsa + hashlib), so a
device bug can't hide behind shared code.
"""

import hashlib
import struct
import time

from ecdsa import VerifyingKey, SECP256k1, BadSignatureError
from ecdsa.util import sigdecode_der

CLA = 0xB5

INS_GET_INFO = 0x01
INS_CUT = 0x10
INS_PAIR_COMMIT = 0x21
INS_PAIR_RESPOND = 0x22
INS_PAIR_REVEAL = 0x23
INS_PAIR_FINISH = 0x24
INS_PAIR_SAS = 0x25
INS_GET_ALBUM = 0x30
INS_PRESS_REQUEST = 0x31
INS_PRESS_OFFER = 0x32
INS_PRESS_LOAD_ALBUM = 0x33
INS_PRESS_ACCEPT = 0x34
INS_GET_BUNDLE = 0x40
INS_CHALLENGE = 0x41

SW_OK = "9000"
SW_SOLD_OUT = "b104"

PUBKEY_LEN = 65
MAC_LEN = 32
ALBUM_PAYLOAD_LEN = 4 + PUBKEY_LEN + 1 + 32 + 2
ALBUM_CERT_LEN = ALBUM_PAYLOAD_LEN + 1 + 72
PRESSING_PAYLOAD_LEN = 4 + 32 + 2 + 2 + PUBKEY_LEN
PRESSING_CERT_LEN = PRESSING_PAYLOAD_LEN + 1 + 72


def apdu_hex(ins: int, data: bytes = b"", p1: int = 0, p2: int = 0) -> str:
    return bytes([CLA, ins, p1, p2, len(data)]).hex() + data.hex()


def split_sw(resp_hex: str):
    return bytes.fromhex(resp_hex[:-4]), resp_hex[-4:]


class Presse:
    """Wraps a SpeculosDevice with presse commands. UI-gated commands take a
    `tap` callable run once the review screen is up."""

    def __init__(self, device):
        self.dev = device

    def cmd(self, ins: int, data: bytes = b"", p1: int = 0) -> bytes:
        resp = self.dev.apdu(apdu_hex(ins, data, p1))
        body, sw = split_sw(resp)
        assert sw == SW_OK, f"{self.dev.name}: INS {ins:#x} returned SW {sw}"
        return body

    def cmd_sw(self, ins: int, data: bytes = b"", p1: int = 0) -> str:
        """Variant returning the status word for error-path tests."""
        _, sw = split_sw(self.dev.apdu(apdu_hex(ins, data, p1)))
        return sw

    def cmd_gated(self, ins: int, data: bytes, button_text: str, wait_text: str):
        """Fire a UI-gated APDU, wait for its review screen, tap the button."""
        thread, result = self.dev.apdu_async_start(apdu_hex(ins, data))
        assert self.dev.wait_for_text(wait_text), (
            f"{self.dev.name}: never saw '{wait_text}': {self.dev.screen_texts()}"
        )
        self.tap_text(button_text)
        thread.join(timeout=30)
        assert "data" in result, f"{self.dev.name}: gated INS {ins:#x} never returned"
        body, sw = split_sw(result["data"])
        return body, sw

    def tap_text(self, needle: str, timeout: float = 10.0):
        deadline = time.time() + timeout
        while time.time() < deadline:
            for e in self.dev.events():
                if needle in e.get("text", "") and "x" in e and "y" in e:
                    self.dev.finger(e["x"], e["y"])
                    return
            time.sleep(0.3)
        raise AssertionError(f"{self.dev.name}: no tappable '{needle}'")

    # --- high-level ceremony steps ---

    def get_info(self):
        body = self.cmd(INS_GET_INFO)
        flags = body[0]
        devpub = body[1 : 1 + PUBKEY_LEN]
        edition, counter = struct.unpack_from("<HH", body, 1 + PUBKEY_LEN)
        title_len = body[1 + PUBKEY_LEN + 4]
        title = body[1 + PUBKEY_LEN + 5 : 1 + PUBKEY_LEN + 5 + title_len].decode()
        return {
            "has_master": bool(flags & 1),
            "has_pressing": bool(flags & 2),
            "devpub": devpub,
            "edition": edition,
            "counter": counter,
            "title": title,
        }

    def cut(self, title: str, edition: int) -> bytes:
        data = struct.pack("<H", edition) + title.encode()
        body, sw = self.cmd_gated(INS_CUT, data, "Cut the master", "Cut master")
        assert sw == SW_OK, f"cut failed: {sw}"
        assert len(body) == ALBUM_CERT_LEN
        return body


def run_pairing(master: Presse, receiver: Presse):
    """Happy-path pairing up to (but not including) the SAS taps."""
    commitment = master.cmd(INS_PAIR_COMMIT)
    assert len(commitment) == 32
    eb = receiver.cmd(INS_PAIR_RESPOND, commitment)
    assert len(eb) == PUBKEY_LEN
    ea = master.cmd(INS_PAIR_REVEAL, eb)
    assert len(ea) == PUBKEY_LEN
    receiver.cmd(INS_PAIR_FINISH, ea)
    return ea, eb


def confirm_sas_both(master: Presse, receiver: Presse):
    """Fire PAIR_SAS on both devices, assert the words match on both screens,
    tap both. Returns the SAS bytes of each device."""
    tm, rm = master.dev.apdu_async_start(apdu_hex(INS_PAIR_SAS))
    tr, rr = receiver.dev.apdu_async_start(apdu_hex(INS_PAIR_SAS))
    assert master.dev.wait_for_text("Words match")
    assert receiver.dev.wait_for_text("Words match")

    words_m = sas_words_on_screen(master.dev)
    words_r = sas_words_on_screen(receiver.dev)
    assert words_m == words_r, f"SAS mismatch on screens: {words_m} vs {words_r}"
    assert len(words_m) == 4

    master.tap_text("Words match")
    receiver.tap_text("Words match")
    tm.join(timeout=30)
    tr.join(timeout=30)
    sas_m, sw_m = split_sw(rm["data"])
    sas_r, sw_r = split_sw(rr["data"])
    assert sw_m == SW_OK and sw_r == SW_OK
    assert sas_m == sas_r, "devices derived different SAS bytes"
    return sas_m


def sas_words_on_screen(dev) -> list:
    """The SAS message is a 4-line text block; OCR may deliver it as one
    event or per-line. Find the block adjacent to the instruction text."""
    texts = dev.screen_texts()
    for t in texts:
        parts = [w for w in t.replace("\n", " ").split(" ") if w]
        if len(parts) == 4 and all(w.isalpha() and w.islower() for w in parts):
            return parts
    # Per-line fallback: four consecutive lowercase single-word events.
    words = []
    for t in texts:
        w = t.strip()
        if w.isalpha() and w.islower():
            words.append(w)
        elif words:
            if len(words) >= 4:
                break
            words = []
    return words[-4:] if len(words) >= 4 else words


def run_press(master: Presse, receiver: Presse) -> bytes:
    """One full press onto the receiver. Returns the PressingCert."""
    album_msg = master.cmd(INS_GET_ALBUM)
    req = receiver.cmd(INS_PRESS_REQUEST)
    cert_mac, sw = master.cmd_gated(INS_PRESS_OFFER, req, "Press this copy", "Press ")
    assert sw == SW_OK, f"press offer failed: {sw}"
    receiver.cmd(INS_PRESS_LOAD_ALBUM, album_msg)
    _, sw = receiver.cmd_gated(INS_PRESS_ACCEPT, cert_mac, "Receive it", "Receive ")
    assert sw == SW_OK, f"press accept failed: {sw}"
    return cert_mac[:PRESSING_CERT_LEN]


# --- independent verification (no device code, no session secrets) ---


def parse_album_cert(cert: bytes):
    assert len(cert) == ALBUM_CERT_LEN and cert[:4] == b"PRA1"
    albpub = cert[4 : 4 + PUBKEY_LEN]
    title_len = cert[69]
    title = cert[70 : 70 + title_len].decode()
    edition = struct.unpack_from("<H", cert, 102)[0]
    sig_len = cert[ALBUM_PAYLOAD_LEN]
    sig = cert[ALBUM_PAYLOAD_LEN + 1 : ALBUM_PAYLOAD_LEN + 1 + sig_len]
    return albpub, title, edition, sig, cert[:ALBUM_PAYLOAD_LEN]


def parse_pressing_cert(cert: bytes):
    assert len(cert) == PRESSING_CERT_LEN and cert[:4] == b"PRP1"
    album_id = cert[4:36]
    number, edition = struct.unpack_from("<HH", cert, 36)
    recvpub = cert[40 : 40 + PUBKEY_LEN]
    sig_len = cert[PRESSING_PAYLOAD_LEN]
    sig = cert[PRESSING_PAYLOAD_LEN + 1 : PRESSING_PAYLOAD_LEN + 1 + sig_len]
    return album_id, number, edition, recvpub, sig, cert[:PRESSING_PAYLOAD_LEN]


def ecdsa_verify(pubkey_uncompressed: bytes, payload: bytes, sig_der: bytes) -> bool:
    vk = VerifyingKey.from_string(pubkey_uncompressed, curve=SECP256k1)
    digest = hashlib.sha256(payload).digest()
    try:
        return vk.verify_digest(sig_der, digest, sigdecode=sigdecode_der)
    except BadSignatureError:
        return False


def verify_chain(album_cert: bytes, pressing_cert: bytes, holder_devpub: bytes) -> dict:
    """Full offline verification: album self-signature, pressing signature,
    album_id linkage, device binding, number sanity."""
    albpub, title, edition, alb_sig, alb_payload = parse_album_cert(album_cert)
    assert ecdsa_verify(albpub, alb_payload, alb_sig), "album cert signature invalid"

    album_id, number, p_edition, recvpub, p_sig, p_payload = parse_pressing_cert(pressing_cert)
    assert ecdsa_verify(albpub, p_payload, p_sig), "pressing cert signature invalid"
    assert album_id == hashlib.sha256(albpub).digest(), "album_id mismatch"
    assert p_edition == edition, "edition mismatch between certs"
    assert 1 <= number <= edition, "pressing number out of range"
    assert recvpub == holder_devpub, "pressing not bound to this device"
    return {"title": title, "number": number, "edition": edition}


def verify_possession(presse: Presse, pressing_cert: bytes):
    """Challenge-response: the device proves it holds the bound key, live."""
    import os

    _, _, _, recvpub, _, _ = parse_pressing_cert(pressing_cert)
    nonce = os.urandom(32)
    body = presse.cmd(INS_CHALLENGE, nonce)
    sig_len = body[0]
    sig = body[1 : 1 + sig_len]
    assert ecdsa_verify(recvpub, b"presse-verify" + nonce, sig), "challenge signature invalid"
