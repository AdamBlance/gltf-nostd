use alloc::vec::Vec;
use crate::buffer;
// use crate::image;

use crate::{Document, Error, Gltf, Result};

/// Return type of `import`.
type Import = (Document, Vec<buffer::Data>);

impl buffer::Data {
    /// Construct a buffer data object by reading the given source.
    /// If `base` is provided, then external filesystem references will
    /// be resolved from this directory.
    /// `blob` represents the `BIN` section of a binary glTF file,
    /// and it will be taken to fill the buffer if the `source` refers to it.
    pub fn from_blob(
        blob: &mut Option<Vec<u8>>,
    ) -> Result<Self> {
        let mut data = blob.take().ok_or(Error::MissingBlob)?;
        while data.len() % 4 != 0 {
            data.push(0);
        }
        Ok(buffer::Data(data))
    }
}

/// Import buffer data referenced by a glTF document.
///
/// ### Note
///
/// This function is intended for advanced users who wish to forego loading image data.
/// A typical user should call [`import`] instead.
pub fn import_buffers(
    document: &Document,
    mut blob: Option<Vec<u8>>,
) -> Result<Vec<buffer::Data>> {
    let mut buffers = Vec::new();
    for buffer in document.buffers() {
        let data = buffer::Data::from_blob(&mut blob)?;
        if data.len() < buffer.length() {
            return Err(Error::BufferLength {
                buffer: buffer.index(),
                expected: buffer.length(),
                actual: data.len(),
            });
        }
        buffers.push(data);
    }
    Ok(buffers)
}

// impl image::Data {
//     /// Construct an image data object by reading the given source.
//     /// If `base` is provided, then external filesystem references will
//     /// be resolved from this directory.
//     pub fn from_source(
//         source: image::Source<'_>,
//         buffer_data: &[buffer::Data],
//     ) -> Result<Self> {
//         #[cfg(feature = "guess_mime_type")]
//         let guess_format = |encoded_image: &[u8]| match image_crate::guess_format(encoded_image) {
//             Ok(image_crate::ImageFormat::Png) => Some(Png),
//             Ok(`image_crate::ImageFormat::Jpeg) => Some(Jpeg),
//             _ => None,
//         };
//         #[cfg(not(feature = "guess_mime_type"))]
//         let guess_format = |_encoded_image: &[u8]| None;
//         let decoded_image = match source {
//             image::Source::View { view, mime_type } => {
//                 let parent_buffer_data = &buffer_data[view.buffer().index()].0;
//                 let begin = view.offset();
//                 let end = begin + view.length();
//                 let encoded_image = &parent_buffer_data[begin..end];
//                 let encoded_format = match mime_type {
//                     "image/png" => Png,
//                     "image/jpeg" => Jpeg,
//                     _ => match guess_format(encoded_image) {
//                         Some(format) => format,
//                         None => return Err(Error::UnsupportedImageEncoding),
//                     },
//                 };
//                 image_crate::load_from_memory_with_format(encoded_image, encoded_format)?
//             }
//         };
// 
//         image::Data::new(decoded_image)
//     }
// }

// /// Import image data referenced by a glTF document.
// ///
// /// ### Note
// ///
// /// This function is intended for advanced users who wish to forego loading buffer data.
// /// A typical user should call [`import`] instead.
// pub fn import_images(
//     document: &Document,
//     buffer_data: &[buffer::Data],
// ) -> Result<Vec<image::Data>> {
//     let mut images = Vec::new();
//     for image in document.images() {
//         images.push(image::Data::from_source(image.source(), buffer_data)?);
//     }
//     Ok(images)
// }

fn import_impl(Gltf { document, blob }: Gltf) -> Result<Import> {
    let buffer_data = import_buffers(&document, blob)?;
    // let image_data = import_images(&document, &buffer_data)?;
    let import = (document, buffer_data);
    Ok(import)
}




fn import_slice_impl(slice: &[u8]) -> Result<Import> {
    import_impl(Gltf::from_slice(slice)?)
}

/// Import glTF 2.0 from a slice.
///
/// File paths in the document are assumed to be relative to the current working
/// directory.
///
/// ### Note
///
/// This function is intended for advanced users.
/// A typical user should call [`import`] instead.
///
/// ```
/// # extern crate gltf;
/// # use std::fs;
/// # use std::io::Read;
/// # fn run() -> Result<(), gltf::Error> {
/// # let path = "examples/Box.glb";
/// # let mut file = fs::File::open(path).map_err(gltf::Error::Io)?;
/// # let mut bytes = Vec::new();
/// # file.read_to_end(&mut bytes).map_err(gltf::Error::Io)?;
/// # #[allow(unused)]
/// let (document, buffers, images) = gltf::import_slice(bytes.as_slice())?;
/// # Ok(())
/// # }
/// # fn main() {
/// #     run().expect("test failure");
/// # }
/// ```
pub fn import_slice<S>(slice: S) -> Result<Import>
where
    S: AsRef<[u8]>,
{
    import_slice_impl(slice.as_ref())
}
