# PNG

This is a png viewer and "editor" written with luminance and libpng.

## Usage

`png-rs [FILE] [FILE]`

The second file is optional, and if given `png-rs` will write the image to this second file. Outfile will
always be saved using 8-bit RGBA.

Currently, only cropping is supported. Cropping is done with arrow-keys, and uncropping is done by holding shift and an arrow key.
Hold ctrl while cropping to corp faster.
