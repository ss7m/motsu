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
        Image {
            height,
            width,
            data,
            phantom: PhantomData,
        }
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

    pub fn row_size(&self) -> usize {
        self.width * P::NUM_CHANNELS
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> P {
        P::from_slice(&self.data, y * self.row_size() + x * P::NUM_CHANNELS)
    }

    pub fn convert<Q>(&self) -> Image<Q>
    where
        Q: Pixel,
        P: PixelConvert<Q>,
    {
        let mut data = Vec::with_capacity(self.width * self.height * Q::NUM_CHANNELS);

        for i in 0..self.height {
            for j in 0..self.width {
                let pixel: Q = self.get_pixel(j, i).convert();
                data.append(&mut pixel.into_vec());
            }
        }

        Image {
            height: self.height,
            width: self.width,
            data,
            phantom: PhantomData,
        }
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

        Image {
            height: self.height,
            width: self.width - amt,
            data,
            phantom: PhantomData,
        }
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

        Image {
            height: self.height,
            width: self.width - amt,
            data,
            phantom: PhantomData,
        }
    }

    fn crop_top(&self, amt: usize) -> Image<P> {
        if self.height <= amt {
            return self.clone();
        }

        let data = self.data[amt * self.row_size()..].to_vec();

        Image {
            height: self.height - amt,
            width: self.width,
            data,
            phantom: PhantomData,
        }
    }

    fn crop_bottom(&self, amt: usize) -> Image<P> {
        if self.height <= amt {
            return self.clone();
        }

        let data = self.data[..(self.height - amt) * self.row_size()].to_vec();

        Image {
            height: self.height - amt,
            width: self.width,
            data,
            phantom: PhantomData,
        }
    }

    pub fn crop(&self, left: usize, right: usize, top: usize, bottom: usize) -> Image<P> {
        self.crop_top(top)
            .crop_bottom(bottom)
            .crop_left(left)
            .crop_right(right)
    }
}
