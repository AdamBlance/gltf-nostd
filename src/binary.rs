use alloc::borrow::{Cow, ToOwned};
use alloc::vec;
use alloc::vec::Vec;
use core::{fmt, mem};
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use core2::io;
use core2::io::Read;
use serde_json::error::Category::Io;

/// Represents a Glb loader error.
#[derive(Debug)]
pub enum Error {
    /// Unsupported version.
    Version(u32),
    /// Magic says that file is not glTF.
    Magic(u32),
    /// Length specified in GLB header exceeeds that of slice.
    Length {
        /// length specified in GLB header.
        length: u32,
        /// Actual length of data read.
        length_read: usize,
    },
    /// Stream ended before we could read the chunk.
    ChunkLength {
        /// chunkType error happened at.
        ty: ChunkType,
        /// chunkLength.
        length: u32,
        /// Actual length of data read.
        length_read: usize,
    },
    /// Chunk of this chunkType was not expected.
    ChunkType(ChunkType),
    /// Unknown chunk type.
    UnknownChunkType(u32),
}

/// Binary glTF contents.
#[derive(Clone, Debug)]
pub struct Glb<'a> {
    /// The header section of the `.glb` file.
    pub header: Header,
    /// The JSON section of the `.glb` file.
    pub json: Cow<'a, [u8]>,
    /// The optional BIN section of the `.glb` file.
    pub bin: Option<Cow<'a, [u8]>>,
}

/// The header section of a .glb file.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Header {
    /// Must be `b"glTF"`.
    pub magic: u32,
    /// Must be `2`.
    pub version: u32,
    /// Must match the length of the parent .glb file.
    pub length: u32,
}

/// GLB chunk type.
#[derive(Copy, Clone, Debug)]
pub enum ChunkType {
    /// `JSON` chunk.
    Json,
    /// `BIN` chunk.
    Bin,
}

/// Chunk header with no data read yet.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
struct ChunkHeader {
    /// The length of the chunk data in byte excluding the header.
    length: u32,
    /// Chunk type.
    ty: ChunkType,
}

impl Header {
    fn from_reader(mut data: &[u8]) -> Result<Self, Error> {
        let (prefix, words, suffix) = unsafe {
            data.align_to::<u32>()
        };
        assert!(prefix.is_empty() && suffix.is_empty());
        // We only validate magic as we don't care for version and length of
        // contents, the caller does.  Let them decide what to do next with
        // regard to version and length.
        if words[0] == 0x46546C67 {
            let output = Ok(Self {
                magic: words[0],
                version: words[1],
                length: words[2],
            });
            data = &data[12..];
            output
        } else {
            Err(Error::Magic(words[0]))
        }
    }

    fn size_of() -> usize {
        12
    }
}

impl ChunkHeader {
    fn from_reader(mut data: &[u8]) -> Result<Self, Error> {
        let (prefix, words, suffix) = unsafe {
            data.align_to::<u32>()
        };
        assert!(prefix.is_empty() && suffix.is_empty());
        let length = words[3];
        let ty = words[4];
        let ty = match &ty {
            0x4E4F534A => Ok(ChunkType::Json),
            0x004E4942 => Ok(ChunkType::Bin),
            _ => Err(Error::UnknownChunkType(ty)),
        }?;
        
        let output = Ok(Self { length, ty });
        data = &data[8..];
        output 
    }
}

fn split_binary_gltf(mut data: &[u8]) -> Result<(&[u8], Option<&[u8]>), Error> {
    let (json, data) = ChunkHeader::from_reader(data)
        .and_then(|json_h| {
            if let ChunkType::Json = json_h.ty {
                Ok(json_h)
            } else {
                Err(Error::ChunkType(json_h.ty))
            }
        })
        .and_then(|json_h| {
            if json_h.length as usize <= data.len() {
                Ok(json_h)
            } else {
                Err(Error::ChunkLength {
                    ty: json_h.ty,
                    length: json_h.length,
                    length_read: data.len(),
                })
            }
        })
        // We have verified that json_h.length is no greater than that of
        // data.len().
        .map(|json_h| data.split_at(json_h.length as usize))?;

    let bin = if !data.is_empty() {
        ChunkHeader::from_reader(data)
            .and_then(|bin_h| {
                if let ChunkType::Bin = bin_h.ty {
                    Ok(bin_h)
                } else {
                    Err(Error::ChunkType(bin_h.ty))
                }
            })
            .and_then(|bin_h| {
                if bin_h.length as usize <= data.len() {
                    Ok(bin_h)
                } else {
                    Err(Error::ChunkLength {
                        ty: bin_h.ty,
                        length: bin_h.length,
                        length_read: data.len(),
                    })
                }
            })
            // We have verified that bin_h.length is no greater than that
            // of data.len().
            .map(|bin_h| data.split_at(bin_h.length as usize))
            .map(|(x, _)| Some(x))?
    } else {
        None
    };
    Ok((json, bin))
}

impl<'a> Glb<'a> {
    /// Splits loaded GLB into its three chunks.
    ///
    /// * Mandatory GLB header.
    /// * Mandatory JSON chunk.
    /// * Optional BIN chunk.
    pub fn from_slice(data: &'a [u8]) -> Result<Self, crate::Error> {
        let header = Header::from_reader(data)
            .and_then(|header| {
                let contents_length = header.length as usize - Header::size_of();
                if contents_length <= data.len() { 
                    Ok(header)
                } else {
                    Err(Error::Length {
                        length: contents_length as u32,
                        length_read: data.len(),
                    })
                }
            })
            .map_err(crate::Error::Binary)?;
        match header.version {
            2 => split_binary_gltf(data)
                .map(|(json, bin)| Glb {
                    header,
                    json: json.into(),
                    bin: bin.map(Into::into),
                })
                .map_err(crate::Error::Binary),
            x => Err(crate::Error::Binary(Error::Version(x))),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Error::Version(_) => "unsupported version",
                Error::Magic(_) => "not glTF magic",
                Error::Length { .. } => "could not completely read the object",
                Error::ChunkLength { ty, .. } => match ty {
                    ChunkType::Json => "JSON chunk length exceeds that of slice",
                    ChunkType::Bin => "BIN\\0 chunk length exceeds that of slice",
                },
                Error::ChunkType(ty) => match ty {
                    ChunkType::Json => "was not expecting JSON chunk",
                    ChunkType::Bin => "was not expecting BIN\\0 chunk",
                },
                Error::UnknownChunkType(_) => "unknown chunk type",
            }
        )
    }
}

impl core::error::Error for Error {}
