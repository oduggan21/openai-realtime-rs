use base64::Engine;
use ringbuf::HeapRb;
use rubato::{FastFixedIn, PolynomialDegree};

pub const REALTIME_API_PCM16_SAMPLE_RATE: f64 = 24000.0;

pub fn create_resampler(in_sampling_rate: f64, out_sampling_rate: f64, chunk_size: usize) -> anyhow::Result<FastFixedIn<f32>> {
    let resampler = FastFixedIn::<f32>::new(
        out_sampling_rate / in_sampling_rate,
        1.0,
        PolynomialDegree::Cubic,
        chunk_size,
        1
    )?;
    Ok(resampler)
}

pub fn split_for_chunks(samples: &[f32], chunk_size: usize) -> Vec<Vec<f32>> {
    samples.chunks(chunk_size).map(|chunk| {
        let mut chunk = chunk.to_vec();
        chunk.resize(chunk_size, 0.0);
        chunk
    }).collect()
}

pub fn shared_buffer(size: usize) -> HeapRb<f32> {
    HeapRb::new(size)
}

pub fn decode_all(fragments: Vec<String>) -> Vec<f32> {
    fragments.iter().flat_map(|fragment| {
        let decoded = decode(fragment);
        println!("decoded: {:?}", &decoded.len());
        decoded
    }).collect()
}

pub fn decode(fragment: &str) -> Vec<f32> {
    if let Ok(pcm16) = base64::engine::general_purpose::STANDARD.decode(fragment) {
        pcm16.chunks_exact(2).map(|chunk| {
            let v = i16::from_le_bytes([chunk[0], chunk[1]]);
            (v as f32 / i16::MAX as f32).clamp(-1.0, 1.0)
        }).collect()
    } else {
        tracing::error!("Failed to decode base64 fragment");
        Vec::new()
    }
}

pub fn encode(pcm32: &[f32]) -> String {
    let pcm16: Vec<u8> = pcm32.iter().flat_map(|&sample| {
        ((sample * i16::MAX as f32) as i16).clamp(i16::MIN, i16::MAX).to_le_bytes().to_vec()
    }).collect();
    base64::engine::general_purpose::STANDARD.encode(&pcm16)
}