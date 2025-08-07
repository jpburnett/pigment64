pub mod color;
pub mod image;

pub use crate::image::native_image::NativeImage;
pub use crate::image::png_image::{PNGImage, create_palette_from_png};

#[cfg_attr(not(feature = "python_bindings"), allow(unused_imports))]
use std::io::Cursor;

// Imports from external crates
use num_enum::TryFromPrimitive;
use strum_macros::{EnumCount, EnumIter};
use thiserror::Error;

// PyO3 imports (will only be used if the feature is enabled)
#[cfg(feature = "python_bindings")]
use pyo3::prelude::*;
#[cfg(feature = "python_bindings")]
use pyo3::types::PyBytes;

#[derive(Debug, Error)]
pub enum Pigment64Error {
    #[error("Invalid image size for TLUT: {0:?}")]
    InvalidSizeForTlut(ImageSize),
    #[error("Unknown image size value: {0}")]
    UnknownImageSize(u8),
    #[error("Unknown image format value: {0}")]
    UnknownImageFormat(u8),
    #[error("Unknown image type value: {0}")]
    UnknownImageType(u8),
    #[error("Unknown texture LUT value: {0}")]
    UnknownTextureLUT(u8),
    #[error("A TLUT color table is required for this image format")]
    MissingTlut,
    #[error("The specified TLUT mode is not supported: {0:?}")]
    UnsupportedTlutMode(TextureLUT),
    #[error("TLUT index is out of bounds")]
    TlutIndexOutOfBounds,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("PNG encoding error: {0}")]
    PngEncoding(#[from] png::EncodingError),
    #[error("PNG decoding error: {0}")]
    PngDecoding(#[from] png::DecodingError),
    #[error("PNG is missing a palette, which is required for this operation")]
    MissingPngPalette,
    #[error(
        "Unsupported PNG format for conversion to {target_format:?}: color={color:?}, depth={depth:?}"
    )]
    UnsupportedPngConversion {
        color: png::ColorType,
        depth: png::BitDepth,
        target_format: ImageType,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum ImageSize {
    Bits1 = 4,
    Bits4 = 0,
    Bits8 = 1,
    Bits16 = 2,
    Bits32 = 3,
    DD = 5,
}

impl ImageSize {
    /// Returns the size of the TLUT (Table Look-Up Table) based on the image size.
    ///
    /// # Returns
    ///
    /// The size of the TLUT as a `usize` value.
    ///
    /// # Panics
    ///
    /// This method will panic if the image size is invalid.
    pub fn get_tlut_size(&self) -> Result<usize, Pigment64Error> {
        match self {
            ImageSize::Bits1 => Ok(0b10),
            ImageSize::Bits4 => Ok(0x10),
            ImageSize::Bits8 => Ok(0x100),
            ImageSize::Bits16 => Ok(0x1000),
            ImageSize::Bits32 => Ok(0x10000),
            _ => Err(Pigment64Error::InvalidSizeForTlut(*self)),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum ImageFormat {
    Rgba = 0,
    Yuv = 1,
    Ci = 2,
    Ia = 3,
    I = 4,
}

/// Represents the type of image.
///
/// This enum is used to specify the type of image, which determines the size and format of the
/// image data.
/// Each variant corresponds to a specific image type, such as indexed color (Ci), grayscale (I),
/// grayscale with alpha (Ia), or red-green-blue-alpha (RGBA).
///
#[derive(Copy, Clone, Debug, PartialEq, EnumCount, EnumIter, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum ImageType {
    I1,
    I4,
    I8,
    Ia4,
    Ia8,
    Ia16,
    Ci4,
    Ci8,
    Rgba16,
    Rgba32,
}

impl ImageType {
    /// Returns the size of the image type.
    ///
    /// This function returns the size of the image type, which represents the number of bits used
    /// to store each pixel. The size is determined based on the image type variant.
    ///
    /// # Returns
    ///
    /// - `ImageSize` - The size of the image type.
    pub fn get_size(&self) -> ImageSize {
        match self {
            ImageType::Ci4 => ImageSize::Bits4,
            ImageType::Ci8 => ImageSize::Bits8,
            ImageType::I1 => ImageSize::Bits1,
            ImageType::I4 => ImageSize::Bits4,
            ImageType::I8 => ImageSize::Bits8,
            ImageType::Ia4 => ImageSize::Bits4,
            ImageType::Ia8 => ImageSize::Bits8,
            ImageType::Ia16 => ImageSize::Bits16,
            ImageType::Rgba16 => ImageSize::Bits16,
            ImageType::Rgba32 => ImageSize::Bits32,
        }
    }

    /// Returns the format of the image type.
    ///
    /// This method returns the format of the image type, which represents the color model used by
    /// the image. The format is determined based on the image type variant.
    ///
    /// # Returns
    ///
    /// - `ImageFormat` - The format of the image type.
    pub fn get_format(&self) -> ImageFormat {
        match self {
            ImageType::Ci4 => ImageFormat::Ci,
            ImageType::Ci8 => ImageFormat::Ci,
            ImageType::I1 => ImageFormat::I,
            ImageType::I4 => ImageFormat::I,
            ImageType::I8 => ImageFormat::I,
            ImageType::Ia4 => ImageFormat::Ia,
            ImageType::Ia8 => ImageFormat::Ia,
            ImageType::Ia16 => ImageFormat::Ia,
            ImageType::Rgba16 => ImageFormat::Rgba,
            ImageType::Rgba32 => ImageFormat::Rgba,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum TextureLUT {
    None = 0,
    Rgba16 = 2,
    Ia16 = 3,
}

// --- Python Bindings ---

#[cfg(feature = "python_bindings")]
#[pyclass(name = "PNGImage")]
struct PyPNGImage {
    img: PNGImage,
}

#[cfg(feature = "python_bindings")]
#[pymethods]
impl PyPNGImage {
    #[new]
    fn new(bytes: &[u8]) -> PyResult<Self> {
        let img = PNGImage::read(bytes).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Failed to read PNG: {}", e))
        })?;
        Ok(PyPNGImage { img })
    }

    fn as_i8(&self) -> PyResult<Py<PyBytes>> {
        let mut buf = Vec::new();
        self.img.as_i8(&mut buf).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to convert to I8: {}",
                e
            ))
        })?;
        let py = unsafe { Python::assume_gil_acquired() };
        Ok(PyBytes::new(py, &buf).into())
    }

    fn as_rgba16(&self) -> PyResult<Py<PyBytes>> {
        let mut buf = Vec::new();
        self.img.as_rgba16(&mut buf).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to convert to RGBA16: {}",
                e
            ))
        })?;
        let py = unsafe { Python::assume_gil_acquired() };
        Ok(PyBytes::new(py, &buf).into())
    }
}

