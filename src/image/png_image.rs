use crate::ImageType;
use crate::color::Color;
use anyhow::{Result, anyhow};
use byteorder::{BigEndian, WriteBytesExt};
use png::{BitDepth, ColorType};
use std::io::{Read, Write};

#[derive(Debug, Clone)]
pub struct PNGImage {
    /// Raw image data in row-major order.
    pub data: Vec<u8>,
    /// The color type of the image.
    pub color_type: ColorType,
    /// The bit depth of each color channel.
    pub bit_depth: BitDepth,
    /// The width of the image in pixels.
    pub width: u32,
    /// The height of the image in pixels.
    pub height: u32,
}

#[inline]
fn u8_to_u4(x: u8) -> u8 {
    x >> 4
}

impl PNGImage {
    pub fn read<R: Read>(r: R) -> Result<Self> {
        let decoder = png::Decoder::new(r);
        let mut reader = decoder.read_info()?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf)?;
        let input_bytes = &buf[..info.buffer_size()];

        Ok(PNGImage {
            data: input_bytes.to_vec(),
            color_type: info.color_type,
            bit_depth: info.bit_depth,
            width: info.width,
            height: info.height,
        })
    }

    pub fn flip(&self, flip_x: bool, flip_y: bool) -> PNGImage {
        let mut flipped_bytes = vec![0; self.data.len()];
        let samples = self.color_type.samples();

        for y in 0..self.height {
            for x in 0..self.width {
                let old_x = if flip_x { self.width - 1 - x } else { x };
                let old_y = if flip_y { self.height - 1 - y } else { y };
                let old_index = (old_y * self.width + old_x) as usize * samples;
                let new_index = (y * self.width + x) as usize * samples;
                flipped_bytes[new_index..new_index + samples]
                    .copy_from_slice(&self.data[old_index..old_index + samples]);
            }
        }

        PNGImage {
            data: flipped_bytes,
            ..self.clone()
        }
    }

    /// Writes the image as a PNG to the given writer.
    pub fn as_png<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut encoder = png::Encoder::new(writer, self.width, self.height);
        encoder.set_color(self.color_type);
        encoder.set_depth(self.bit_depth);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&self.data)?;
        Ok(())
    }

    pub fn as_native<W: Write>(&self, writer: &mut W, image_type: ImageType) -> Result<()> {
        match image_type {
            ImageType::I1 => self.as_i1(writer),
            ImageType::I4 => self.as_i4(writer),
            ImageType::I8 => self.as_i8(writer),
            ImageType::Ia4 => self.as_ia4(writer),
            ImageType::Ia8 => self.as_ia8(writer),
            ImageType::Ia16 => self.as_ia16(writer),
            ImageType::Ci4 => self.as_ci4(writer),
            ImageType::Ci8 => self.as_ci8(writer),
            ImageType::Rgba32 => self.as_rgba32(writer),
            ImageType::Rgba16 => self.as_rgba16(writer),
        }
    }

    pub fn as_ci8<W: Write>(&self, writer: &mut W) -> Result<()> {
        if self.bit_depth != BitDepth::Eight || self.color_type != ColorType::Indexed {
            return Err(anyhow!(
                "Invalid format for CI8 conversion. Expected 8-bit Indexed PNG. Got: {:?}, {:?}",
                self.color_type,
                self.bit_depth
            ));
        }
        writer.write_all(&self.data)?;
        Ok(())
    }

    pub fn as_ci4<W: Write>(&self, writer: &mut W) -> Result<()> {
        if self.color_type != ColorType::Indexed {
            return Err(anyhow!(
                "Invalid color type for CI4 conversion: {:?}. Expected Indexed.",
                self.color_type
            ));
        }

        match self.bit_depth {
            BitDepth::Four => writer.write_all(&self.data)?,
            BitDepth::Eight => {
                for chunk in self.data.chunks_exact(2) {
                    writer.write_u8(chunk[0] << 4 | chunk[1])?;
                }
            }
            _ => {
                return Err(anyhow!(
                    "Unsupported bit depth for CI4 conversion: {:?}",
                    self.bit_depth
                ));
            }
        }

        Ok(())
    }

    pub fn as_i1<W: Write>(&self, writer: &mut W) -> Result<()> {
        if let (ColorType::Grayscale, BitDepth::One) = (self.color_type, self.bit_depth) {
            writer.write_all(&self.data)?;
        } else {
            // Convert to i8 and then convert to i1
            let mut i8_data = Vec::new();
            self.as_i8(&mut i8_data)?;

            for pixels in i8_data.chunks_exact(8) {
                // Combine the 8 pixels into a single byte
                let mut byte = 0;
                for (i, pixel) in pixels.iter().copied().enumerate() {
                    // If its intensity is over half, set the bit
                    if pixel > u8::MAX / 2 {
                        byte |= 1 << (7 - i);
                    }
                }
                writer.write_u8(byte)?;
            }
        }
        Ok(())
    }

    pub fn as_i4<W: Write>(&self, writer: &mut W) -> Result<()> {
        match (self.color_type, self.bit_depth) {
            (ColorType::Grayscale, BitDepth::Four) => writer.write_all(&self.data)?,
            (ColorType::Grayscale, BitDepth::Eight) => {
                for chunk in self.data.chunks_exact(2) {
                    writer.write_u8(chunk[0] << 4 | u8_to_u4(chunk[1]))?;
                }
            }
            (ColorType::Rgba, BitDepth::Eight) => {
                for chunk in self.data.chunks_exact(8) {
                    let c1 = Color::RGBA(chunk[0], chunk[1], chunk[2], chunk[3]);
                    let i1 = c1.rgb_to_intensity();
                    let c2 = Color::RGBA(chunk[4], chunk[5], chunk[6], chunk[7]);
                    let i2 = c2.rgb_to_intensity();
                    writer.write_u8(u8_to_u4(i1) << 4 | u8_to_u4(i2))?;
                }
            }
            (ColorType::Rgb, BitDepth::Eight) => {
                for chunk in self.data.chunks_exact(6) {
                    let c1 = Color::RGB(chunk[0], chunk[1], chunk[2]);
                    let i1 = c1.rgb_to_intensity();
                    let c2 = Color::RGB(chunk[3], chunk[4], chunk[5]);
                    let i2 = c2.rgb_to_intensity();
                    writer.write_u8(u8_to_u4(i1) << 4 | u8_to_u4(i2))?;
                }
            }
            p => return Err(anyhow!("Unsupported format for I4 conversion: {:?}", p)),
        }
        Ok(())
    }

    pub fn as_i8<W: Write>(&self, writer: &mut W) -> Result<()> {
        match (self.color_type, self.bit_depth) {
            (ColorType::Grayscale, BitDepth::Eight) => writer.write_all(&self.data)?,
            (ColorType::Grayscale, BitDepth::Four) => {
                for chunk in self.data.chunks_exact(2) {
                    writer.write_u8(chunk[0] << 4 | chunk[1])?;
                }
            }
            (ColorType::Rgba, BitDepth::Eight) => {
                for chunk in self.data.chunks_exact(4) {
                    let c = Color::RGBA(chunk[0], chunk[1], chunk[2], chunk[3]);
                    writer.write_u8(c.rgb_to_intensity())?;
                }
            }
            (ColorType::Rgb, BitDepth::Eight) => {
                for chunk in self.data.chunks_exact(3) {
                    let c = Color::RGB(chunk[0], chunk[1], chunk[2]);
                    writer.write_u8(c.rgb_to_intensity())?;
                }
            }
            p => return Err(anyhow!("Unsupported format for I8 conversion: {:?}", p)),
        }
        Ok(())
    }

    pub fn as_ia4<W: Write>(&self, writer: &mut W) -> Result<()> {
        match (self.color_type, self.bit_depth) {
            (ColorType::GrayscaleAlpha, BitDepth::Eight) => {
                for chunk in self.data.chunks_exact(4) {
                    let intensity = (chunk[0] >> 5) << 1;
                    let alpha = (chunk[1] > 127) as u8;
                    let high = intensity | alpha;

                    let intensity = (chunk[2] >> 5) << 1;
                    let alpha = (chunk[3] > 127) as u8;
                    let low = intensity | alpha;

                    writer.write_u8(high << 4 | (low & 0xF))?;
                }
            }
            (ColorType::Rgba, BitDepth::Eight) => {
                for chunk in self.data.chunks_exact(8) {
                    let c1 = Color::RGBA(chunk[0], chunk[1], chunk[2], chunk[3]);
                    let intensity1 = (c1.rgb_to_intensity() >> 5) << 1;
                    let alpha1 = (c1.a > 127) as u8;

                    let c2 = Color::RGBA(chunk[4], chunk[5], chunk[6], chunk[7]);
                    let intensity2 = (c2.rgb_to_intensity() >> 5) << 1;
                    let alpha2 = (c2.a > 127) as u8;

                    let high = intensity1 | alpha1;
                    let low = intensity2 | alpha2;
                    writer.write_u8(high << 4 | (low & 0xF))?;
                }
            }
            p => return Err(anyhow!("Unsupported format for IA4 conversion: {:?}", p)),
        }
        Ok(())
    }

    pub fn as_ia8<W: Write>(&self, writer: &mut W) -> Result<()> {
        match (self.color_type, self.bit_depth) {
            (ColorType::GrayscaleAlpha, BitDepth::Eight) => {
                for chunk in self.data.chunks_exact(2) {
                    writer.write_u8(chunk[0] << 4 | (chunk[1] & 0x0F))?;
                }
            }
            (ColorType::Rgba, BitDepth::Eight) => {
                for chunk in self.data.chunks_exact(4) {
                    let c = Color::RGBA(chunk[0], chunk[1], chunk[2], chunk[3]);
                    let i = (c.rgb_to_intensity() >> 4) & 0xF;
                    let a = (c.a >> 4) & 0xF;
                    writer.write_u8(i << 4 | a)?;
                }
            }
            p => return Err(anyhow!("Unsupported format for IA8 conversion: {:?}", p)),
        }
        Ok(())
    }

    pub fn as_ia16<W: Write>(&self, writer: &mut W) -> Result<()> {
        match (self.color_type, self.bit_depth) {
            (ColorType::GrayscaleAlpha, BitDepth::Eight) => writer.write_all(&self.data)?,
            (ColorType::Rgba, BitDepth::Eight) => {
                for chunk in self.data.chunks_exact(4) {
                    let c = Color::RGBA(chunk[0], chunk[1], chunk[2], chunk[3]);
                    let i = c.rgb_to_intensity();
                    let a = c.a;
                    writer.write_u8(i)?;
                    writer.write_u8(a)?;
                }
            }
            p => return Err(anyhow!("Unsupported format for IA16 conversion: {:?}", p)),
        }
        Ok(())
    }

    pub fn as_rgba16<W: Write>(&self, writer: &mut W) -> Result<()> {
        match (self.color_type, self.bit_depth) {
            (ColorType::Rgba, BitDepth::Eight) => {
                for chunk in self.data.chunks_exact(4) {
                    let color = Color::RGBA(chunk[0], chunk[1], chunk[2], chunk[3]);
                    writer.write_u16::<BigEndian>(color.to_u16())?;
                }
            }
            p => return Err(anyhow!("Unsupported format for RGBA16 conversion: {:?}", p)),
        }
        Ok(())
    }

    pub fn as_rgba32<W: Write>(&self, writer: &mut W) -> Result<()> {
        match (self.color_type, self.bit_depth) {
            (ColorType::Rgba, BitDepth::Eight) => writer.write_all(&self.data)?,
            p => return Err(anyhow!("Unsupported format for RGBA32 conversion: {:?}", p)),
        }
        Ok(())
    }
}

pub fn create_palette_from_png<R: Read, W: Write>(r: R, writer: &mut W) -> Result<()> {
    let decoder = png::Decoder::new(r);
    let reader = decoder.read_info()?;
    let info = reader.info();

    let rgb_data = info
        .palette
        .as_ref()
        .ok_or_else(|| anyhow!("given PNG has no palette"))?;

    let alpha_data = info.trns.as_ref();

    match alpha_data {
        Some(alpha_data) => {
            for (rgb, &alpha) in rgb_data.chunks_exact(3).zip(alpha_data.iter()) {
                let color = Color::RGBA(rgb[0], rgb[1], rgb[2], alpha);
                writer.write_u16::<BigEndian>(color.to_u16())?;
            }
        }
        None => {
            for rgb in rgb_data.chunks_exact(3) {
                let color = Color::RGB(rgb[0], rgb[1], rgb[2]);
                writer.write_u16::<BigEndian>(color.to_u16())?;
            }
        }
    }

    Ok(())
}
