"""M3: the full mint + press ceremony across two emulated Flexes, with an
untrusted (but honest, here) relay: this test file IS the relay."""

import pytest

from presse_client import (
    Presse,
    apdu_hex,
    split_sw,
    run_pairing,
    confirm_sas_both,
    run_press,
    verify_chain,
    verify_possession,
    INS_COLLECTION,
    INS_GET_ALBUM,
    INS_PRESS_REQUEST,
    INS_PRESS_OFFER,
    SW_SOLD_OUT,
    SW_OK,
)

TITLE = "Nuits Roses"
EDITION = 3


@pytest.fixture
def ceremony(pair):
    a, b = pair
    return Presse(a), Presse(b)


def test_full_ceremony_and_offline_verification(ceremony):
    master, receiver = ceremony

    # Cut the master on A.
    album_cert = master.cut(TITLE, EDITION)
    info_a = master.get_info()
    assert info_a["has_master"] and info_a["counter"] == EDITION

    # Pair through the (honest) relay; humans confirm matching words.
    run_pairing(master, receiver)
    confirm_sas_both(master, receiver)

    # Press 1/EDITION onto B.
    run_press(master, receiver)
    assert master.get_info()["counter"] == EDITION - 1
    info_b = receiver.get_info()
    assert info_b["has_pressing"]

    # Offline verification from B's stored bundle, plus live possession proof.
    pressing_cert = receiver.cmd(0x40, p1=0)
    stored_album = receiver.cmd(0x40, p1=1)
    result = verify_chain(stored_album, pressing_cert, info_b["devpub"])
    assert result == {"title": TITLE, "number": 1, "edition": EDITION}
    verify_possession(receiver, pressing_cert)

    # On-device collection: an album card first (big art, edition line), the
    # detail list on the next page (navigate like a finger would).
    NEXT_PAGE = (430, 550)

    thread, res = master.dev.apdu_async_start(apdu_hex(INS_COLLECTION))
    assert master.dev.wait_for_text("My master, edition of")
    assert master.dev.wait_for_text("left to press")
    master.dev.finger(*NEXT_PAGE)
    assert master.dev.wait_for_text("Still to press")
    assert master.dev.wait_for_text("Pressed 1 of")
    assert master.dev.wait_for_text("for device ")
    master.tap_text("Back")
    thread.join(timeout=30)
    assert split_sw(res["data"])[1] == SW_OK

    thread, res = receiver.dev.apdu_async_start(apdu_hex(INS_COLLECTION))
    assert receiver.dev.wait_for_text("Pressing 1 of")
    assert receiver.dev.wait_for_text(TITLE)
    receiver.dev.finger(*NEXT_PAGE)
    assert receiver.dev.wait_for_text("In my collection")
    receiver.tap_text("Back")
    thread.join(timeout=30)
    assert split_sw(res["data"])[1] == SW_OK


def test_counter_drains_to_sold_out(ceremony):
    master, receiver = ceremony
    master.cut(TITLE, EDITION)
    run_pairing(master, receiver)
    confirm_sas_both(master, receiver)

    album_msg = master.cmd(INS_GET_ALBUM)
    assert len(album_msg) > 0

    # Drain the edition: the receiver requests, the master presses. The
    # receiver only stores the first one; the master's counter doesn't care.
    for expected_number in range(1, EDITION + 1):
        req = receiver.cmd(INS_PRESS_REQUEST)
        cert_mac, sw = master.cmd_gated(INS_PRESS_OFFER, req, "Press this copy", "Press ")
        assert sw == SW_OK
        assert master.get_info()["counter"] == EDITION - expected_number

    # Edition exhausted: the silicon says no.
    req = receiver.cmd(INS_PRESS_REQUEST)
    sw = master.cmd_sw(INS_PRESS_OFFER, req)
    assert sw == SW_SOLD_OUT
    assert master.get_info()["counter"] == 0
