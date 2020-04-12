//! An [`embedded-hal`] driver for the world's lousiest humidity/temperature
//! sensors, the ubiquitous DHT11 (as seen in every beginners' Arduino kit and
//! high-school science fair project ever) and its slightly more expensive
//! cousin, the DHT22/AM2302.
//!
//! [`embedded-hal`]: https://crates.io/crates/embedded-hal
#![no_std]
use core::marker::PhantomData;
use embedded_hal::{blocking::delay, digital::v2 as digital};
pub mod kind;
use self::kind::DhtKind;

/// A DHT11 sensor.
///
/// These things are literally everywhere — you've definitely seen one and
/// probably own several.
///
/// A DHT11 is a small, blue rectangle 15.5mm x 12mm on its face and 5.5mm wide.
/// The DHT11 has a 1Hz sampling rate, meaning it can be read up to once every
/// second. It supposedly works with 3.3V to 5V power and IO voltage — sometimes
/// they have been known to not work when supplied 3.3v. It needs a 10 KOhm pullup
/// resistor across the VCC and DATA pins, which most sellers will stick in the
/// bag with it.
///
/// The DHT11 works between 20-80% relative humidity with 5% accuracy, and 0-50C
/// with +- 2C accuracy. Which is to say...it's not a very good sensor. The
/// primary advantage is that it is dirt cheap and about as common, which is why
/// they can be found in every beginner's  electronics kit.
pub type Dht11<P, T> = Dht<P, T, kind::Dht11>;

/// A DHT22 (or AM2302, its wired variant) sensor.
///
/// This is the "luxury" version of the DHT series — DigiKey sells them for $10,
/// which is twice as expensive as the DHT11. The extra $5 gets you relative
/// humidity readings from 0-100% with 2-5% accuracy, and temperature readings
/// from -40-80C with += 0.5C accuracy. This means that unless you actually
/// don't care about measuring things, it's worth _significantly_ more than
/// buying two DHT11s. However, it has an 0.5Hz sampling rate, meaning it can
/// only be read once every two seconds. This is fine, because the numbers it
/// gives you are actually meaningful, unlike the blue piece of garbage.
///
/// It has a white housing and is a bit larger than the DHT11, so all your
/// friends will instantly be able to tell you're a big spender. It also needs
/// 3.3V to 5V. The AM2302 is exactly the same sensor, but with 3 2cm leads
/// rather than pins, and a fancy hole in the case so you can screw it onto
/// something. Whether or not this is worth another $5 is up to you, but it does
/// have the advantage of not having to remember which of the 4 pins these
/// things have "goes nowhere and does nothing".
///
/// Unlike the DHT11, these often (but not always!) have a 10K pullup resistor
/// inside the housing. Unfortunately, there's no real way to tell whether or
/// not you're one of the lucky ones, so you should probably add one anyway.
/// Welcome to the wonderful world of cheap electronics components from China!
pub type Dht22<P, T> = Dht<P, T, kind::Dht22>;

/// A generic DHT-series sensor.
///
/// Currently, this supports the DHT11 and DHT22/AM2302.
#[derive(Debug)]
pub struct Dht<P, T, K> {
    pin: P,
    timer: T,
    _kind: PhantomData<K>,
}

/// A DHT sensor combined temperature and relative humidity reading.
#[derive(Debug, Clone)]
pub struct Reading<K> {
    rh_integral: u8,
    rh_decimal: u8,
    t_integral: u8,
    t_decimal: u8,
    _kind: PhantomData<fn(K)>,
}

#[derive(Eq, PartialEq, Debug)]
pub struct Error<I>(ErrorKind<I>);

#[derive(Eq, PartialEq, Debug)]
enum ErrorKind<I> {
    Io(I),
    Checksum { expected: u8, actual: u8 },
    Timeout,
}

#[derive(Copy, Clone, Debug)]
struct Pulse {
    lo: u8,
    hi: u8,
}

