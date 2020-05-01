/* Reads wavetable files */

use super::{Wavetable, WavetableRef};
use super::Float;

use std::fs::File;
use std::io::{Read, BufReader};
use std::mem;

use log::{info, trace, warn};

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct ChunkHeader {
    chunk_id: [u8; 4],
    size: u32
}

pub struct WtReader {
    pub base_path: String,
}

impl WtReader {
    pub fn new(path: &str) -> Self {
        let mut base_path = path.to_string();
        let path_bytes = base_path.as_bytes();
        if path_bytes[path_bytes.len() - 1] != b'/' {
            base_path.push('/');
        }
        WtReader{base_path: base_path}
    }

    pub fn read_file(&self, filename: &str) -> Result<WavetableRef, ()> {
        let filename = self.base_path.clone() + filename;
        let file = File::open(filename).unwrap();
        let reader = BufReader::new(file);
        WtReader::read_wavetable(reader, 2048)
    }

    pub fn read_wavetable<R: Read>(mut source: R, samples_per_table: usize) -> Result<WavetableRef, ()> {
        // Read RIFF header
        let result = WtReader::read_header(&mut source);
        match result {
            Ok(header) => { WtReader::skip_chunk(&mut source, 4); }
            Err(()) => return Err(()),
        }
        // Read chunks
        loop {
            let result = WtReader::read_header(&mut source);
            match result {
                Ok(header) => {
                    unsafe { info!("Chunk ID: {:?}\nSize; {}", header.chunk_id, header.size) };
                    if header.chunk_id[0] == 'd' as u8
                    && header.chunk_id[1] == 'a' as u8
                    && header.chunk_id[2] == 't' as u8
                    && header.chunk_id[3] == 'a' as u8 {
                        // Found data section, create wavetable
                        let result = WtReader::read_samples(&mut source, header.size, samples_per_table);
                        let samples = if let Ok(s) = result { s } else { return Err(()) };
                        let num_tables = samples.len();
                        return Result::Ok(Wavetable::new_from_vector(num_tables, 1, samples_per_table, samples));
                    } else {
                        WtReader::skip_chunk(&mut source, header.size);
                    }
                }
                Err(()) => return Err(()),
            }
        }
        Result::Err(())
    }

    fn read_header<R: Read>(source: &mut R) -> Result<ChunkHeader, ()> {
        let mut header: ChunkHeader = unsafe { mem::zeroed() };
        let header_size = mem::size_of::<ChunkHeader>();
        unsafe {
            let header_slize = std::slice::from_raw_parts_mut(&mut header as *mut _ as *mut u8, header_size);
            source.read_exact(header_slize).unwrap();
        }
        info!("\nRead structure: {:#?}", header);
        Ok(header)
    }

    fn skip_chunk<R: Read>(source: &mut R, num_bytes: u32) {
        let mut buf: [u8; 1] = unsafe { mem::zeroed() };
        for i in 0..num_bytes {
            source.read(&mut buf).unwrap();
        }
    }

    /** Read samples into multiple tables.
     *
     * A file is assumed to hold multiple waveshapes, each with
     * <samples_per_table> values. One more value is added to the end of the
     * table automatically.
     *
     * Multiple octave tables per waveshape are currently not supported.
     */
    fn read_samples<R: Read>(source: &mut R, num_bytes: u32, samples_per_table: usize) -> Result<Vec<Vec<Float>>, ()> {
        let mut buf: f32 = unsafe { mem::zeroed() };
        let mut samples: Vec<Vec<Float>> = vec!{};
        let sample_size = mem::size_of::<f32>();
        let num_samples = num_bytes as usize / sample_size;
        let num_tables = num_samples / samples_per_table;
        if num_samples < samples_per_table || num_samples % samples_per_table != 0 {
            info!("Unexpected number of samples: {}", num_samples);
            return Err(());
        }
        info!("{} samples total, {} tables with {} values each", num_samples, num_tables, samples_per_table);
        for i in 0..num_tables {
            samples.push(vec!(0.0; samples_per_table + 1));
        }
        unsafe {
            let sample = std::slice::from_raw_parts_mut(&mut buf as *mut _ as *mut u8, sample_size);
            for i in 0..num_tables {
                let table = &mut samples[i];
                for j in 0..samples_per_table {
                    source.read_exact(sample).unwrap();
                    table[j] = buf as Float;
                }
                table[samples_per_table] = table[0]; // Duplicate first entry as last entry for easy interpolation
                Wavetable::normalize(table);
            }
        }
        Ok(samples)
    }
}


// ----------------------------------------------
//                  Unit tests
// ----------------------------------------------

