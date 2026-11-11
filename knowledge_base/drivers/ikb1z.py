"""iKB-1Z protocol and existing-bus MaixPy adapter; no import-time I/O.

Host/fixture checked, physical integration unverified. UART, servo free and
external M3/M4 stay gated until their firmware/resource contracts are verified.
"""
import time


def _integer(value, low, high, name):
    if type(value) is not int or not low <= value <= high:
        raise ValueError("%s must be an integer in %s..%s" % (name, low, high))
    return value


def encode_io(channel, mode):
    return bytes((8 + _integer(channel, 0, 7, "channel"),
                  _integer(mode, 0, 5, "mode")))


def encode_motor(mask, speed):
    return bytes((0x20 | _integer(mask, 1, 15, "motor mask"),
                  _integer(speed, -100, 100, "speed") & 0xFF))


def encode_servo(mask, scaled):
    _integer(mask, 1, 63, "servo mask")
    if type(scaled) is int and scaled == -1:
        raise NotImplementedError("servo free requires firmware validation")
    return bytes((0x40 | mask, _integer(scaled, 0, 200, "scaled")))


def decode_adc10(data):
    if not isinstance(data, (bytes, bytearray)) or len(data) != 2:
        raise OSError("ADC10 requires exactly two bytes")
    raw = (data[0] << 8) | data[1]
    if raw > 1023:
        raise OSError("ADC10 response outside 0..1023")
    return raw


class IKB1Z:
    def __init__(self, transport, address):
        _integer(address, 0x48, 0x49, "7-bit address")
        if not callable(getattr(transport, "exchange", None)):
            raise ValueError("transport.exchange is required")
        self.transport = transport
        self.address = address

    def _exchange(self, payload, length=0):
        result = self.transport.exchange(self.address, payload, length)
        if not isinstance(result, (bytes, bytearray)) or len(result) != length:
            raise OSError("incomplete iKB-1Z response")
        return result

    def output(self, channel, high):
        if type(high) is not bool:
            raise ValueError("high must be bool")
        self._exchange(encode_io(channel, int(high)))

    def _read_byte(self, channel, mode, maximum):
        value = self._exchange(encode_io(channel, mode), 1)[0]
        if value > maximum:
            raise OSError("iKB-1Z response outside requested scale")
        return value

    def input(self, channel, pullup=False):
        if type(pullup) is not bool:
            raise ValueError("pullup must be bool")
        return self._read_byte(channel, 3 if pullup else 2, 1)

    def adc8(self, channel):
        return self._read_byte(channel, 4, 255)

    def adc_percent(self, channel):
        return self._read_byte(channel, 5, 100)

    def adc10(self, channel):
        command = 0x80 | (_integer(channel, 0, 7, "channel") << 4)
        return decode_adc10(self._exchange(bytes((command,)), 2))

    def motor(self, mask, speed):
        payload = encode_motor(mask, speed)
        if mask & ~3:
            raise NotImplementedError("M3/M4 require external driver/resource validation")
        self._exchange(payload)

    def stop(self):
        self.motor(3, 0)

    def servo(self, mask, scaled):
        self._exchange(encode_servo(mask, scaled))

    def servo_free(self, mask):
        _integer(mask, 1, 63, "servo mask")
        raise NotImplementedError("servo free requires firmware validation")

    def uart_read(self, *args):
        raise NotImplementedError("UART framing requires firmware validation")

    def reset(self):
        self._exchange(b"\x00")


class MaixI2CTransport:
    """Borrow an already configured external maix.peripheral.i2c.I2C object.

    Caller supplies the SAME threading.Lock for all users of the bus. No scan,
    pinmux changes, module loading, bus creation/destruction or automatic retry.
    Native writeto/readfrom have no verified timeout parameter: the deadline
    bounds lock waiting and rejects late results, but cannot interrupt native
    I/O. Use the Plus process supervisor to stop a stuck program.
    """
    def __init__(self, bus, shared_lock, timeout_ms):
        _integer(timeout_ms, 1, 2147483647, "timeout_ms")
        if not all(callable(getattr(bus, name, None)) for name in ("writeto", "readfrom")):
            raise ValueError("existing Maix I2C bus required")
        if not all(callable(getattr(shared_lock, name, None)) for name in ("acquire", "release")):
            raise ValueError("shared bus lock required")
        self.bus, self.lock = bus, shared_lock
        self.timeout = timeout_ms / 1000.0

    def exchange(self, address, payload, read_length):
        _integer(address, 0x48, 0x49, "7-bit address")
        _integer(read_length, 0, 2, "read_length")
        if not isinstance(payload, bytes) or not 1 <= len(payload) <= 2:
            raise ValueError("command requires one or two bytes")
        deadline = time.monotonic() + self.timeout
        if not self.lock.acquire(timeout=self.timeout):
            raise TimeoutError("iKB-1Z shared bus lock timeout")
        try:
            self._check_deadline(deadline)
            written = self.bus.writeto(address, payload)
            if type(written) is not int or written != len(payload):
                raise OSError("iKB-1Z write failed or incomplete")
            self._check_deadline(deadline)
            result = self.bus.readfrom(address, read_length) if read_length else b""
            self._check_deadline(deadline)
            if not isinstance(result, (bytes, bytearray)) or len(result) != read_length:
                raise OSError("iKB-1Z read failed or incomplete")
            return result
        finally:
            self.lock.release()

    @staticmethod
    def _check_deadline(deadline):
        if time.monotonic() >= deadline:
            raise TimeoutError("iKB-1Z operation exceeded deadline; no automatic retry")
