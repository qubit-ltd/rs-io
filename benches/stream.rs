use std::io::Cursor;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use qubit_io::{
    BinaryReadExt, BinaryReader, BinaryWriteExt, BinaryWriter, Leb128ReadExt, Leb128Reader,
    Leb128WriteExt, Leb128Writer, LittleEndian, NonStrict, ZigZagReadExt, ZigZagReader,
    ZigZagWriteExt, ZigZagWriter,
};

const BINARY_BATCH: usize = 1_048_576;
const BINARY_REPEAT: usize = 512;
const VARINT_COUNT: usize = 1_048_576;
const VARINT_REPEAT: usize = 512;

#[derive(Clone, Copy)]
struct Record {
    id: u64,
    user_id: u32,
    flag: u8,
    delta: i64,
    score: f32,
    weight: f64,
    ts_ms: u64,
}

#[derive(Clone, Copy)]
struct PseudoRng {
    state: u64,
    has_normal_cache: bool,
    normal_cache: f64,
}

impl PseudoRng {
    const fn new(seed: u64) -> Self {
        Self {
            state: seed,
            has_normal_cache: false,
            normal_cache: 0.0,
        }
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        let mut z = self.state;
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    #[inline]
    fn next_unit_f64(&mut self) -> f64 {
        ((self.next_u64() as f64) + 1.0) / ((u64::MAX as f64) + 2.0)
    }

    #[inline]
    fn next_normal_f64(&mut self) -> f64 {
        if self.has_normal_cache {
            self.has_normal_cache = false;
            return self.normal_cache;
        }

        loop {
            let u1 = self.next_unit_f64();
            let u2 = self.next_unit_f64();
            if u1 > 0.0 {
                let magnitude = (-2.0 * u1.ln()).sqrt();
                let angle = std::f64::consts::PI * 2.0 * u2;
                self.has_normal_cache = true;
                self.normal_cache = magnitude * angle.sin();
                return magnitude * angle.cos();
            }
        }
    }

    #[inline]
    fn next_normal_u64(&mut self, mean: f64, stddev: f64) -> u64 {
        let mut sample = self.next_normal_f64() * stddev + mean;
        if sample.is_nan() {
            sample = mean;
        }
        if sample <= 0.0 {
            0
        } else if sample >= (u64::MAX as f64) {
            u64::MAX
        } else {
            sample.round() as u64
        }
    }

    #[inline]
    fn next_normal_i64(&mut self, mean: f64, stddev: f64) -> i64 {
        let mut sample = self.next_normal_f64() * stddev + mean;
        if sample.is_nan() {
            sample = mean;
        }
        sample = sample.clamp(i64::MIN as f64, i64::MAX as f64);
        sample.round() as i64
    }

    #[inline]
    fn gen_record(&mut self, idx: u64) -> Record {
        let id_noise = self.next_normal_u64(2_000_000.0, 150_000.0);
        let user_noise = self.next_normal_u64(200_000.0, 40_000.0);
        let delta = self.next_normal_i64(0.0, 5_000_000.0);
        let score_noise = self.next_normal_f64() * 0.25;
        let weight_noise = self.next_normal_f64() * 50.0;

        Record {
            id: idx.wrapping_mul(1_000_003) ^ id_noise,
            user_id: (user_noise as u32).wrapping_add(1_024),
            flag: (idx.wrapping_add(self.next_u64()) % 8) as u8,
            delta,
            score: (0.5 + score_noise).clamp(0.0, 1.0) as f32,
            weight: (500.0 + weight_noise).max(0.0),
            ts_ms: (idx << 8).wrapping_add(id_noise),
        }
    }
}

#[inline]
fn build_records() -> Vec<Record> {
    let mut rng = PseudoRng::new(0x1234_5678_9abc_def0);
    (0..BINARY_BATCH as u64)
        .map(|idx| rng.gen_record(idx))
        .collect()
}

#[inline]
fn write_records_wrapper(records: &[Record]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(records.len() * 48);
    let mut writer = BinaryWriter::<_, LittleEndian>::new(Cursor::new(&mut payload));

    for value in records {
        writer.write_u64(value.id).unwrap();
        writer.write_u32(value.user_id).unwrap();
        writer.write_u8(value.flag).unwrap();
        writer.write_i64(value.delta).unwrap();
        writer.write_f32(value.score).unwrap();
        writer.write_f64(value.weight).unwrap();
        writer.write_u64(value.ts_ms).unwrap();
    }

    payload
}

#[inline]
fn write_records_ext(records: &[Record]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(records.len() * 48);
    let mut cursor = Cursor::new(&mut payload);

    for value in records {
        cursor.write_u64_le(value.id).unwrap();
        cursor.write_u32_le(value.user_id).unwrap();
        cursor.write_u8(value.flag).unwrap();
        cursor.write_i64_le(value.delta).unwrap();
        cursor.write_f32_le(value.score).unwrap();
        cursor.write_f64_le(value.weight).unwrap();
        cursor.write_u64_le(value.ts_ms).unwrap();
    }

    payload
}

#[inline]
fn read_records_wrapper(mut bytes: &[u8]) {
    let mut reader = BinaryReader::<_, LittleEndian>::new(Cursor::new(&mut bytes));
    let mut digest = 0u64;

    for _ in 0..BINARY_BATCH {
        let id = reader.read_u64().unwrap();
        let user_id = reader.read_u32().unwrap();
        let flag = reader.read_u8().unwrap();
        let delta = reader.read_i64().unwrap();
        let score = reader.read_f32().unwrap();
        let weight = reader.read_f64().unwrap();
        let ts_ms = reader.read_u64().unwrap();

        digest ^= id;
        digest ^= user_id as u64;
        digest ^= u64::from(flag);
        digest ^= delta as u64;
        digest ^= score.to_bits() as u64;
        digest ^= weight.to_bits();
        digest ^= ts_ms;
    }

    criterion::black_box(digest);
}

#[inline]
fn read_records_ext(mut bytes: &[u8]) {
    let mut cursor = Cursor::new(&mut bytes);
    let mut digest = 0u64;

    for _ in 0..BINARY_BATCH {
        let id = cursor.read_u64_le().unwrap();
        let user_id = cursor.read_u32_le().unwrap();
        let flag = cursor.read_u8().unwrap();
        let delta = cursor.read_i64_le().unwrap();
        let score = cursor.read_f32_le().unwrap();
        let weight = cursor.read_f64_le().unwrap();
        let ts_ms = cursor.read_u64_le().unwrap();

        digest ^= id;
        digest ^= user_id as u64;
        digest ^= u64::from(flag);
        digest ^= delta as u64;
        digest ^= score.to_bits() as u64;
        digest ^= weight.to_bits();
        digest ^= ts_ms;
    }

    criterion::black_box(digest);
}

#[inline]
fn build_varint_values() -> Vec<u64> {
    let mut rng = PseudoRng::new(0xCAFE_BABE_1234_5678);
    let mut values = Vec::with_capacity(VARINT_COUNT);

    for idx in 0..VARINT_COUNT as u64 {
        let mut value = rng.next_normal_u64(8_192.0, 6_000.0);
        value %= 25_000_000;

        if idx % 257 == 0 {
            value = 1u64 << (idx % 56);
        } else if idx % 811 == 0 {
            value = u64::MAX >> (idx % 48);
        }

        values.push(value);
    }

    values
}

#[inline]
fn build_signed_values() -> Vec<i64> {
    let mut rng = PseudoRng::new(0xDEAD_BEEF_0000_1111);
    let mut values = Vec::with_capacity(VARINT_COUNT);

    for idx in 0..VARINT_COUNT as i64 {
        let mut value = rng.next_normal_i64(0.0, 250_000.0);
        if idx % 233 == 0 {
            value = i64::MAX / 3;
        } else if idx % 419 == 0 {
            value = i64::MIN / 3;
        }

        values.push(value);
    }

    values
}

fn bench_prod_binary_pipeline(c: &mut Criterion) {
    let records = build_records();
    let encoded = write_records_wrapper(&records);
    let ext_encoded = write_records_ext(&records);
    assert_eq!(encoded, ext_encoded);
    let bytes_processed = (BINARY_BATCH * BINARY_REPEAT * 41) as u64;

    let mut group = c.benchmark_group("prod_binary_pipeline");
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(12);
    group.throughput(Throughput::Bytes(bytes_processed));

    group.bench_function(BenchmarkId::from_parameter("ext_write_record_batch"), |b| {
        b.iter(|| {
            let mut payload = Vec::with_capacity(records.len() * 48);
            for _ in 0..BINARY_REPEAT {
                payload.clear();
                let mut cursor = Cursor::new(&mut payload);
                for value in &records {
                    cursor.write_u64_le(value.id).unwrap();
                    cursor.write_u32_le(value.user_id).unwrap();
                    cursor.write_u8(value.flag).unwrap();
                    cursor.write_i64_le(value.delta).unwrap();
                    cursor.write_f32_le(value.score).unwrap();
                    cursor.write_f64_le(value.weight).unwrap();
                    cursor.write_u64_le(value.ts_ms).unwrap();
                }
                criterion::black_box(cursor.position());
            }
        })
    });

    group.bench_function(
        BenchmarkId::from_parameter("wrapper_write_record_batch"),
        |b| {
            b.iter(|| {
                let mut payload = Vec::with_capacity(records.len() * 48);
                for _ in 0..BINARY_REPEAT {
                    payload.clear();
                    let mut cursor = Cursor::new(&mut payload);
                    let mut writer = BinaryWriter::<_, LittleEndian>::new(&mut cursor);
                    for value in &records {
                        writer.write_u64(value.id).unwrap();
                        writer.write_u32(value.user_id).unwrap();
                        writer.write_u8(value.flag).unwrap();
                        writer.write_i64(value.delta).unwrap();
                        writer.write_f32(value.score).unwrap();
                        writer.write_f64(value.weight).unwrap();
                        writer.write_u64(value.ts_ms).unwrap();
                    }
                    criterion::black_box(cursor.position());
                }
            })
        },
    );

    group.bench_function(BenchmarkId::from_parameter("ext_read_record_batch"), |b| {
        b.iter(|| {
            for _ in 0..BINARY_REPEAT {
                read_records_ext(&encoded);
            }
        })
    });

    group.bench_function(
        BenchmarkId::from_parameter("wrapper_read_record_batch"),
        |b| {
            b.iter(|| {
                for _ in 0..BINARY_REPEAT {
                    read_records_wrapper(&encoded);
                }
            })
        },
    );

    group.finish();
}

fn bench_prod_varints(c: &mut Criterion) {
    let values = build_varint_values();

    let mut encoded = Vec::with_capacity(values.len() * 10);
    {
        let mut writer = Leb128Writer::new(Cursor::new(&mut encoded));
        for value in &values {
            writer.write_u64(*value).unwrap();
        }
        writer.into_inner().set_position(0);
    }
    let mut ext_encoded = Vec::with_capacity(values.len() * 10);
    {
        let mut cursor = Cursor::new(&mut ext_encoded);
        for value in &values {
            cursor.write_uleb_u64(*value).unwrap();
        }
    }
    assert_eq!(encoded, ext_encoded);
    let bytes_processed = (VARINT_COUNT * VARINT_REPEAT * 2) as u64;

    let mut group = c.benchmark_group("prod_varints");
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(12);
    group.throughput(Throughput::Bytes(bytes_processed));

    group.bench_function(
        BenchmarkId::from_parameter("ext_leb128_write_u64_batch"),
        |b| {
            b.iter(|| {
                for _ in 0..VARINT_REPEAT {
                    let mut payload = Vec::with_capacity(values.len() * 10);
                    let mut cursor = Cursor::new(&mut payload);
                    for value in &values {
                        cursor.write_uleb_u64(*value).unwrap();
                    }
                    criterion::black_box(cursor.into_inner().len());
                }
            })
        },
    );

    group.bench_function(
        BenchmarkId::from_parameter("wrapper_leb128_write_u64_batch"),
        |b| {
            b.iter(|| {
                for _ in 0..VARINT_REPEAT {
                    let mut payload = Vec::with_capacity(values.len() * 10);
                    let mut writer = Leb128Writer::new(Cursor::new(&mut payload));
                    for value in &values {
                        writer.write_u64(*value).unwrap();
                    }
                    criterion::black_box(writer.into_inner().into_inner().len());
                }
            })
        },
    );

    group.bench_function(
        BenchmarkId::from_parameter("ext_leb128_read_u64_batch"),
        |b| {
            b.iter(|| {
                let mut checksum = 0u64;
                for _ in 0..VARINT_REPEAT {
                    let mut cursor = Cursor::new(&encoded);
                    for _ in 0..values.len() {
                        checksum ^= cursor.read_uleb_u64().unwrap();
                    }
                }
                criterion::black_box(checksum);
            })
        },
    );

    group.bench_function(
        BenchmarkId::from_parameter("wrapper_leb128_read_u64_batch"),
        |b| {
            b.iter(|| {
                let mut checksum = 0u64;
                for _ in 0..VARINT_REPEAT {
                    let mut reader = Leb128Reader::<_, NonStrict>::new(Cursor::new(&encoded));
                    for _ in 0..values.len() {
                        checksum ^= reader.read_u64().unwrap();
                    }
                }
                criterion::black_box(checksum);
            })
        },
    );

    group.finish();
}

fn bench_prod_signed_varints(c: &mut Criterion) {
    let values = build_signed_values();

    let mut encoded = Vec::with_capacity(values.len() * 16);
    {
        let mut writer = ZigZagWriter::new(Cursor::new(&mut encoded));
        for value in &values {
            writer.write_i64(*value).unwrap();
        }
        writer.into_inner().set_position(0);
    }
    let mut ext_encoded = Vec::with_capacity(values.len() * 16);
    {
        let mut cursor = Cursor::new(&mut ext_encoded);
        for value in &values {
            cursor.write_zig_zag_i64(*value).unwrap();
        }
    }
    assert_eq!(encoded, ext_encoded);
    let bytes_processed = (VARINT_COUNT * VARINT_REPEAT * 2) as u64;

    let mut group = c.benchmark_group("prod_signed_varints");
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(12);
    group.throughput(Throughput::Bytes(bytes_processed));

    group.bench_function(
        BenchmarkId::from_parameter("ext_zigzag_write_i64_batch"),
        |b| {
            b.iter(|| {
                for _ in 0..VARINT_REPEAT {
                    let mut payload = Vec::with_capacity(values.len() * 8);
                    let mut cursor = Cursor::new(&mut payload);
                    for value in &values {
                        cursor.write_zig_zag_i64(*value).unwrap();
                    }
                    criterion::black_box(cursor.into_inner().len());
                }
            })
        },
    );

    group.bench_function(
        BenchmarkId::from_parameter("wrapper_zigzag_write_i64_batch"),
        |b| {
            b.iter(|| {
                for _ in 0..VARINT_REPEAT {
                    let mut payload = Vec::with_capacity(values.len() * 8);
                    let mut writer = ZigZagWriter::new(Cursor::new(&mut payload));
                    for value in &values {
                        writer.write_i64(*value).unwrap();
                    }
                    criterion::black_box(writer.into_inner().into_inner().len());
                }
            })
        },
    );

    group.bench_function(
        BenchmarkId::from_parameter("ext_zigzag_read_i64_batch"),
        |b| {
            b.iter(|| {
                let mut checksum = 0i64;
                for _ in 0..VARINT_REPEAT {
                    let mut cursor = Cursor::new(&encoded);
                    for _ in 0..values.len() {
                        checksum ^= cursor.read_zig_zag_i64().unwrap();
                    }
                }
                criterion::black_box(checksum);
            })
        },
    );

    group.bench_function(
        BenchmarkId::from_parameter("wrapper_zigzag_read_i64_batch"),
        |b| {
            b.iter(|| {
                let mut checksum = 0i64;
                for _ in 0..VARINT_REPEAT {
                    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new(&encoded));
                    for _ in 0..values.len() {
                        checksum ^= reader.read_i64().unwrap();
                    }
                }
                criterion::black_box(checksum);
            })
        },
    );

    group.finish();
}

criterion_group!(
    benches,
    bench_prod_binary_pipeline,
    bench_prod_varints,
    bench_prod_signed_varints,
);
criterion_main!(benches);
