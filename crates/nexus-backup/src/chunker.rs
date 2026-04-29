use crate::error::BackupError;
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(Debug, Clone, Copy)]
pub struct ChunkerParams {
    pub min_size: u32,
    pub avg_size: u32,
    pub max_size: u32,
}

impl Default for ChunkerParams {
    fn default() -> Self {
        Self { min_size: 4 * 1024, avg_size: 64 * 1024, max_size: 1024 * 1024 }
    }
}

pub struct Chunk {
    pub plaintext_offset: u64,
    pub plaintext_length: u32,
    pub plaintext_bytes: Vec<u8>,
}

pub struct Chunker<R> {
    reader: R,
    params: ChunkerParams,
    buf: Vec<u8>,
    offset: u64,
    eof: bool,
}

impl<R: AsyncRead + Unpin> Chunker<R> {
    pub fn new(reader: R, params: ChunkerParams) -> Self {
        Self {
            reader,
            params,
            buf: Vec::with_capacity(params.max_size as usize * 2),
            offset: 0,
            eof: false,
        }
    }

    async fn fill_until(&mut self, target: usize) -> Result<(), BackupError> {
        while self.buf.len() < target && !self.eof {
            let mut tmp = vec![0u8; (target - self.buf.len()).max(64 * 1024)];
            let n = self.reader.read(&mut tmp).await?;
            if n == 0 { self.eof = true; break; }
            tmp.truncate(n);
            self.buf.extend_from_slice(&tmp);
        }
        Ok(())
    }

    pub async fn next_chunk(&mut self) -> Result<Option<Chunk>, BackupError> {
        self.fill_until(self.params.max_size as usize).await?;
        if self.buf.is_empty() { return Ok(None); }

        let cdc = fastcdc::v2020::FastCDC::new(
            &self.buf,
            self.params.min_size,
            self.params.avg_size,
            self.params.max_size,
        );
        let first = cdc.into_iter().next();
        let cut_at = match first {
            Some(chunk_meta) => chunk_meta.length,
            None => self.buf.len(),
        };

        let bytes: Vec<u8> = self.buf.drain(..cut_at).collect();
        let chunk = Chunk {
            plaintext_offset: self.offset,
            plaintext_length: bytes.len() as u32,
            plaintext_bytes: bytes,
        };
        self.offset += chunk.plaintext_length as u64;
        Ok(Some(chunk))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::BufReader;

    fn deterministic_payload(size: usize) -> Vec<u8> {
        let mut v = vec![0u8; size];
        let mut s: u64 = 0xdeadbeefu64;
        for byte in v.iter_mut() {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *byte = (s >> 33) as u8;
        }
        v
    }

    #[tokio::test]
    async fn chunks_emit_in_order_and_cover_input() {
        let payload = deterministic_payload(1_500_000);
        let reader = BufReader::new(&payload[..]);
        let mut c = Chunker::new(reader, ChunkerParams::default());
        let mut total = 0u64;
        let mut last_offset: i64 = -1;
        while let Some(chunk) = c.next_chunk().await.unwrap() {
            assert!(chunk.plaintext_offset as i64 > last_offset);
            assert_eq!(chunk.plaintext_bytes.len(), chunk.plaintext_length as usize);
            assert_eq!(chunk.plaintext_offset, total);
            total += chunk.plaintext_length as u64;
            last_offset = chunk.plaintext_offset as i64;
        }
        assert_eq!(total, payload.len() as u64);
    }

    #[tokio::test]
    async fn deterministic_chunking_same_input() {
        let payload = deterministic_payload(800_000);
        let mut c1 = Chunker::new(BufReader::new(&payload[..]), ChunkerParams::default());
        let mut c2 = Chunker::new(BufReader::new(&payload[..]), ChunkerParams::default());

        let mut h1 = Vec::new();
        let mut h2 = Vec::new();
        while let Some(chunk) = c1.next_chunk().await.unwrap() { h1.push(blake3::hash(&chunk.plaintext_bytes)); }
        while let Some(chunk) = c2.next_chunk().await.unwrap() { h2.push(blake3::hash(&chunk.plaintext_bytes)); }
        assert_eq!(h1, h2, "FastCDC must be deterministic for the same input");
    }

    #[tokio::test]
    async fn empty_input_yields_no_chunks() {
        let payload: Vec<u8> = Vec::new();
        let mut c = Chunker::new(BufReader::new(&payload[..]), ChunkerParams::default());
        assert!(c.next_chunk().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn small_input_yields_single_chunk() {
        let payload = deterministic_payload(2 * 1024);
        let mut c = Chunker::new(BufReader::new(&payload[..]), ChunkerParams::default());
        let chunk = c.next_chunk().await.unwrap().expect("one chunk");
        assert_eq!(chunk.plaintext_length as usize, payload.len());
        assert!(c.next_chunk().await.unwrap().is_none());
    }
}