#[cfg(feature = "python_bindings")]
#[pyfunction]
fn extract_palette_from_png_bytes(py: Python, png_bytes: &[u8]) -> PyResult<Py<PyBytes>> {
    let mut png_cursor = Cursor::new(png_bytes);
    let mut palette_data_vec: Vec<u8> = Vec::new();

    create_palette_from_png(&mut png_cursor, &mut palette_data_vec).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Failed to extract palette: {}", e))
    })?;

    let py_bytes = PyBytes::new(py, &palette_data_vec);
    Ok(py_bytes.into())
}

#[cfg(feature = "python_bindings")]
#[pyfunction]
#[pyo3(name = "native_to_png")]
fn native_to_png_py(
    py: Python,
    bytes: &[u8],
    img_type_str: &str,
    width: u32,
    height: u32,
    tlut: Option<&[u8]>,
) -> PyResult<Py<PyBytes>> {
    let img_type = match img_type_str {
        "rgba32" => ImageType::Rgba32,
        "rgba16" => ImageType::Rgba16,
        "ia16" => ImageType::Ia16,
        "ia8" => ImageType::Ia8,
        "ia4" => ImageType::Ia4,
        "i8" => ImageType::I8,
        "i4" => ImageType::I4,
        "i1" => ImageType::I1,
        "ci8" => ImageType::Ci8,
        "ci4" => ImageType::Ci4,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid image type: '{}'",
                img_type_str
            )));
        }
    };

    let mut reader = Cursor::new(bytes);

    // This now returns a `Result` with `anyhow::Error`, which we need to handle.
    // We can map it to a Python exception.
    let native_image_result = NativeImage::read(&mut reader, img_type, width, height);
    let native_image = native_image_result.map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Failed to read native image: {}",
            e
        ))
    })?;

    let mut png_buf = Vec::new();

    // This also returns a `Result` with `anyhow::Error`.
    native_image.as_png(&mut png_buf, tlut).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to encode PNG: {}", e))
    })?;

    Ok(PyBytes::new(py, &png_buf).into())
}

#[cfg(feature = "python_bindings")]
#[pymodule]
#[pyo3(name = "pigment64")]
fn pigment64_py_module(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(extract_palette_from_png_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(native_to_png_py, m)?)?;
    m.add_class::<PyPNGImage>()?;
    Ok(())
}
