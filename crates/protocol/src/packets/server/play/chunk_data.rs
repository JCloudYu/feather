use std::fmt::{self, Debug};
use std::sync::Arc;

use base::{Chunk, ChunkSection};
use parking_lot::RwLock;
use serde::Serialize;

use crate::{io::VarInt, Nbt, ProtocolVersion, Readable, Writeable};

#[derive(Serialize)]
struct Heightmaps {
    #[serde(rename = "MOTION_BLOCKING")]
    #[serde(serialize_with = "nbt::i64_array")]
    motion_blocking: [i64; 37],
}

/// Packet to load a chunk on the client.
#[derive(Clone)]
pub struct ChunkData {
    /// The chunk to send.
    pub chunk: Arc<RwLock<Chunk>>,
}

impl Debug for ChunkData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("ChunkData");
        debug_struct.field("position", &self.chunk.read().position());
        debug_struct.finish()
    }
}

impl Writeable for ChunkData {
    fn write(&self, buffer: &mut Vec<u8>, version: ProtocolVersion) {
        let chunk = self.chunk.read();

        chunk.position().x.write(buffer, version);
        chunk.position().z.write(buffer, version);

        // full chunk (always true for us)
        true.write(buffer, version);

        // Compute primary bit mask
        let mut bitmask = 0;
        for (y, section) in chunk.sections().iter().enumerate() {
            if section.is_some() {
                bitmask |= 1 << y as i32;
            }
        }
        VarInt(bitmask).write(buffer, version);

        let heightmaps = build_heightmaps(&chunk);
        Nbt(heightmaps).write(buffer, version);

        VarInt(1024).write(buffer, version); // length of biomes
        for &biome in chunk.biomes().as_slice() {
            VarInt(biome.id() as i32).write(buffer, version);
        }

        // Sections
        let mut data = Vec::new();
        for section in chunk.sections() {
            if let Some(section) = section {
                encode_section(section, &mut data, version);
            }
        }
        VarInt(data.len() as i32).write(buffer, version);
        buffer.extend_from_slice(&data);

        VarInt(0).write(buffer, version); // number of block entities - always 0 for Feather
    }
}

fn build_heightmaps(chunk: &Chunk) -> Heightmaps {
    let mut motion_blocking = [0; 37];
    let chunk_motion_blocking = chunk.heightmaps().motion_blocking.as_u64_slice();
    motion_blocking.copy_from_slice(bytemuck::cast_slice::<_, i64>(chunk_motion_blocking));
    Heightmaps { motion_blocking }
}

fn encode_section(section: &ChunkSection, buffer: &mut Vec<u8>, version: ProtocolVersion) {
    (section.non_air_blocks() as u16).write(buffer, version);
    (section.blocks().data().bits_per_value() as u8).write(buffer, version);

    if let Some(palette) = section.blocks().palette() {
        VarInt(palette.len() as i32).write(buffer, version);
        for &block in palette.as_slice() {
            VarInt(block.vanilla_id() as i32).write(buffer, version);
        }
    }

    let data = section.blocks().data().as_u64_slice();
    VarInt(data.len() as i32).write(buffer, version);
    for &x in data {
        x.write(buffer, version);
    }
}

impl Readable for ChunkData {
    fn read(_buffer: &mut std::io::Cursor<&[u8]>, _version: ProtocolVersion) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        todo!()
    }
}
