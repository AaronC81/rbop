use alloc::{vec::Vec, vec};

pub trait Serializable where Self: Sized {
    fn serialize(&self) -> Vec<u8>;

    fn deserialize(bytes: &mut dyn Iterator<Item = u8>) -> Option<Self>;
}

impl<T : num_traits::PrimInt> Serializable for T {
    // Numbers are only used for counting here, and it's quite unlikely we'll get any over a byte
    // large. So, numbers are serialized using a funky variable-length representation:
    //   0x02 = 2
    //   0xFE = 254
    //   0xFF 0x00 = 255
    //   0xFF 0x02 = 257
    //   0xFF 0xFF 0x02 = 512
    // 0xFF is always followed by another byte which is added to the 0xFF.
    fn serialize(&self) -> Vec<u8> {
        if self < &Self::zero() { panic!("cannot serialize negative numbers"); }

        let mut result = vec![];
        let mut current = *self;
        while current >= Self::from(0xFF).unwrap() {
            current = current - Self::from(0xFF).unwrap();
            result.push(0xFF);
        }
        result.push(num_traits::cast(current).unwrap());

        result
    }

    fn deserialize(bytes: &mut dyn Iterator<Item = u8>) -> Option<Self> {
        let mut result = Self::zero();

        loop {
            let byte = bytes.next()?;
            result = result + Self::from(byte).unwrap();
            if byte != 0xFF { break; }
        }

        Some(result)
    }
}
