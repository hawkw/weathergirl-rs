pub trait DhtKind {
    #[doc(hidden)]
    const START_DELAY_US: u16;

    #[doc(hidden)]
    fn temp_celcius(integral: u8, decimal: u8) -> f32;

    #[doc(hidden)]
    fn humidity_percent(integral: u8, decimal: u8) -> f32;
}

pub struct Dht11 {
    _p: (),
}

pub struct Dht22 {
    _p: (),
}

impl DhtKind for Dht11 {
    // Datasheet says 20 ms.
    const START_DELAY_US: u16 = 20 * 1000;

    fn temp_celcius(integral: u8, decimal: u8) -> f32 {
        // XXX(eliza): this is kind of copied from the Adafruit driver implementation,
        // which doesn't really explain what it's doing.
        let mut temp = integral as f32;
        if decimal & 0x80 != 0 {
            temp = -1.0 - temp;
        }
        temp + (decimal & 0x0f) as f32 * 0.1
    }

    fn humidity_percent(integral: u8, decimal: u8) -> f32 {
        integral as f32 + decimal as f32 * 0.1
    }
}

impl DhtKind for Dht22 {
    // Datasheet says "at least" 1 ms, so we'll delay for just over 1ms.
    const START_DELAY_US: u16 = 1100;

    fn temp_celcius(integral: u8, decimal: u8) -> f32 {
        let mut temp = (((integral & 0x7F) as u16) << 8 | decimal as u16) as f32;
        temp *= 0.1;
        if integral & 0x80 != 0 {
            temp *= -1.0;
        }
        temp
    }

    fn humidity_percent(integral: u8, decimal: u8) -> f32 {
        ((integral as u16) << 8 | decimal as u16) as f32 * 0.1
    }
}
