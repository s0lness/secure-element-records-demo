"""M4: the relay turns hostile. Key substitution, tampering, replay,
commitment cheating, grinding caps: every attack must die at the layer
designed to kill it."""

import pytest

from mitm import MitmEndpoint
from presse_client import (
    Presse,
    apdu_hex,
    split_sw,
    run_pairing,
    confirm_sas_both,
    sas_words_on_screen,
    PRESSING_CERT_LEN,
    INS_PAIR_COMMIT,
    INS_PAIR_RESPOND,
    INS_PAIR_REVEAL,
    INS_PAIR_FINISH,
    INS_PAIR_SAS,
    INS_GET_ALBUM,
    INS_PRESS_REQUEST,
    INS_PRESS_OFFER,
    INS_PRESS_LOAD_ALBUM,
    INS_PRESS_ACCEPT,
    SW_OK,
)

SW_DENY = "6985"
SW_BAD_STATE = "b101"
SW_BAD_MAC = "b102"
SW_BAD_CERT = "b103"
SW_TOO_MANY = "b109"

TITLE = "Nuits Roses"
EDITION = 3


@pytest.fixture
def ceremony(pair):
    a, b = pair
    return Presse(a), Presse(b)


def full_mitm(master: Presse, receiver: Presse):
    """Runs the two half-pairings and returns both MITM endpoints."""
    to_a = MitmEndpoint()
    to_b = MitmEndpoint()

    # Side A: real master, fake receiver.
    master.cmd(INS_PAIR_COMMIT)
    ea = master.cmd(INS_PAIR_REVEAL, to_a.pub)
    to_a.derive(ea, as_master=False)

    # Side B: fake master, real receiver.
    eb = receiver.cmd(INS_PAIR_RESPOND, to_b.commitment())
    receiver.cmd(INS_PAIR_FINISH, to_b.pub)
    to_b.derive(eb, as_master=True)

    return to_a, to_b


def test_mitm_produces_different_sas_words(ceremony):
    """The core security claim: a key-substituting relay cannot make the two
    screens agree. Vigilant humans abort; the session dies."""
    master, receiver = ceremony
    to_a, to_b = full_mitm(master, receiver)

    # The words each device is about to show are already determined.
    assert to_a.sas != to_b.sas, "MITM accidentally matched 4-byte SAS (p=2^-32)"

    tm, _ = master.dev.apdu_async_start(apdu_hex(INS_PAIR_SAS))
    tr, _ = receiver.dev.apdu_async_start(apdu_hex(INS_PAIR_SAS))
    assert master.dev.wait_for_text("Words match")
    assert receiver.dev.wait_for_text("Words match")

    words_a = sas_words_on_screen(master.dev)
    words_b = sas_words_on_screen(receiver.dev)
    assert len(words_a) == 4 and len(words_b) == 4
    assert words_a != words_b, "screens agree under MITM: SAS is broken"

    # The humans do their job.
    master.tap_text("Abort")
    receiver.tap_text("Abort")
    tm.join(timeout=30)
    tr.join(timeout=30)

    # Aborted sessions are dead: no MACed traffic possible.
    assert master.cmd_sw(INS_GET_ALBUM) == SW_BAD_STATE
    assert receiver.cmd_sw(INS_PRESS_REQUEST) == SW_BAD_STATE


