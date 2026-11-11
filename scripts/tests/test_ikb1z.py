"""Protocol properties, C/Python parity and Maix transport failure/concurrency tests."""
import ctypes
import importlib.util
import os
from pathlib import Path
import threading
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("ikb1z_driver", ROOT / "knowledge_base/drivers/ikb1z.py")
ikb = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ikb)


class Bus:
    def __init__(self):
        self.events = []
        self.written = None
        self.reply = b"\x02\xab"

    def writeto(self, address, payload):
        self.events.append(("write", address, payload))
        return len(payload) if self.written is None else self.written

    def readfrom(self, address, length):
        self.events.append(("read", address, length))
        return self.reply


class DriverTests(unittest.TestCase):
    def setUp(self):
        self.bus = Bus()
        self.lock = threading.Lock()
        self.transport = ikb.MaixI2CTransport(self.bus, self.lock, 100)
        self.device = ikb.IKB1Z(self.transport, 0x49)

    def test_initialization_has_no_io_and_validates_config(self):
        self.assertEqual(self.bus.events, [])
        for address in (0x20, 0x90, 0x47, 0x4A, True, 72.0):
            with self.assertRaises(ValueError):
                ikb.IKB1Z(self.transport, address)
        for timeout in (0, -1, True, 0.5):
            with self.assertRaises(ValueError):
                ikb.MaixI2CTransport(self.bus, self.lock, timeout)

    def test_reference_vectors_and_onboard_masks(self):
        self.device.motor(1, 50)
        self.device.motor(2, -50)
        self.device.motor(1, 0)
        self.device.stop()
        self.device.servo(3, 100)
        self.device.output(7, True)
        self.device.reset()
        self.assertEqual([event[2] for event in self.bus.events], [
            b"\x21\x32", b"\x22\xce", b"\x21\x00", b"\x23\x00",
            b"\x43\x64", b"\x0f\x01", b"\x00"])

    def test_gated_capabilities_and_invalid_inputs_send_no_command(self):
        for action in (lambda: self.device.motor(4, 50), lambda: self.device.motor(15, 0),
                       lambda: self.device.servo_free(1), lambda: self.device.servo(1, -1),
                       lambda: self.device.uart_read(1)):
            with self.assertRaises(NotImplementedError):
                action()
        for action in (lambda: self.device.motor(0, 0), lambda: self.device.motor(1, 101),
                       lambda: self.device.output(8, True), lambda: self.device.output(0, 1),
                       lambda: self.device.input(0, 1), lambda: self.device.adc10(-1),
                       lambda: self.device.servo(64, 100), lambda: self.device.servo(1, 201)):
            with self.assertRaises(ValueError):
                action()
        self.assertEqual(self.bus.events, [])

    def test_adc10_write_then_read_with_unshifted_address_and_exact_bytes(self):
        self.assertEqual(self.device.adc10(7), 683)
        self.assertEqual(self.bus.events, [("write", 0x49, b"\xf0"), ("read", 0x49, 2)])
        self.assertFalse(self.lock.locked())
        for response in (None, b"", b"\x01", b"\x00\x01\x02", b"\x04\x00"):
            self.bus.reply = response
            with self.assertRaises(OSError):
                self.device.adc10(0)
            self.assertFalse(self.lock.locked())

    def test_digital_adc_scales_are_validated_separately(self):
        self.bus.reply = b"\x01"
        self.assertEqual(self.device.input(3, True), 1)
        self.assertEqual(self.bus.events[0][2], b"\x0b\x03")
        self.bus.reply = b"\xff"
        self.assertEqual(self.device.adc8(0), 255)
        for read in (self.device.input, self.device.adc_percent):
            with self.assertRaises(OSError):
                read(0)

    def test_short_or_failed_writes_are_not_retried_and_never_read(self):
        for written in (-1, 0, 2, True):
            self.bus.events.clear()
            self.bus.written = written
            with self.assertRaises(OSError):
                self.device.adc10(0)  # command is one byte
            self.assertEqual(len(self.bus.events), 1)
            self.assertFalse(self.lock.locked())

    def test_lock_wait_and_late_native_results_are_timeouts(self):
        self.transport.timeout = 0.001
        self.lock.acquire()
        with self.assertRaises(TimeoutError):
            self.device.adc10(0)
        self.lock.release()
        self.assertEqual(self.bus.events, [])
        self.transport.timeout = 1
        with patch.object(ikb.time, "monotonic", side_effect=[0, 0, 1]):
            with self.assertRaises(TimeoutError):
                self.device.adc10(0)
        self.assertEqual(self.bus.events, [("write", 0x49, b"\x80")])
        self.assertFalse(self.lock.locked())

    def test_native_exceptions_release_shared_lock_without_retry(self):
        with patch.object(self.bus, "writeto", side_effect=OSError("NACK")) as write:
            with self.assertRaisesRegex(OSError, "NACK"):
                self.device.motor(1, 50)
            self.assertEqual(write.call_count, 1)
        self.assertFalse(self.lock.locked())

    def test_two_devices_cannot_interleave_command_response_pairs(self):
        events = []
        class SlowBus:
            def writeto(self, address, data):
                events.append(("write", address, data))
                time.sleep(0.005)
                return len(data)
            def readfrom(self, address, length):
                events.append(("read", address, length))
                return b"\x01"
        bus, lock = SlowBus(), threading.Lock()
        devices = [ikb.IKB1Z(ikb.MaixI2CTransport(bus, lock, 1000), address) for address in (0x48, 0x49)]
        errors = []
        def run(device):
            try:
                for _ in range(10):
                    self.assertEqual(device.input(0), 1)
            except Exception as error:
                errors.append(error)
        workers = [threading.Thread(target=run, args=(device,)) for device in devices]
        for worker in workers: worker.start()
        for worker in workers: worker.join(timeout=5)
        self.assertFalse(any(worker.is_alive() for worker in workers))
        self.assertEqual(errors, [])
        self.assertEqual(len(events), 40)
        for write, read in zip(events[::2], events[1::2]):
            self.assertEqual((write[0], read[0], write[1]), ("write", "read", read[1]))

    def test_existing_4125_image_declares_adapter_api(self):
        import json
        reference = json.loads((ROOT / "docs/research/uai-plus-runtime-2026-10-26/api-contracts-2026-10-27.json").read_text(encoding="utf-8"))
        # Locate the captured module independently of its enclosing manifest key.
        def find(value):
            if isinstance(value, dict):
                if "peripheral.i2c" in value:
                    return value["peripheral.i2c"]
                for child in value.values():
                    found = find(child)
                    if found is not None: return found
            return None
        module = find(reference)
        self.assertIsNotNone(module)
        self.assertIn("I2C.writeto", module["symbols"])
        self.assertIn("I2C.readfrom", module["symbols"])
        self.assertEqual(module["symbols"]["I2C.writeto"]["returns"], "int")


