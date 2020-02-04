#![allow(dead_code)]

#[derive(Clone, Copy)]
pub struct Gray {
    pub gray: u8,
}

#[derive(Clone, Copy)]
pub struct RGB {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Clone, Copy)]
pub struct GrayA {
    pub gray: u8,
    pub alpha: u8,
}

#[derive(Clone, Copy)]
pub struct RGBA {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

fn rgb_to_gray(r: u8, g: u8, b: u8) -> u8 {
    let r = 0.3 * (r as f32);
    let g = 0.59 * (g as f32);
    let b = 0.11 * (b as f32);

    (r + g + b) as u8
}

// TODO: Once const generics are more robust, implement for slices
// the const version of from_slice may or may not be a good idea...
pub trait Pixel: Copy {
    const NUM_CHANNELS: usize;

    fn into_vec(self) -> Vec<u8>;
    fn from_slice(vec: &[u8], idx: usize) -> Self;
    // fn into_slice(self) -> [u8; Self::NUM_CHANNELS];
    // fn from_slice(vec: &[u8; Self::NUM_CHANNELS]) -> Self;
}

impl Pixel for Gray {
    const NUM_CHANNELS: usize = 1;

    fn into_vec(self) -> Vec<u8> {
        vec![self.gray]
    }

    fn from_slice(vec: &[u8], idx: usize) -> Gray {
        Gray { gray: vec[idx] }
    }
}

impl Pixel for RGB {
    const NUM_CHANNELS: usize = 3;

    fn into_vec(self) -> Vec<u8> {
        vec![self.red, self.green, self.blue]
    }

    fn from_slice(vec: &[u8], idx: usize) -> RGB {
        RGB {
            red: vec[idx],
            green: vec[idx + 1],
            blue: vec[idx + 2],
        }
    }
}

impl Pixel for RGBA {
    const NUM_CHANNELS: usize = 4;

    fn into_vec(self) -> Vec<u8> {
        vec![self.red, self.green, self.blue, self.alpha]
    }

    fn from_slice(vec: &[u8], idx: usize) -> RGBA {
        RGBA {
            red: vec[idx],
            green: vec[idx + 1],
            blue: vec[idx + 2],
            alpha: vec[idx + 3],
        }
    }
}

impl Pixel for GrayA {
    const NUM_CHANNELS: usize = 2;

    fn into_vec(self) -> Vec<u8> {
        vec![self.gray, self.alpha]
    }

    fn from_slice(vec: &[u8], idx: usize) -> GrayA {
        GrayA {
            gray: vec[idx],
            alpha: vec[idx + 2],
        }
    }
}

pub trait PixelConvert<T>: Pixel
where
    T: Pixel,
{
    fn convert(self) -> T;
}

// Convert implementations for RGB
impl PixelConvert<RGB> for RGB {
    fn convert(self) -> RGB {
        self
    }
}

impl PixelConvert<Gray> for RGB {
    fn convert(self) -> Gray {
        Gray {
            gray: rgb_to_gray(self.red, self.green, self.blue),
        }
    }
}

impl PixelConvert<RGBA> for RGB {
    fn convert(self) -> RGBA {
        RGBA {
            red: self.red,
            green: self.green,
            blue: self.blue,
            alpha: 0xff,
        }
    }
}

impl PixelConvert<GrayA> for RGB {
    fn convert(self) -> GrayA {
        GrayA {
            gray: rgb_to_gray(self.red, self.green, self.blue),
            alpha: 0xff,
        }
    }
}

// Convert implementations for Gray
impl PixelConvert<RGB> for Gray {
    fn convert(self) -> RGB {
        RGB {
            red: self.gray,
            green: self.gray,
            blue: self.gray,
        }
    }
}

impl PixelConvert<Gray> for Gray {
    fn convert(self) -> Gray {
        self
    }
}

impl PixelConvert<RGBA> for Gray {
    fn convert(self) -> RGBA {
        RGBA {
            red: self.gray,
            green: self.gray,
            blue: self.gray,
            alpha: 0xff,
        }
    }
}

impl PixelConvert<GrayA> for Gray {
    fn convert(self) -> GrayA {
        GrayA {
            gray: self.gray,
            alpha: 0xff,
        }
    }
}

// Convert implementations for RGBA
impl PixelConvert<RGB> for RGBA {
    fn convert(self) -> RGB {
        RGB {
            red: self.red,
            green: self.green,
            blue: self.blue,
        }
    }
}

impl PixelConvert<Gray> for RGBA {
    fn convert(self) -> Gray {
        Gray {
            gray: rgb_to_gray(self.red, self.green, self.blue),
        }
    }
}

impl PixelConvert<RGBA> for RGBA {
    fn convert(self) -> RGBA {
        self
    }
}

impl PixelConvert<GrayA> for RGBA {
    fn convert(self) -> GrayA {
        GrayA {
            gray: rgb_to_gray(self.red, self.green, self.blue),
            alpha: self.alpha,
        }
    }
}

// Convert implementations for GrayA
impl PixelConvert<RGB> for GrayA {
    fn convert(self) -> RGB {
        RGB {
            red: self.gray,
            green: self.gray,
            blue: self.gray,
        }
    }
}

impl PixelConvert<Gray> for GrayA {
    fn convert(self) -> Gray {
        Gray { gray: self.gray }
    }
}

impl PixelConvert<RGBA> for GrayA {
    fn convert(self) -> RGBA {
        RGBA {
            red: self.gray,
            green: self.gray,
            blue: self.gray,
            alpha: self.alpha,
        }
    }
}

impl PixelConvert<GrayA> for GrayA {
    fn convert(self) -> GrayA {
        self
    }
}