def test_mitm_tampered_cert_dies_on_signature(ceremony):
    """Even if both humans are fooled into confirming mismatched words, a
    tampered pressing (rebound to another device key) still dies: the
    certificate signature covers the receiver key and the MITM does not hold
    the album key."""
    master, receiver = ceremony
    album_cert = master.cut(TITLE, EDITION)
    to_a, to_b = full_mitm(master, receiver)

    # Fooled humans confirm on both devices.
    for p in (master, receiver):
        t, r = p.dev.apdu_async_start(apdu_hex(INS_PAIR_SAS))
        assert p.dev.wait_for_text("Words match")
        p.tap_text("Words match")
        t.join(timeout=30)
        assert split_sw(r["data"])[1] == SW_OK

    # MITM requests a pressing from the master for ITS OWN key.
    req_payload = to_a.pub
    req = req_payload + to_a.mac_send(INS_PRESS_REQUEST, req_payload)
    cert_mac, sw = master.cmd_gated(INS_PRESS_OFFER, req, "Press this copy", "Press ")
    assert sw == SW_OK
    cert = cert_mac[:PRESSING_CERT_LEN]
    assert to_a.mac_verify(INS_PRESS_OFFER, cert, cert_mac[PRESSING_CERT_LEN:])

    # Relay the album to B, re-MACed with B's session key.
    album_payload = master.cmd(INS_GET_ALBUM)[: len(album_cert)]
    receiver.cmd(INS_PRESS_LOAD_ALBUM, album_payload + to_b.mac_send(INS_GET_ALBUM, album_payload))

    # Forward the pressing to B, re-MACed. The cert is bound to the MITM's
    # key, not B's: B must refuse on certificate grounds (not MAC grounds).
    forged = cert + to_b.mac_send(INS_PRESS_OFFER, cert)
    sw = receiver.cmd_sw(INS_PRESS_ACCEPT, forged)
    assert sw == SW_BAD_CERT

    # Crude rebinding (patch the pubkey bytes inside the cert) dies the same
    # way, because the signature covers the receiver key.
    info_b = receiver.get_info()
    patched = bytearray(cert)
    patched[40 : 40 + 65] = info_b["devpub"]
    patched = bytes(patched)
    forged2 = patched + to_b.mac_send(INS_PRESS_OFFER, patched)
    sw = receiver.cmd_sw(INS_PRESS_ACCEPT, forged2)
    assert sw == SW_BAD_CERT


def test_commitment_cheating_is_caught(ceremony):
    """A master (or relay) revealing an ephemeral that doesn't match the
    commitment is rejected by the receiver: the anti-grinding backbone."""
    master, receiver = ceremony
    commitment = master.cmd(INS_PAIR_COMMIT)
    receiver.cmd(INS_PAIR_RESPOND, commitment)
    liar = MitmEndpoint()
    sw = receiver.cmd_sw(INS_PAIR_FINISH, liar.pub)
    assert sw == SW_BAD_MAC


def test_replayed_pressing_is_rejected(ceremony):
    """Replaying the same MACed pressing message is killed by the sequence
    counter, and a second accept by the one-pressing-per-device rule."""
    master, receiver = ceremony
    master.cut(TITLE, EDITION)
    run_pairing(master, receiver)
    confirm_sas_both(master, receiver)

    album_msg = master.cmd(INS_GET_ALBUM)
    req = receiver.cmd(INS_PRESS_REQUEST)
    cert_mac, sw = master.cmd_gated(INS_PRESS_OFFER, req, "Press this copy", "Press ")
    assert sw == SW_OK
    receiver.cmd(INS_PRESS_LOAD_ALBUM, album_msg)
    _, sw = receiver.cmd_gated(INS_PRESS_ACCEPT, cert_mac, "Receive it", "Receive ")
    assert sw == SW_OK

    # Replay: the receiver's recv_seq has moved on; the stale MAC is dead.
    assert receiver.cmd_sw(INS_PRESS_ACCEPT, cert_mac) == SW_BAD_MAC


def test_pairing_attempts_are_capped(ceremony):
    """Silent online SAS-grinding is bounded: the per-boot attempt counter
    shuts the door long before 2^32 tries."""
    master, _ = ceremony
    for _ in range(8):
        body, sw = split_sw(master.dev.apdu(apdu_hex(INS_PAIR_COMMIT)))
        assert sw == SW_OK
    assert master.cmd_sw(INS_PAIR_COMMIT) == SW_TOO_MANY