struct TestContext {
}

impl TestContext {
    pub fn new() -> Self {
        TestContext{}
    }

    pub fn test(&mut self, ptr: &[u8]) -> bool {
        let reader = BufReader::new(ptr);
        let result = WtReader::read_wavetable(reader, 512);
        match result {
            Ok(x) => true,
            Err(()) => false
        }
    }
}

#[test]
fn single_wave_can_be_read() {
    let mut context = TestContext::new();
    assert!(context.test(SINGLE_WAVE));
}

#[test]
fn partial_wave_is_rejected() {
    let mut context = TestContext::new();
    assert!(!context.test(PARTIAL_WAVE));
}

#[test]
fn base_path_is_set_up_correctly() {
    let wtr = WtReader::new("NoSlash");
    assert!(wtr.base_path == "NoSlash/".to_string());

    let wtr = WtReader::new("WithSlash/");
    assert!(wtr.base_path == "WithSlash/".to_string());
}

const PARTIAL_WAVE: &[u8] = &[
    // RIFF/ WAVE header
    'R' as u8, 'I' as u8, 'F' as u8, 'F' as u8,
    0x04, 0x00, 0x00, 0x00,
    'W' as u8, 'A' as u8, 'V' as u8, 'E' as u8,
    // Junk chunk
    'j' as u8, 'u' as u8, 'n' as u8, 'k' as u8,
    0x04, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
    // TODO: fmt chunk
    // data chunk
    'd' as u8, 'a' as u8, 't' as u8, 'a' as u8,
    0x10, 0x00, 0x00, 0x00,
    0xa0, 0xc4, 0xb8, 0xba,
    0xa0, 0x54, 0xfc, 0x3a,
    0xf0, 0x0f, 0xaa, 0x3b,
    0x54, 0x2f, 0x0a, 0x3c,
];

