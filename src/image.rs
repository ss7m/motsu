#![allow(dead_code)]
use crate::pixel::*;

use std::marker::PhantomData;

// Immutable representation of an image
#[derive(Clone)]
pub struct Image<P>
where
    P: Pixel,
{
    height: usize,
    width: usize,
    data: Vec<u8>,
    phantom: PhantomData<*const P>,
}

impl<P> Image<P>
where
    P: Pixel,
{
    pub fn new(height: usize, width: usize, data: Vec<u8>) -> Image<P> {
        let mut image = Image::make_image(height, width, data);
        image.shrink_data();
        image
    }

    // Make an Image<P> without resizing the vector
    fn make_image(height: usize, width: usize, data: Vec<u8>) -> Image<P> {
        Image {
            height,
            width,
            data,
            phantom: PhantomData,
        }
    }

    fn shrink_data(&mut self) {
        let size = self.height * self.width * P::NUM_CHANNELS;
        if self.data.len() != size {
            self.data.resize_with(size, || 0);
        }
        self.data.shrink_to_fit();
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn into_raw(self) -> Vec<u8> {
        self.data
    }

    pub fn row_size(&self) -> usize {
        self.width * P::NUM_CHANNELS
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> P {
        P::from_slice(&self.data, y * self.row_size() + x * P::NUM_CHANNELS)
    }

    pub fn to_pixels(&self) -> Vec<Vec<P>> {
        let mut pixels = Vec::with_capacity(self.height);
        for y in 0..self.height {
            let mut row = Vec::with_capacity(self.width);
            for x in 0..self.width {
                row.push(self.get_pixel(x, y));
            }
            pixels.push(row);
        }

        pixels
    }

    pub fn from_pixels(pixels: Vec<Vec<P>>) -> Image<P> {
        if pixels.is_empty() || pixels[0].is_empty() {
            Image::make_image(0, 0, Vec::with_capacity(0))
        } else {
            let height = pixels.len();
            let width = pixels[0].len();
            let mut data = Vec::with_capacity(height * width * P::NUM_CHANNELS);

            for row in pixels {
                for pixel in row {
                    data.extend_from_slice(&pixel.into_vec());
                }
            }

            Image::make_image(height, width, data)
        }
    }

    pub fn convert<Q>(&self) -> Image<Q>
    where
        Q: Pixel,
        P: PixelConvert<Q>,
    {
        let mut data = Vec::with_capacity(self.width * self.height * Q::NUM_CHANNELS);
        for y in 0..self.height {
            for x in 0..self.width {
                let pixel: Q = self.get_pixel(x, y).convert();
                data.append(&mut pixel.into_vec());
            }
        }

        Image::new(self.height, self.width, data)
    }

    fn crop_left(&self, amt: usize) -> Image<P> {
        if self.width <= amt {
            return self.clone();
        }

        let mut data = Vec::with_capacity((self.width - amt) * self.height * P::NUM_CHANNELS);

        for i in 0..self.height {
            let start = i * self.row_size();
            let end = start + self.row_size();
            let row = &self.data[start + P::NUM_CHANNELS * amt..end];
            data.extend_from_slice(row);
        }

        Image::make_image(self.height, self.width - amt, data)
    }

    fn crop_right(&self, amt: usize) -> Image<P> {
        if self.width <= amt {
            return self.clone();
        }

        let mut data = Vec::with_capacity((self.width - amt) * self.height * P::NUM_CHANNELS);

        for i in 0..self.height {
            let start = i * self.row_size();
            let end = start + self.row_size();
            let row = &self.data[start..end - P::NUM_CHANNELS * amt];
            data.extend_from_slice(row);
        }

        Image::make_image(self.height, self.width - amt, data)
    }

    fn crop_top(&self, amt: usize) -> Image<P> {
        if self.height <= amt {
            return self.clone();
        }

        let data = self.data[amt * self.row_size()..].to_vec();
        Image::make_image(self.height - amt, self.width, data)
    }

    fn crop_bottom(&self, amt: usize) -> Image<P> {
        if self.height <= amt {
            return self.clone();
        }

        let data = self.data[..(self.height - amt) * self.row_size()].to_vec();
        Image::make_image(self.height - amt, self.width, data)
    }

    pub fn crop(&self, left: usize, right: usize, top: usize, bottom: usize) -> Image<P> {
        let mut image = self
            .crop_top(top)
            .crop_bottom(bottom)
            .crop_left(left)
            .crop_right(right);
        image.shrink_data();
        image
    }
}
