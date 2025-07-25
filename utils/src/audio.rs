use base64::Engine;
use ringbuf::HeapRb;
use rubato::{FastFixedIn, PolynomialDegree};

pub const REALTIME_API_PCM16_SAMPLE_RATE: f64 = 24000.0;

//function to return a resampler object that helps us to change the sample rate of an audio sample
pub fn create_resampler(in_sampling_rate: f64, out_sampling_rate: f64, chunk_size: usize) -> anyhow::Result<FastFixedIn<f32>>
{
    let resampler = FastFixedIn::<f32>::new(
        out_sampling_rate / in_sampling_rate,
        1.0,
        PolynomialDegree::Cubic,
        chunk_size,
        1
    )?;
    Ok(resampler)
}

//in technical terms we are getting the iterator of the slice, using the mpa function to apply a function
//to each value in the iterator, then storing all values in the iterator as individual vectors inside a large vector
pub fn split_for_chunks(samples: &[f32], chunk_size: usize) -> Vec<Vec<f32>> {
    //split up our audi sample into chunks of a specified size, make each chink a vector, and zero out the vector
    //if the vector is not 0, then collect all of the vectors into a vector itself
    samples.chunks(chunk_size).map(|chunk| {
        let mut chunk = chunk.to_vec();
        chunk.resize(chunk_size, 0.0);
        chunk
    }).collect()
}

//create the ring buffer to store audio on the heap
pub fn shared_buffer(size: usize) -> HeapRb<f32> {
    HeapRb::new(size)
}

//decode an entire collection of audio responses
pub fn decode_all(fragments: Vec<String>) -> Vec<f32> {
    fragments.iter().flat_map(|fragment| {
        let decoded = decode(fragment);
        println!("decoded: {:?}", &decoded.len());
        decoded
    }).collect()
}

pub fn decode(base64_fragment: &str) -> Vec<f32> {
    decode_f32(base64_fragment)
}

//this function take the base64 string and converts it to a vector of floats 
//it takes the string makes it into a binary vector of u8, converts those chunks to i16 values, then 
//converts those i16 to float32 returning that array
pub fn decode_f32(base64_fragment: &str) -> Vec<f32> {
    if let Ok(pcm16) = base64::engine::general_purpose::STANDARD.decode(base64_fragment) {
        pcm16.chunks_exact(2).map(|chunk| {
            let v = i16::from_le_bytes([chunk[0], chunk[1]]);
            (v as f32 / 32768.0).clamp(-1.0, 1.0)
        }).collect()
    } else {
        tracing::error!("Failed to decode base64 fragment");
        Vec::new()
    }
}

//function to just decode to pcm 16 values
pub fn decode_i16(base64_fragment: &str) -> Vec<i16> {
    if let Ok(pcm16) = base64::engine::general_purpose::STANDARD.decode(base64_fragment) {
        pcm16.chunks_exact(2).map(|chunk| {
            i16::from_le_bytes([chunk[0], chunk[1]])
        }).collect()
    } else {
        tracing::error!("Failed to decode base64 fragment");
        Vec::new()
    }
}


pub fn encode(pcm32: &[f32]) -> String {
    encode_f32(pcm32)
}

pub fn encode_f32(pcm32: &[f32]) -> String {
    //turn pcm 32 into binary
    let pcm16: Vec<u8> = pcm32.to_binary();
    //return binary as base64 string
    base64::engine::general_purpose::STANDARD.encode(&pcm16)
}

pub fn encode_i16(pcm16: &[i16]) -> String {
    let pcm16: Vec<u8> = pcm16.to_binary();
    base64::engine::general_purpose::STANDARD.encode(&pcm16)
}

pub fn convert_f32_to_i16(pcm32: &[f32]) -> Vec<i16> {
    pcm32.iter().map(|&sample| {
        (sample * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16
    }).collect()
}

pub fn convert_i16_to_f32(pcm16: &[i16]) -> Vec<f32> {
    pcm16.iter().map(|&sample| {
        sample as f32 / 32768.0
    }).collect()
}

pub trait ToBinary {
    fn to_binary(&self) -> Vec<u8>;
}

impl ToBinary for [i16] {
    fn to_binary(&self) -> Vec<u8> {
        self.iter().flat_map(|&sample| sample.to_le_bytes().to_vec()).collect()
    }
}

impl ToBinary for [f32] {
    fn to_binary(&self) -> Vec<u8> {
        self.iter().flat_map(|&sample| {
            let v = (sample * 32768.0).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            v.to_le_bytes().to_vec()
        }).collect()
    }
}