const SINGLE_WAVE: &[u8] = &[
    'R' as u8, 'I' as u8, 'F' as u8, 'F' as u8,
    0x04, 0x00, 0x00, 0x00,
    'W' as u8, 'A' as u8, 'V' as u8, 'E' as u8,
    // Junk chunk
    'j' as u8, 'u' as u8, 'n' as u8, 'k' as u8,
    0x04, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
    // TODO: fmt chunk
    // data chunk
    'd' as u8, 'a' as u8, 't' as u8, 'a' as u8,
    0x00, 0x08, 0x00, 0x00,
    0xa0, 0xc4, 0xb8, 0xba, 0xa0, 0x54, 0xfc, 0x3a,
    0xf0, 0x0f, 0xaa, 0x3b, 0x54, 0x2f, 0x0a, 0x3c, 0x9c, 0x35, 0x40, 0x3c, 0x48, 0x43, 0x76, 0x3c,
    0x2a, 0x33, 0x96, 0x3c, 0x18, 0xb3, 0xb1, 0x3c, 0x1c, 0x50, 0xcd, 0x3c, 0xc5, 0x82, 0xe8, 0x3c,
    0xa8, 0x39, 0x01, 0x3d, 0x33, 0xdc, 0x0d, 0x3d, 0x3e, 0x6d, 0x1a, 0x3d, 0x8e, 0x5e, 0x27, 0x3d,
    0x44, 0x7a, 0x34, 0x3d, 0x8b, 0x5b, 0x41, 0x3d, 0xb6, 0x98, 0x4d, 0x3d, 0x25, 0x41, 0x59, 0x3d,
    0x98, 0x12, 0x65, 0x3d, 0x3c, 0xb0, 0x71, 0x3d, 0x23, 0xf3, 0x7e, 0x3d, 0xef, 0xfa, 0x85, 0x3d,
    0x19, 0xda, 0x8b, 0x3d, 0x7c, 0x23, 0x91, 0x3d, 0xf7, 0x76, 0x96, 0x3d, 0x46, 0x6f, 0x9c, 0x3d,
    0xe8, 0xe9, 0xa2, 0x3d, 0x0e, 0x66, 0xa9, 0x3d, 0x0b, 0x3f, 0xaf, 0x3d, 0x3c, 0x83, 0xb4, 0x3d,
    0x4a, 0xa0, 0xb9, 0x3d, 0xb2, 0xf8, 0xbe, 0x3d, 0x1e, 0x5b, 0xc4, 0x3d, 0x74, 0xcf, 0xc9, 0x3d,
    0xfe, 0x6f, 0xcf, 0x3d, 0xfa, 0x29, 0xd5, 0x3d, 0x1c, 0xe7, 0xda, 0x3d, 0x8f, 0x6e, 0xe0, 0x3d,
    0x4b, 0x9f, 0xe5, 0x3d, 0x8c, 0xb4, 0xea, 0x3d, 0x21, 0xcd, 0xef, 0x3d, 0x52, 0xd5, 0xf4, 0x3d,
    0x38, 0xc8, 0xf9, 0x3d, 0xcd, 0xf7, 0xfe, 0x3d, 0x4e, 0x25, 0x02, 0x3e, 0xe2, 0xcc, 0x04, 0x3e,
    0x96, 0x64, 0x07, 0x3e, 0x3a, 0xc8, 0x09, 0x3e, 0x08, 0x1c, 0x0c, 0x3e, 0xab, 0x7e, 0x0e, 0x3e,
    0x7e, 0xf7, 0x10, 0x3e, 0xe8, 0x6f, 0x13, 0x3e, 0xde, 0xdb, 0x15, 0x3e, 0xf0, 0x3c, 0x18, 0x3e,
    0xdb, 0x81, 0x1a, 0x3e, 0x30, 0xcf, 0x1c, 0x3e, 0x28, 0x0c, 0x1f, 0x3e, 0x19, 0x41, 0x21, 0x3e,
    0x36, 0x81, 0x23, 0x3e, 0x5e, 0xd1, 0x25, 0x3e, 0xa8, 0x30, 0x28, 0x3e, 0xc1, 0x70, 0x2a, 0x3e,
    0x78, 0x95, 0x2c, 0x3e, 0x5b, 0xa2, 0x2e, 0x3e, 0x59, 0xa7, 0x30, 0x3e, 0x84, 0xc0, 0x32, 0x3e,
    0x8e, 0xd2, 0x34, 0x3e, 0x17, 0xf1, 0x36, 0x3e, 0x54, 0x0f, 0x39, 0x3e, 0xf8, 0x46, 0x3b, 0x3e,
    0x2b, 0x67, 0x3d, 0x3e, 0xb2, 0x64, 0x3f, 0x3e, 0xc8, 0x65, 0x41, 0x3e, 0xcf, 0x43, 0x43, 0x3e,
    0x78, 0x1c, 0x45, 0x3e, 0x48, 0xe7, 0x46, 0x3e, 0x08, 0xbb, 0x48, 0x3e, 0x09, 0xbf, 0x4a, 0x3e,
    0xbd, 0xf1, 0x4c, 0x3e, 0xb0, 0x19, 0x4f, 0x3e, 0xc2, 0xd4, 0x50, 0x3e, 0xec, 0x6b, 0x52, 0x3e,
    0x1a, 0xf4, 0x53, 0x3e, 0xde, 0xb0, 0x55, 0x3e, 0x16, 0xb0, 0x57, 0x3e, 0x88, 0xa3, 0x59, 0x3e,
    0x82, 0x7f, 0x5b, 0x3e, 0xb9, 0x45, 0x5d, 0x3e, 0xde, 0x09, 0x5f, 0x3e, 0x42, 0xae, 0x60, 0x3e,
    0x6f, 0x42, 0x62, 0x3e, 0xde, 0xdf, 0x63, 0x3e, 0xba, 0x78, 0x65, 0x3e, 0x81, 0x43, 0x67, 0x3e,
    0xa2, 0xfb, 0x68, 0x3e, 0xc6, 0xa0, 0x6a, 0x3e, 0x61, 0x25, 0x6c, 0x3e, 0x4d, 0x9e, 0x6d, 0x3e,
    0x64, 0x2c, 0x6f, 0x3e, 0xe9, 0xb6, 0x70, 0x3e, 0xae, 0x53, 0x72, 0x3e, 0x24, 0xe5, 0x73, 0x3e,
    0x78, 0x85, 0x75, 0x3e, 0x0a, 0x1a, 0x77, 0x3e, 0x96, 0x96, 0x78, 0x3e, 0x63, 0xfa, 0x79, 0x3e,
    0x88, 0x36, 0x7b, 0x3e, 0xf4, 0x96, 0x7c, 0x3e, 0xa9, 0xee, 0x7d, 0x3e, 0x3d, 0x48, 0x7f, 0x3e,
    0x3b, 0x5a, 0x80, 0x3e, 0xa0, 0x16, 0x81, 0x3e, 0x6f, 0xd1, 0x81, 0x3e, 0x05, 0x86, 0x82, 0x3e,
    0xa8, 0x33, 0x83, 0x3e, 0xa2, 0xcb, 0x83, 0x3e, 0x32, 0x6e, 0x84, 0x3e, 0xa1, 0x10, 0x85, 0x3e,
    0x8f, 0xa5, 0x85, 0x3e, 0x05, 0x47, 0x86, 0x3e, 0x63, 0xe6, 0x86, 0x3e, 0x74, 0x81, 0x87, 0x3e,
    0x91, 0x1c, 0x88, 0x3e, 0x80, 0xb9, 0x88, 0x3e, 0x18, 0x47, 0x89, 0x3e, 0xbc, 0xdd, 0x89, 0x3e,
    0xbf, 0x7c, 0x8a, 0x3e, 0xce, 0x12, 0x8b, 0x3e, 0xcb, 0xaf, 0x8b, 0x3e, 0xea, 0x46, 0x8c, 0x3e,
    0xec, 0xd7, 0x8c, 0x3e, 0x72, 0x63, 0x8d, 0x3e, 0xd1, 0xde, 0x8d, 0x3e, 0x10, 0x48, 0x8e, 0x3e,
    0xcd, 0xbb, 0x8e, 0x3e, 0x02, 0x53, 0x8f, 0x3e, 0x50, 0xfc, 0x8f, 0x3e, 0xb5, 0xa4, 0x90, 0x3e,
    0x10, 0x23, 0x91, 0x3e, 0xa9, 0x8a, 0x91, 0x3e, 0xb3, 0xf0, 0x91, 0x3e, 0x0a, 0x63, 0x92, 0x3e,
    0xf9, 0xe3, 0x92, 0x3e, 0x3b, 0x65, 0x93, 0x3e, 0x99, 0xdf, 0x93, 0x3e, 0x7a, 0x5c, 0x94, 0x3e,
    0x20, 0xd7, 0x94, 0x3e, 0xfa, 0x3b, 0x95, 0x3e, 0x42, 0x98, 0x95, 0x3e, 0x4c, 0xf7, 0x95, 0x3e,
    0x8a, 0x5a, 0x96, 0x3e, 0xfc, 0xd8, 0x96, 0x3e, 0x26, 0x57, 0x97, 0x3e, 0xf6, 0xc6, 0x97, 0x3e,
    0xc6, 0x37, 0x98, 0x3e, 0xc6, 0xa5, 0x98, 0x3e, 0x2c, 0x01, 0x99, 0x3e, 0x44, 0x59, 0x99, 0x3e,
    0x72, 0xb0, 0x99, 0x3e, 0x00, 0x04, 0x9a, 0x3e, 0xe2, 0x6a, 0x9a, 0x3e, 0x5a, 0xd2, 0x9a, 0x3e,
    0x90, 0x31, 0x9b, 0x3e, 0xe0, 0x97, 0x9b, 0x3e, 0x4a, 0xfb, 0x9b, 0x3e, 0xb3, 0x52, 0x9c, 0x3e,
    0xd1, 0xa3, 0x9c, 0x3e, 0x2b, 0xf3, 0x9c, 0x3e, 0xf4, 0x3b, 0x9d, 0x3e, 0xa8, 0x90, 0x9d, 0x3e,
    0x78, 0xe1, 0x9d, 0x3e, 0xab, 0x31, 0x9e, 0x3e, 0xd6, 0x86, 0x9e, 0x3e, 0x5a, 0xd8, 0x9e, 0x3e,
    0x10, 0x26, 0x9f, 0x3e, 0x74, 0x67, 0x9f, 0x3e, 0x46, 0xa5, 0x9f, 0x3e, 0x78, 0xe9, 0x9f, 0x3e,
    0xd8, 0x44, 0xa0, 0x3e, 0x6a, 0xa4, 0xa0, 0x3e, 0xbb, 0xf5, 0xa0, 0x3e, 0x08, 0x2a, 0xa1, 0x3e,
    0x02, 0x45, 0xa1, 0x3e, 0x25, 0x75, 0xa1, 0x3e, 0x13, 0xbf, 0xa1, 0x3e, 0x90, 0x18, 0xa2, 0x3e,
    0x39, 0x6f, 0xa2, 0x3e, 0x6c, 0xb5, 0xa2, 0x3e, 0xf6, 0xf1, 0xa2, 0x3e, 0xec, 0x26, 0xa3, 0x3e,
    0x45, 0x50, 0xa3, 0x3e, 0x44, 0x64, 0xa3, 0x3e, 0xdb, 0x8b, 0xa3, 0x3e, 0xc1, 0xd8, 0xa3, 0x3e,
    0x4c, 0x3d, 0xa4, 0x3e, 0x67, 0x99, 0xa4, 0x3e, 0x53, 0xbc, 0xa4, 0x3e, 0x1a, 0xbb, 0xa4, 0x3e,
    0x73, 0xb8, 0xa4, 0x3e, 0xe2, 0xd9, 0xa4, 0x3e, 0xc0, 0x08, 0xa5, 0x3e, 0x91, 0x41, 0xa5, 0x3e,
    0xe0, 0x81, 0xa5, 0x3e, 0x78, 0xc9, 0xa5, 0x3e, 0x7c, 0x0c, 0xa6, 0x3e, 0x78, 0x30, 0xa6, 0x3e,
    0xf8, 0x3a, 0xa6, 0x3e, 0x5d, 0x3f, 0xa6, 0x3e, 0xb1, 0x56, 0xa6, 0x3e, 0x36, 0x7c, 0xa6, 0x3e,
    0x86, 0xa1, 0xa6, 0x3e, 0x75, 0xcc, 0xa6, 0x3e, 0xf2, 0xf7, 0xa6, 0x3e, 0xeb, 0x22, 0xa7, 0x3e,
    0x1d, 0x45, 0xa7, 0x3e, 0xea, 0x61, 0xa7, 0x3e, 0x86, 0x78, 0xa7, 0x3e, 0x48, 0x91, 0xa7, 0x3e,
    0xac, 0xae, 0xa7, 0x3e, 0xde, 0xc6, 0xa7, 0x3e, 0xb3, 0xdf, 0xa7, 0x3e, 0x22, 0xf7, 0xa7, 0x3e,
    0xf1, 0x0a, 0xa8, 0x3e, 0x84, 0x1c, 0xa8, 0x3e, 0x61, 0x29, 0xa8, 0x3e, 0x2c, 0x35, 0xa8, 0x3e,
    0x92, 0x43, 0xa8, 0x3e, 0x42, 0x5c, 0xa8, 0x3e, 0x09, 0x74, 0xa8, 0x3e, 0x70, 0x8c, 0xa8, 0x3e,
    0x00, 0xa5, 0xa8, 0x3e, 0x2c, 0xbc, 0xa8, 0x3e, 0x6c, 0xc9, 0xa8, 0x3e, 0xde, 0xc3, 0xa8, 0x3e,
    0xed, 0xb8, 0xa8, 0x3e, 0x3d, 0xb9, 0xa8, 0x3e, 0x78, 0xd2, 0xa8, 0x3e, 0x54, 0xf6, 0xa8, 0x3e,
    0xea, 0x10, 0xa9, 0x3e, 0x56, 0x19, 0xa9, 0x3e, 0x2e, 0x19, 0xa9, 0x3e, 0x75, 0x19, 0xa9, 0x3e,
    0xb6, 0x17, 0xa9, 0x3e, 0x2a, 0x18, 0xa9, 0x3e, 0x86, 0x18, 0xa9, 0x3e, 0x3e, 0x1d, 0xa9, 0x3e,
    0x9c, 0x24, 0xa9, 0x3e, 0x3e, 0x29, 0xa9, 0x3e, 0xf7, 0x23, 0xa9, 0x3e, 0x88, 0x1a, 0xa9, 0x3e,
    0x43, 0x12, 0xa9, 0x3e, 0xfc, 0x0c, 0xa9, 0x3e, 0xfe, 0x0a, 0xa9, 0x3e, 0x86, 0x09, 0xa9, 0x3e,
    0x86, 0x07, 0xa9, 0x3e, 0x48, 0x07, 0xa9, 0x3e, 0x60, 0x03, 0xa9, 0x3e, 0x22, 0xf6, 0xa8, 0x3e,
    0x12, 0xe0, 0xa8, 0x3e, 0xc6, 0xcd, 0xa8, 0x3e, 0x02, 0xc4, 0xa8, 0x3e, 0x2a, 0xc5, 0xa8, 0x3e,
    0x68, 0xc3, 0xa8, 0x3e, 0x48, 0xb8, 0xa8, 0x3e, 0xbe, 0xa3, 0xa8, 0x3e, 0x86, 0x8f, 0xa8, 0x3e,
    0xe4, 0x82, 0xa8, 0x3e, 0x6e, 0x7b, 0xa8, 0x3e, 0x0e, 0x76, 0xa8, 0x3e, 0x48, 0x6a, 0xa8, 0x3e,
    0xd0, 0x5c, 0xa8, 0x3e, 0x62, 0x4c, 0xa8, 0x3e, 0xee, 0x34, 0xa8, 0x3e, 0xec, 0x17, 0xa8, 0x3e,
    0xde, 0xf7, 0xa7, 0x3e, 0x42, 0xe0, 0xa7, 0x3e, 0x82, 0xca, 0xa7, 0x3e, 0xe6, 0xba, 0xa7, 0x3e,
    0x52, 0xac, 0xa7, 0x3e, 0x2f, 0xa3, 0xa7, 0x3e, 0xe9, 0x9b, 0xa7, 0x3e, 0xf4, 0x89, 0xa7, 0x3e,
    0x66, 0x6f, 0xa7, 0x3e, 0x04, 0x50, 0xa7, 0x3e, 0x46, 0x31, 0xa7, 0x3e, 0xfa, 0x10, 0xa7, 0x3e,
    0x4d, 0xf0, 0xa6, 0x3e, 0x28, 0xd4, 0xa6, 0x3e, 0x64, 0xb8, 0xa6, 0x3e, 0x42, 0xa1, 0xa6, 0x3e,
    0xff, 0x83, 0xa6, 0x3e, 0x70, 0x5e, 0xa6, 0x3e, 0x0a, 0x37, 0xa6, 0x3e, 0x3a, 0x16, 0xa6, 0x3e,
    0x5c, 0x00, 0xa6, 0x3e, 0x8e, 0xee, 0xa5, 0x3e, 0xc6, 0xd7, 0xa5, 0x3e, 0xab, 0xb2, 0xa5, 0x3e,
    0xb2, 0x87, 0xa5, 0x3e, 0x78, 0x61, 0xa5, 0x3e, 0xcf, 0x3f, 0xa5, 0x3e, 0xf3, 0x21, 0xa5, 0x3e,
    0x6d, 0x02, 0xa5, 0x3e, 0x39, 0xe0, 0xa4, 0x3e, 0x36, 0xbb, 0xa4, 0x3e, 0xf6, 0x94, 0xa4, 0x3e,
    0x98, 0x6a, 0xa4, 0x3e, 0x4d, 0x3d, 0xa4, 0x3e, 0x20, 0x0f, 0xa4, 0x3e, 0x9f, 0xe0, 0xa3, 0x3e,
    0xd5, 0xae, 0xa3, 0x3e, 0x75, 0x86, 0xa3, 0x3e, 0x96, 0x75, 0xa3, 0x3e, 0x1f, 0x76, 0xa3, 0x3e,
    0xf4, 0x68, 0xa3, 0x3e, 0x08, 0x36, 0xa3, 0x3e, 0x4c, 0xe7, 0xa2, 0x3e, 0xd3, 0x99, 0xa2, 0x3e,
    0xf0, 0x6a, 0xa2, 0x3e, 0xeb, 0x50, 0xa2, 0x3e, 0xd1, 0x33, 0xa2, 0x3e, 0x36, 0x0f, 0xa2, 0x3e,
    0x9b, 0xe4, 0xa1, 0x3e, 0x7c, 0xb4, 0xa1, 0x3e, 0xe4, 0x80, 0xa1, 0x3e, 0xde, 0x49, 0xa1, 0x3e,
    0x86, 0x10, 0xa1, 0x3e, 0xc1, 0xdd, 0xa0, 0x3e, 0x58, 0xb5, 0xa0, 0x3e, 0xcc, 0x88, 0xa0, 0x3e,
    0x0c, 0x56, 0xa0, 0x3e, 0xb8, 0x21, 0xa0, 0x3e, 0xd6, 0xe6, 0x9f, 0x3e, 0xc8, 0xb0, 0x9f, 0x3e,
    0x4a, 0x7b, 0x9f, 0x3e, 0xa8, 0x47, 0x9f, 0x3e, 0xa4, 0x15, 0x9f, 0x3e, 0xe5, 0xea, 0x9e, 0x3e,
    0xe2, 0xc0, 0x9e, 0x3e, 0x44, 0x8e, 0x9e, 0x3e, 0x32, 0x5a, 0x9e, 0x3e, 0x72, 0x21, 0x9e, 0x3e,
    0x7f, 0xe7, 0x9d, 0x3e, 0xe6, 0xab, 0x9d, 0x3e, 0x04, 0x6f, 0x9d, 0x3e, 0xd8, 0x3f, 0x9d, 0x3e,
    0x9a, 0x1e, 0x9d, 0x3e, 0xf8, 0x04, 0x9d, 0x3e, 0xda, 0xd5, 0x9c, 0x3e, 0xf4, 0x87, 0x9c, 0x3e,
    0x69, 0x32, 0x9c, 0x3e, 0xc4, 0xe7, 0x9b, 0x3e, 0x64, 0xb3, 0x9b, 0x3e, 0x63, 0x85, 0x9b, 0x3e,
    0xf8, 0x5d, 0x9b, 0x3e, 0xd6, 0x3f, 0x9b, 0x3e, 0xf8, 0x20, 0x9b, 0x3e, 0x98, 0xf0, 0x9a, 0x3e,
    0xc6, 0x85, 0x9a, 0x3e, 0x3e, 0x0a, 0x9a, 0x3e, 0xbc, 0xa9, 0x99, 0x3e, 0xd1, 0x89, 0x99, 0x3e,
    0xdd, 0x87, 0x99, 0x3e, 0x18, 0x73, 0x99, 0x3e, 0x88, 0x30, 0x99, 0x3e, 0x23, 0xce, 0x98, 0x3e,
    0x3c, 0x7a, 0x98, 0x3e, 0x45, 0x3a, 0x98, 0x3e, 0xb3, 0x02, 0x98, 0x3e, 0x0d, 0xd3, 0x97, 0x3e,
    0xd5, 0x9d, 0x97, 0x3e, 0xfc, 0x6e, 0x97, 0x3e, 0xfe, 0x34, 0x97, 0x3e, 0x2d, 0xf1, 0x96, 0x3e,
    0x7a, 0x9b, 0x96, 0x3e, 0xc8, 0x4a, 0x96, 0x3e, 0x68, 0x0c, 0x96, 0x3e, 0x1e, 0xd0, 0x95, 0x3e,
    0x29, 0x9c, 0x95, 0x3e, 0x12, 0x61, 0x95, 0x3e, 0xa0, 0x27, 0x95, 0x3e, 0x00, 0xea, 0x94, 0x3e,
    0x18, 0xa1, 0x94, 0x3e, 0x92, 0x51, 0x94, 0x3e, 0xc2, 0xfd, 0x93, 0x3e, 0xd6, 0xbb, 0x93, 0x3e,
    0xc9, 0x7d, 0x93, 0x3e, 0x62, 0x45, 0x93, 0x3e, 0xd4, 0x08, 0x93, 0x3e, 0xb7, 0xcb, 0x92, 0x3e,
    0x72, 0x8e, 0x92, 0x3e, 0xf0, 0x42, 0x92, 0x3e, 0x86, 0xf0, 0x91, 0x3e, 0x15, 0x9c, 0x91, 0x3e,
    0x39, 0x54, 0x91, 0x3e, 0x46, 0x1e, 0x91, 0x3e, 0x12, 0xe8, 0x90, 0x3e, 0xe6, 0xa5, 0x90, 0x3e,
    0xa8, 0x52, 0x90, 0x3e, 0x84, 0x00, 0x90, 0x3e, 0x25, 0xaf, 0x8f, 0x3e, 0x44, 0x64, 0x8f, 0x3e,
    0xc1, 0x1f, 0x8f, 0x3e, 0xb3, 0xe0, 0x8e, 0x3e, 0x4f, 0xb5, 0x8e, 0x3e, 0xb6, 0x8c, 0x8e, 0x3e,
    0x0f, 0x51, 0x8e, 0x3e, 0x72, 0xfe, 0x8d, 0x3e, 0x8e, 0xa3, 0x8d, 0x3e, 0x3b, 0x4a, 0x8d, 0x3e,
    0x63, 0xef, 0x8c, 0x3e, 0x3e, 0x9b, 0x8c, 0x3e, 0x90, 0x50, 0x8c, 0x3e, 0xa7, 0x21, 0x8c, 0x3e,
    0x67, 0xfc, 0x8b, 0x3e, 0xa1, 0xc3, 0x8b, 0x3e, 0x1a, 0x67, 0x8b, 0x3e, 0x0c, 0xfd, 0x8a, 0x3e,
    0x4e, 0x9f, 0x8a, 0x3e, 0xf2, 0x5b, 0x8a, 0x3e, 0xb6, 0x25, 0x8a, 0x3e, 0xe3, 0xea, 0x89, 0x3e,
    0x30, 0xa5, 0x89, 0x3e, 0x6a, 0x5c, 0x89, 0x3e, 0xb2, 0x0e, 0x89, 0x3e, 0xfe, 0xbc, 0x88, 0x3e,
    0xd6, 0x65, 0x88, 0x3e, 0x12, 0x13, 0x88, 0x3e, 0x70, 0xc9, 0x87, 0x3e, 0x96, 0x85, 0x87, 0x3e,
    0x47, 0x3e, 0x87, 0x3e, 0xba, 0xed, 0x86, 0x3e, 0x3d, 0x96, 0x86, 0x3e, 0x14, 0x42, 0x86, 0x3e,
    0x14, 0xfb, 0x85, 0x3e, 0x77, 0xbb, 0x85, 0x3e, 0xd6, 0x7a, 0x85, 0x3e, 0xca, 0x38, 0x85, 0x3e,
    0x3d, 0xf3, 0x84, 0x3e, 0x40, 0xaa, 0x84, 0x3e, 0x7b, 0x59, 0x84, 0x3e, 0x87, 0x00, 0x84, 0x3e,
    0x48, 0xa7, 0x83, 0x3e, 0x58, 0x5a, 0x83, 0x3e, 0x04, 0x14, 0x83, 0x3e, 0x49, 0xce, 0x82, 0x3e,
    0x94, 0x89, 0x82, 0x3e, 0xf7, 0x44, 0x82, 0x3e, 0x8a, 0xfc, 0x81, 0x3e, 0x41, 0xb0, 0x81, 0x3e,
    0x40, 0x5b, 0x81, 0x3e, 0x2c, 0x05, 0x81, 0x3e, 0xef, 0xb2, 0x80, 0x3e, 0xdc, 0x63, 0x80, 0x3e,
    0x87, 0x14, 0x80, 0x3e, 0x42, 0x96, 0x7f, 0x3e, 0x11, 0x11, 0x7f, 0x3e, 0x8e, 0x85, 0x7e, 0x3e,
    0xd3, 0xdf, 0x7d, 0x3e, 0xca, 0x0e, 0x7d, 0x3e, 0x33, 0x3a, 0x7c, 0x3e, 0x77, 0x96, 0x7b, 0x3e,
    0xd8, 0x32, 0x7b, 0x3e, 0xee, 0xdc, 0x7a, 0x3e, 0x82, 0x56, 0x7a, 0x3e, 0x8b, 0x93, 0x79, 0x3e,
    0x5c, 0xba, 0x78, 0x3e, 0xe2, 0xfd, 0x77, 0x3e, 0x09, 0x64, 0x77, 0x3e, 0x10, 0xd9, 0x76, 0x3e,
    0x79, 0x49, 0x76, 0x3e, 0xac, 0xb9, 0x75, 0x3e, 0xfa, 0x27, 0x75, 0x3e, 0xa8, 0x90, 0x74, 0x3e,
    0x2e, 0xf2, 0x73, 0x3e, 0xb0, 0x50, 0x73, 0x3e, 0x0c, 0xa7, 0x72, 0x3e, 0xe7, 0xef, 0x71, 0x3e,
    0x9e, 0x35, 0x71, 0x3e, 0xf5, 0x8f, 0x70, 0x3e, 0x3b, 0x1d, 0x70, 0x3e, 0x66, 0xc2, 0x6f, 0x3e,
    0x9b, 0x47, 0x6f, 0x3e, 0x84, 0x8d, 0x6e, 0x3e, 0xac, 0xaf, 0x6d, 0x3e, 0x6d, 0xe3, 0x6c, 0x3e,
    0x0c, 0x3e, 0x6c, 0x3e, 0xd6, 0xb2, 0x6b, 0x3e, 0x3a, 0x26, 0x6b, 0x3e, 0x54, 0x96, 0x6a, 0x3e,
    0x2c, 0x08, 0x6a, 0x3e, 0x9f, 0x70, 0x69, 0x3e, 0x88, 0xc7, 0x68, 0x3e, 0x88, 0x0d, 0x68, 0x3e,
    0xd7, 0x5b, 0x67, 0x3e, 0x53, 0xb9, 0x66, 0x3e, 0x20, 0x21, 0x66, 0x3e, 0xd8, 0x8f, 0x65, 0x3e,
    0xc8, 0xf6, 0x64, 0x3e, 0x4d, 0x63, 0x64, 0x3e, 0x2f, 0xc8, 0x63, 0x3e, 0x01, 0x25, 0x63, 0x3e,
    0xb4, 0x6e, 0x62, 0x3e, 0x9f, 0xc2, 0x61, 0x3e, 0xd6, 0x28, 0x61, 0x3e, 0xe4, 0x9b, 0x60, 0x3e,
    0x8a, 0x15, 0x60, 0x3e, 0x64, 0x7b, 0x5f, 0x3e, 0x82, 0xd7, 0x5e, 0x3e, 0x25, 0x2b, 0x5e, 0x3e,
    0xae, 0x70, 0x5d, 0x3e, 0x5f, 0x96, 0x5c, 0x3e, 0x47, 0xc9, 0x5b, 0x3e, 0x7b, 0x40, 0x5b, 0x3e,
];