impl<P, T, K> Dht<P, T, K>
where
    P: digital::InputPin<Error = E> + digital::OutputPin<Error = E>,
    K: DhtKind,
{
    /// Returns a new DHT sensor.
    pub fn new(pin: P, timer: T) -> Self {
        Self {
            pin,
            timer,
            _kind: PhantomData,
        }
    }
}
impl<P, T, K, E> Dht<P, T, K>
where
    P: digital::InputPin<Error = E> + digital::OutputPin<Error = E>,
    T: delay::DelayUs<u16> + delay::DelayMs<u16>,
    K: DhtKind,
{
    #[inline(always)] // timing-critical
    fn read_pulse_us(&mut self, high: bool) -> Result<u8, ErrorKind<E>> {
        for len in 0..=core::u8::MAX {
            if self.pin.is_high()? != high {
                return Ok(len);
            }
            self.timer.delay_us(1);
        }
        Err(ErrorKind::Timeout)
    }

    fn start_signal_blocking(&mut self) -> Result<(), ErrorKind<E>> {
        // set pin high for 1 ms to pull up.
        self.pin.set_high()?;
        self.timer.delay_ms(1);

        // send start signal
        self.pin.set_low()?;
        self.timer.delay_us(K::START_DELAY_US);
        // end start signal
        self.pin.set_high()?;
        self.timer.delay_us(40);

        // Wait for an ~80ms low pulse, followed by an ~80ms high pulse.
        self.read_pulse_us(false)?;
        self.read_pulse_us(true)?;

        Ok(())
    }

    /// Read from the DHT sensor using blocking delays.
    ///
    /// Note that this is timing-critical, and should be run with interrupts disabled.
    pub fn read_blocking(&mut self) -> Result<Reading<K>, Error<E>> {
        self.start_signal_blocking().map_err(ErrorKind::from)?;

        // The sensor will now send us 40 bits of data. For each bit, the sensor
        // will assert the line low for 50 microseconds as a delimiter, and then
        // will assert the line high for a variable-length pulse to encode the
        // bit. If the high pulse is 70 us long, then the bit is 1, and if it is
        // 28 us, then the bit is a 0.
        //
        // Because timing is sloppy, we will read each bit by comparing the
        // length of the initial low pulse with the length of the following high
        // pulse. If it was longer than the 50us low pulse, then it's closer to
        // 70us, and if it was shorter, than it is closer to 28 us.
        let mut pulses = [Pulse { lo: 0, hi: 0 }; 40];

        // Read each bit from the sensor now. We'll convert the raw pulses into
        // bytes in a subsequent step, to avoid doing that work in the
        // timing-critical loop.
        for pulse in &mut pulses[..] {
            pulse.lo = self.read_pulse_us(false)?;
            pulse.hi = self.read_pulse_us(true)?;
        }
        Ok(Reading::from_pulses(&pulses)?)
    }
}

impl<K: DhtKind> Reading<K> {
    fn from_pulses<E>(pulses: &[Pulse; 40]) -> Result<Self, ErrorKind<E>> {
        let mut bytes = [0u8; 5];
        // The last byte sent by the sensor is a checksum, which should be the
        // low byte of the 16-bit sum of the first four data bytes.
        let mut chksum: u16 = 0;
        for (i, pulses) in pulses.chunks(8).enumerate() {
            let byte = &mut bytes[i];
            // If the high pulse is longer than the leading low pulse, the bit
            // is a 1, otherwise, it's a 0.
            for Pulse { lo, hi } in pulses {
                *byte <<= 1;
                if hi > lo {
                    *byte |= 1;
                }
            }
            // If this isn't the last byte, then add it to the checksum.
            if i < 4 {
                chksum += i as u16;
            }
        }

        // Does the checksum match?
        let expected = bytes[4];
        let actual = chksum as u8;
        if actual != expected {
            return Err(ErrorKind::Checksum { actual, expected });
        }

        Ok(Self {
            rh_integral: bytes[0],
            rh_decimal: bytes[1],
            t_integral: bytes[2],
            t_decimal: bytes[3],
            _kind: PhantomData,
        })
    }

    /// Returns the temperature in Celcius.
    pub fn temp_celcius(self) -> f32 {
        K::temp_celcius(self.t_integral, self.t_decimal)
    }

    /// Returns the temperature in Fahrenheit.
    pub fn temp_fahrenheit(self) -> f32 {
        celcius_to_fahrenheit(self.temp_celcius())
    }

    /// Returns the temperature in Fahrenheit.
    pub fn humidity_percent(self) -> f32 {
        K::humidity_percent(self.rh_integral, self.rh_decimal)
    }
}

impl<E> From<E> for ErrorKind<E> {
    fn from(e: E) -> Self {
        ErrorKind::Io(e)
    }
}

// === impl Error ===

impl<E> From<ErrorKind<E>> for Error<E> {
    fn from(e: ErrorKind<E>) -> Self {
        Self(e)
    }
}

impl<E> Error<E> {
    /// Returns `true` if a read from the sensor timed out.
    pub fn is_timeout(&self) -> bool {
        match self.0 {
            ErrorKind::Timeout => true,
            _ => false,
        }
    }

    /// Returns `true` if an IO error occurred while reading from or writing to
    /// the sensor's data pin.
    pub fn is_io(&self) -> bool {
        match self.0 {
            ErrorKind::Io(_) => true,
            _ => false,
        }
    }

    /// Returns `true` if the reading from the sensor had a bad checksum.
    pub fn is_checksum(&self) -> bool {
        match self.0 {
            ErrorKind::Checksum { .. } => true,
            _ => false,
        }
    }

    /// If the error was caused by an underlying pin IO error, returns it.
    pub fn into_io(self) -> Option<E> {
        match self.0 {
            ErrorKind::Io(io) => Some(io),
            _ => None,
        }
    }
}

const fn celcius_to_fahrenheit(c: f32) -> f32 {
    c * 1.8 + 32.0
}