@unittest.skipUnless(os.environ.get("IKB1Z_TEST_LIBRARY"), "native parity runs through npm run test:ikb")
class NativeParity(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.native = ctypes.CDLL(os.environ["IKB1Z_TEST_LIBRARY"])
        cls.array = ctypes.c_uint8 * 2
        for name in ("ikb1z_encode_io", "ikb1z_encode_motor", "ikb1z_encode_servo"):
            function = getattr(cls.native, name)
            function.argtypes = [ctypes.c_uint, ctypes.c_int, ctypes.POINTER(ctypes.c_uint8)]
            function.restype = ctypes.c_int
        cls.native.ikb1z_decode_adc10.argtypes = [ctypes.POINTER(ctypes.c_uint8), ctypes.c_size_t, ctypes.POINTER(ctypes.c_uint16)]

    def test_all_valid_io_motor_servo_values_match_c_and_independent_expected_bytes(self):
        for channel in range(8):
            for mode in range(6):
                buffer = self.array()
                self.assertEqual(self.native.ikb1z_encode_io(channel, mode, buffer), 0)
                self.assertEqual(bytes(buffer), bytes((8 + channel, mode)))
                self.assertEqual(bytes(buffer), ikb.encode_io(channel, mode))
        for mask in range(1, 16):
            for speed in range(-100, 101):
                buffer = self.array()
                self.assertEqual(self.native.ikb1z_encode_motor(mask, speed, buffer), 0)
                self.assertEqual(bytes(buffer), bytes((32 + mask, speed % 256)))
                self.assertEqual(bytes(buffer), ikb.encode_motor(mask, speed))
        for mask in range(1, 64):
            for scaled in range(201):
                buffer = self.array()
                self.assertEqual(self.native.ikb1z_encode_servo(mask, scaled, buffer), 0)
                self.assertEqual(bytes(buffer), bytes((64 + mask, scaled)))
                self.assertEqual(bytes(buffer), ikb.encode_servo(mask, scaled))

    def test_all_adc10_words_accept_only_0_through_1023_without_hiding_high_bits(self):
        for word in range(65536):
            buffer = self.array(word >> 8, word & 255)
            value = ctypes.c_uint16(777)
            status = self.native.ikb1z_decode_adc10(buffer, 2, ctypes.byref(value))
            if word <= 1023:
                self.assertEqual(status, 0)
                self.assertEqual(value.value, word)
                self.assertEqual(ikb.decode_adc10(bytes(buffer)), word)
            else:
                self.assertEqual(status, 5)
                self.assertEqual(value.value, 777)
                with self.assertRaises(OSError):
                    ikb.decode_adc10(bytes(buffer))


if __name__ == "__main__":
    unittest.main()
