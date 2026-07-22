"""M5: the library landing screen, the record card, and the sleeve hash bound
into the album certificate.

The library is now the app's home screen. These tests drive it the way a
finger would (Speculos touch events located by OCR) and check the parts that
can be asserted without pixel comparison: the text the certificate drives, the
navigation, and -- cryptographically, through the independent verifier -- that
the sleeve is part of the signed identity of the edition."""

import struct

import pytest

from presse_client import (
    Presse,
    apdu_hex,
    split_sw,
    parse_album_cert,
    verify_sleeve,
    upload_art,
    run_pairing,
    confirm_sas_both,
    run_press,
    ART_LEN,
    INS_COLLECTION,
    SLEEVE_HASH_LEN,
    SW_OK,
)

TITLE = "Nocturne"
EDITION = 5


def a_sleeve(seed: int = 0) -> bytes:
    """A deterministic packed sleeve. The device only ever hashes these bytes,
    so the content is irrelevant; determinism keeps the test reproducible."""
    return bytes(((i * 7 + seed) & 0xFF) for i in range(ART_LEN))


# --- the library landing screen -----------------------------------------


def test_library_empty_state(device):
    """A fresh device opens on the library with the empty-state message."""
    assert device.wait_for_text("Enclave Records"), device.screen_texts()
    assert device.wait_for_text("No records yet"), device.screen_texts()


def test_library_lists_the_master_after_cut(device):
    """After a cut the library redraws to show the record: title from the
    certificate, plus a status line."""
    p = Presse(device)
    p.cut(TITLE, EDITION)
    assert device.wait_for_text(TITLE), device.screen_texts()
    assert device.wait_for_text("left to press"), device.screen_texts()


def test_tapping_a_row_opens_the_record_card(device):
    """Tapping the record row opens its card (the full-size sleeve screen with
    the edition line), and Back returns to the library."""
    p = Presse(device)
    p.cut(TITLE, EDITION)
    assert device.wait_for_text(TITLE)
    p.tap_text(TITLE)
    assert device.wait_for_text("My master, edition of"), device.screen_texts()
    p.tap_text("Back")
    # Back on the library: the row is shown again.
    assert device.wait_for_text(TITLE), device.screen_texts()


def test_record_title_comes_from_the_certificate(device):
    """The title shown on the card is the certificate's, not baked into the
    bitmap: it survives whatever art (or none) is uploaded."""
    p = Presse(device)
    upload_art(p, a_sleeve())
    album_cert = p.cut(TITLE, EDITION)
    _, cert_title, _, _, _, _ = parse_album_cert(album_cert)
    assert cert_title == TITLE
    p.tap_text(TITLE)
    assert device.wait_for_text(TITLE), device.screen_texts()
    p.tap_text("Back")


# --- the library redraws only when it could have changed ----------------


def test_bulk_art_upload_leaves_the_library_intact(device):
    """A full sleeve is ~50 SET_ART chunks. Streamed while the library is the
    screen on display, none of them disturbs the record it lists nor stalls the
    device: the library yields to each chunk but does not repaint per chunk (the
    on-hardware flicker/slowness this gating removes). Correctness is what we can
    assert here; the absence of a repaint is confirmed by screenshot on device."""
    p = Presse(device)
    p.cut(TITLE, EDITION)
    assert device.wait_for_text(TITLE), device.screen_texts()

    upload_art(p, a_sleeve())  # the burst, served with the library on screen

    # The library still lists the record, unchanged, and the device is still
    # serving commands: the burst neither corrupted the screen nor wedged it.
    assert device.wait_for_text(TITLE), device.screen_texts()
    assert device.wait_for_text("left to press"), device.screen_texts()
    assert p.get_info()["title"] == TITLE


def test_press_repaints_the_receiver_with_the_real_sleeve(pair):
    """The receiver repaints its library only when the pressing lands. Carrying
    the sleeve across before PRESS_ACCEPT means that single repaint shows the
    real, hash-verified cover, never the generative placeholder: the record card
    reports the sleeve "Verified"."""
    a, b = pair
    master, receiver = Presse(a), Presse(b)

    art = a_sleeve()
    upload_art(master, art)  # sealed into the cut's signed sleeve hash
    master.cut(TITLE, EDITION)
    run_pairing(master, receiver)
    confirm_sas_both(master, receiver)
    run_press(master, receiver, carry_from=master)

    # B's library repainted on accept and now lists the pressing.
    assert b.wait_for_text(TITLE), b.screen_texts()

    thread, res = b.apdu_async_start(apdu_hex(INS_COLLECTION))
    assert b.wait_for_text("Pressing 1 of"), b.screen_texts()
    b.finger(430, 550)  # swipe to the provenance page
    # The carried bytes hash to the sealed sleeve hash, so B vouches for them.
    assert b.wait_for_text("Verified"), b.screen_texts()
    receiver.tap_text("Back")
    thread.join(timeout=30)
    assert split_sw(res["data"])[1] == SW_OK


# --- the sleeve hash inside the album certificate -----------------------


def test_cut_binds_the_uploaded_sleeve_hash(device):
    """Art uploaded before the cut is hashed into the signed certificate: a
    third party can confirm the sleeve bytes against it, and a single flipped
    byte fails."""
    p = Presse(device)
    art = a_sleeve()
    upload_art(p, art)
    album_cert = p.cut(TITLE, EDITION)

    assert verify_sleeve(album_cert, art), "genuine sleeve must verify"

    tampered = bytearray(art)
    tampered[0] ^= 0x01
    assert not verify_sleeve(album_cert, bytes(tampered)), "tampered sleeve must fail"


def test_cut_without_art_binds_no_sleeve(device):
    """With nothing uploaded, the cut records the all-zero sentinel: the
    edition is signed as having no sleeve, and verification of any bytes
    against it fails."""
    p = Presse(device)
    album_cert = p.cut(TITLE, EDITION)
    _, _, _, sleeve_hash, _, _ = parse_album_cert(album_cert)
    assert sleeve_hash == b"\x00" * SLEEVE_HASH_LEN
    assert not verify_sleeve(album_cert, a_sleeve())


def test_mismatched_art_still_renders_the_record(device):
    """A device holding art whose hash does not match the certificate must
    fall back to generative art, not error: the record screen still opens and
    shows the title. (The fallback itself is visual; here we assert the flow
    does not fail closed on a benign mismatch.)"""
    p = Presse(device)
    upload_art(p, a_sleeve(seed=1))
    p.cut(TITLE, EDITION)
    # Overwrite the art so its hash no longer matches the sealed certificate.
    upload_art(p, a_sleeve(seed=99))

    thread, result = device.apdu_async_start(apdu_hex(INS_COLLECTION))
    assert device.wait_for_text("My master, edition of"), device.screen_texts()
    p.tap_text("Back")
    thread.join(timeout=30)
    assert split_sw(result["data"])[1] == SW_OK


# --- the ceremony still works with the library on screen ----------------


def test_ceremony_runs_with_the_library_as_home(pair):
    """The cut is a UI-gated APDU that arrives while the library is the screen
    on display. It must yield, run the review, and leave the master recorded:
    the whole reason the library runs an APDU-aware event loop."""
    a, b = pair
    master = Presse(a)
    assert a.wait_for_text("No records yet"), a.screen_texts()

    album_cert = master.cut(TITLE, EDITION)
    assert len(album_cert) > 0
    info = master.get_info()
    assert info["has_master"] and info["title"] == TITLE
    # The library redrew from fresh NVM and now lists the record.
    assert a.wait_for_text(TITLE), a.screen_texts()
