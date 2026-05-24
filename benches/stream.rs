use std::env;
use std::fs::{
    self,
    File,
};
use std::io::{
    BufReader,
    BufWriter,
    Read,
    Write,
};
use std::path::{
    Path,
    PathBuf,
};
use std::time::{
    Duration,
    SystemTime,
    UNIX_EPOCH,
};

use criterion::{
    BenchmarkId,
    Criterion,
    Throughput,
    criterion_group,
    criterion_main,
};
use qubit_io::{
    BinaryReadExt,
    BinaryReader,
    BinaryWriteExt,
    BinaryWriter,
    BufferedBinaryReader,
    BufferedBinaryWriter,
    BufferedLeb128Reader,
    BufferedLeb128Writer,
    BufferedZigZagReader,
    BufferedZigZagWriter,
    Leb128ReadExt,
    Leb128Reader,
    Leb128WriteExt,
    Leb128Writer,
    LittleEndian,
    NonStrict,
    ZigZagReadExt,
    ZigZagReader,
    ZigZagWriteExt,
    ZigZagWriter,
};

const BINARY_BATCH: usize = 1_048_576;
const BINARY_REPEAT: usize = 32;
const BINARY_RECORD_BYTES: usize = 41;
const VARINT_COUNT: usize = 262_144;
const VARINT_REPEAT: usize = 64;
const STREAM_BENCH_GROUP_ENV: &str = "QUBIT_IO_STREAM_BENCH_GROUP";
const STREAM_BENCH_GROUP_NAMES: [&str; 3] = ["prod_binary_pipeline", "prod_varints", "prod_signed_varints"];

#[derive(Clone, Copy)]
enum StreamBenchGroup {
    BinaryPipeline,
    Varints,
    SignedVarints,
}

fn selected_stream_bench_group() -> StreamBenchGroup {
    let value = env::var(STREAM_BENCH_GROUP_ENV).unwrap_or_else(|_| {
        panic!(
            "{STREAM_BENCH_GROUP_ENV} must be set to one of: {}. \
             Use benches/run_stream_bench_groups.sh to run all groups in \
             isolated cargo bench processes.",
            STREAM_BENCH_GROUP_NAMES.join(", ")
        );
    });

    match value.as_str() {
        "prod_binary_pipeline" => StreamBenchGroup::BinaryPipeline,
        "prod_varints" => StreamBenchGroup::Varints,
        "prod_signed_varints" => StreamBenchGroup::SignedVarints,
        _ => panic!(
            "{STREAM_BENCH_GROUP_ENV}={value:?} is unsupported; expected one of: {}",
            STREAM_BENCH_GROUP_NAMES.join(", ")
        ),
    }
}

struct BenchmarkFiles {
    dir: PathBuf,
}

impl BenchmarkFiles {
    fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("qubit-io-stream-bench-{}-{now}", std::process::id()));
        fs::create_dir_all(&dir).expect("benchmark temp directory should be created");
        Self { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for BenchmarkFiles {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

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
    (0..BINARY_BATCH as u64).map(|idx| rng.gen_record(idx)).collect()
}

#[inline]
fn write_records_wrapper_file(records: &[Record], path: &Path) {
    let file = File::create(path).expect("binary wrapper output file should be created");
    let buffer = BufWriter::new(file);
    let mut writer = BinaryWriter::<_, LittleEndian>::new(buffer);

    for value in records {
        writer.write_u64(value.id).unwrap();
        writer.write_u32(value.user_id).unwrap();
        writer.write_u8(value.flag).unwrap();
        writer.write_i64(value.delta).unwrap();
        writer.write_f32(value.score).unwrap();
        writer.write_f64(value.weight).unwrap();
        writer.write_u64(value.ts_ms).unwrap();
    }

    writer
        .into_inner()
        .flush()
        .expect("binary wrapper output file should flush");
}

#[inline]
fn write_records_ext_file(records: &[Record], path: &Path) {
    let file = File::create(path).expect("binary ext output file should be created");
    let mut writer = BufWriter::new(file);

    for value in records {
        writer.write_u64_le(value.id).unwrap();
        writer.write_u32_le(value.user_id).unwrap();
        writer.write_u8(value.flag).unwrap();
        writer.write_i64_le(value.delta).unwrap();
        writer.write_f32_le(value.score).unwrap();
        writer.write_f64_le(value.weight).unwrap();
        writer.write_u64_le(value.ts_ms).unwrap();
    }

    writer.flush().expect("binary ext output file should flush");
}

#[inline]
fn write_records_std_native_file(records: &[Record], path: &Path) {
    let file = File::create(path).expect("binary std native output file should be created");
    let mut writer = BufWriter::new(file);

    for value in records {
        writer.write_all(&value.id.to_le_bytes()).unwrap();
        writer.write_all(&value.user_id.to_le_bytes()).unwrap();
        writer.write_all(&[value.flag]).unwrap();
        writer.write_all(&value.delta.to_le_bytes()).unwrap();
        writer.write_all(&value.score.to_le_bytes()).unwrap();
        writer.write_all(&value.weight.to_le_bytes()).unwrap();
        writer.write_all(&value.ts_ms.to_le_bytes()).unwrap();
    }

    writer.flush().expect("binary std native output file should flush");
}

#[inline]
fn write_records_buffered_file(records: &[Record], path: &Path) {
    let file = File::create(path).expect("binary buffered output file should be created");
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::new(file);

    for value in records {
        writer.write_u64(value.id).unwrap();
        writer.write_u32(value.user_id).unwrap();
        writer.write_u8(value.flag).unwrap();
        writer.write_i64(value.delta).unwrap();
        writer.write_f32(value.score).unwrap();
        writer.write_f64(value.weight).unwrap();
        writer.write_u64(value.ts_ms).unwrap();
    }

    let mut file = writer.into_inner().expect("binary buffered output file should flush");
    file.flush().expect("binary buffered output file should flush");
}

#[inline]
fn read_records_wrapper_file(path: &Path) {
    let file = File::open(path).expect("binary wrapper input file should open");
    let buffer = BufReader::new(file);
    let mut reader = BinaryReader::<_, LittleEndian>::new(buffer);
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
fn read_records_ext_file(path: &Path) {
    let file = File::open(path).expect("binary ext input file should open");
    let mut reader = BufReader::new(file);
    let mut digest = 0u64;

    for _ in 0..BINARY_BATCH {
        let id = reader.read_u64_le().unwrap();
        let user_id = reader.read_u32_le().unwrap();
        let flag = reader.read_u8().unwrap();
        let delta = reader.read_i64_le().unwrap();
        let score = reader.read_f32_le().unwrap();
        let weight = reader.read_f64_le().unwrap();
        let ts_ms = reader.read_u64_le().unwrap();

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
fn read_records_std_native_file(path: &Path) {
    let file = File::open(path).expect("binary std native input file should open");
    let mut reader = BufReader::new(file);
    let mut digest = 0u64;
    let mut u64_buffer = [0u8; 8];
    let mut u32_buffer = [0u8; 4];
    let mut u8_buffer = [0u8; 1];

    for _ in 0..BINARY_BATCH {
        reader.read_exact(&mut u64_buffer).unwrap();
        let id = u64::from_le_bytes(u64_buffer);
        reader.read_exact(&mut u32_buffer).unwrap();
        let user_id = u32::from_le_bytes(u32_buffer);
        reader.read_exact(&mut u8_buffer).unwrap();
        let flag = u8_buffer[0];
        reader.read_exact(&mut u64_buffer).unwrap();
        let delta = i64::from_le_bytes(u64_buffer);
        reader.read_exact(&mut u32_buffer).unwrap();
        let score = f32::from_le_bytes(u32_buffer);
        reader.read_exact(&mut u64_buffer).unwrap();
        let weight = f64::from_le_bytes(u64_buffer);
        reader.read_exact(&mut u64_buffer).unwrap();
        let ts_ms = u64::from_le_bytes(u64_buffer);

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
fn read_records_buffered_file(path: &Path) {
    let file = File::open(path).expect("binary buffered input file should open");
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::new(file);
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

#[derive(Clone, Copy)]
enum UlebField {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Usize(usize),
    U128(u128),
}

#[derive(Clone, Copy)]
enum ZigZagField {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    Isize(isize),
    I128(i128),
}

#[inline]
fn build_uleb_fields() -> Vec<UlebField> {
    let mut rng = PseudoRng::new(0xCAFE_BABE_1234_5678);
    let mut fields = Vec::with_capacity(VARINT_COUNT);

    for _ in 0..VARINT_COUNT {
        let field = match rng.next_u64() % 6 {
            0 => UlebField::U8(rng.next_normal_u64(128.0, 64.0).min(u64::from(u8::MAX)) as u8),
            1 => UlebField::U16(rng.next_normal_u64(8_192.0, 6_000.0).min(u64::from(u16::MAX)) as u16),
            2 => UlebField::U32(rng.next_normal_u64(1_000_000.0, 600_000.0).min(u64::from(u32::MAX)) as u32),
            3 => UlebField::U64(random_u64_value(&mut rng)),
            4 => UlebField::Usize(random_u64_value(&mut rng) as usize),
            _ => UlebField::U128(random_u128_value(&mut rng)),
        };
        fields.push(field);
    }

    fields
}

#[inline]
fn build_zigzag_fields() -> Vec<ZigZagField> {
    let mut rng = PseudoRng::new(0xDEAD_BEEF_0000_1111);
    let mut fields = Vec::with_capacity(VARINT_COUNT);

    for _ in 0..VARINT_COUNT {
        let field = match rng.next_u64() % 6 {
            0 => ZigZagField::I8(clamp_i64_to_i8(rng.next_normal_i64(0.0, 64.0))),
            1 => ZigZagField::I16(clamp_i64_to_i16(rng.next_normal_i64(0.0, 8_000.0))),
            2 => ZigZagField::I32(clamp_i64_to_i32(rng.next_normal_i64(0.0, 600_000.0))),
            3 => ZigZagField::I64(random_i64_value(&mut rng)),
            4 => ZigZagField::Isize(random_i64_value(&mut rng) as isize),
            _ => ZigZagField::I128(random_i128_value(&mut rng)),
        };
        fields.push(field);
    }

    fields
}

#[inline]
fn random_u64_value(rng: &mut PseudoRng) -> u64 {
    if rng.next_u64().is_multiple_of(1024) {
        rng.next_u64()
    } else {
        rng.next_normal_u64(25_000_000.0, 18_000_000.0)
    }
}

#[inline]
fn random_u128_value(rng: &mut PseudoRng) -> u128 {
    if rng.next_u64().is_multiple_of(1024) {
        (u128::from(rng.next_u64()) << 64) | u128::from(rng.next_u64())
    } else {
        u128::from(rng.next_normal_u64(1_000_000_000_000.0, 800_000_000_000.0))
    }
}

#[inline]
fn random_i64_value(rng: &mut PseudoRng) -> i64 {
    if rng.next_u64().is_multiple_of(1024) {
        rng.next_u64() as i64
    } else {
        rng.next_normal_i64(0.0, 18_000_000.0)
    }
}

#[inline]
fn random_i128_value(rng: &mut PseudoRng) -> i128 {
    if rng.next_u64().is_multiple_of(1024) {
        let raw = (u128::from(rng.next_u64()) << 64) | u128::from(rng.next_u64());
        raw as i128
    } else {
        i128::from(rng.next_normal_i64(0.0, 800_000_000_000.0))
    }
}

#[inline]
fn clamp_i64_to_i8(value: i64) -> i8 {
    value.clamp(i64::from(i8::MIN), i64::from(i8::MAX)) as i8
}

#[inline]
fn clamp_i64_to_i16(value: i64) -> i16 {
    value.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16
}

#[inline]
fn clamp_i64_to_i32(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

#[inline]
fn write_uleb_ext_file(fields: &[UlebField], path: &Path) {
    let file = File::create(path).expect("LEB128 ext output file should be created");
    let mut writer = BufWriter::new(file);

    for field in fields {
        match *field {
            UlebField::U8(value) => writer.write_uleb_u8(value).unwrap(),
            UlebField::U16(value) => writer.write_uleb_u16(value).unwrap(),
            UlebField::U32(value) => writer.write_uleb_u32(value).unwrap(),
            UlebField::U64(value) => writer.write_uleb_u64(value).unwrap(),
            UlebField::Usize(value) => writer.write_uleb_usize(value).unwrap(),
            UlebField::U128(value) => writer.write_uleb_u128(value).unwrap(),
        }
    }

    writer.flush().expect("LEB128 ext output file should flush");
}

#[inline]
fn write_uleb_wrapper_file(fields: &[UlebField], path: &Path) {
    let file = File::create(path).expect("LEB128 wrapper output file should be created");
    let buffer = BufWriter::new(file);
    let mut writer = Leb128Writer::new(buffer);

    for field in fields {
        match *field {
            UlebField::U8(value) => writer.write_u8(value).unwrap(),
            UlebField::U16(value) => writer.write_u16(value).unwrap(),
            UlebField::U32(value) => writer.write_u32(value).unwrap(),
            UlebField::U64(value) => writer.write_u64(value).unwrap(),
            UlebField::Usize(value) => writer.write_usize(value).unwrap(),
            UlebField::U128(value) => writer.write_u128(value).unwrap(),
        }
    }

    writer
        .into_inner()
        .flush()
        .expect("LEB128 wrapper output file should flush");
}

#[inline]
fn write_uleb_buffered_file(fields: &[UlebField], path: &Path) {
    let file = File::create(path).expect("LEB128 buffered output file should be created");
    let mut writer = BufferedLeb128Writer::new(file);

    for field in fields {
        match *field {
            UlebField::U8(value) => writer.write_u8(value).unwrap(),
            UlebField::U16(value) => writer.write_u16(value).unwrap(),
            UlebField::U32(value) => writer.write_u32(value).unwrap(),
            UlebField::U64(value) => writer.write_u64(value).unwrap(),
            UlebField::Usize(value) => writer.write_usize(value).unwrap(),
            UlebField::U128(value) => writer.write_u128(value).unwrap(),
        }
    }

    let mut file = writer.into_inner().expect("LEB128 buffered output file should flush");
    file.flush().expect("LEB128 buffered output file should flush");
}

#[inline]
fn read_uleb_ext_file(path: &Path, fields: &[UlebField]) {
    let file = File::open(path).expect("LEB128 ext input file should open");
    let mut reader = BufReader::new(file);
    let mut checksum = 0u128;

    for field in fields {
        match *field {
            UlebField::U8(_) => checksum ^= u128::from(reader.read_uleb_u8().unwrap()),
            UlebField::U16(_) => checksum ^= u128::from(reader.read_uleb_u16().unwrap()),
            UlebField::U32(_) => checksum ^= u128::from(reader.read_uleb_u32().unwrap()),
            UlebField::U64(_) => checksum ^= u128::from(reader.read_uleb_u64().unwrap()),
            UlebField::Usize(_) => checksum ^= reader.read_uleb_usize().unwrap() as u128,
            UlebField::U128(_) => checksum ^= reader.read_uleb_u128().unwrap(),
        }
    }

    criterion::black_box(checksum);
}

#[inline]
fn read_uleb_wrapper_file(path: &Path, fields: &[UlebField]) {
    let file = File::open(path).expect("LEB128 wrapper input file should open");
    let buffer = BufReader::new(file);
    let mut reader = Leb128Reader::<_, NonStrict>::new(buffer);
    let mut checksum = 0u128;

    for field in fields {
        match *field {
            UlebField::U8(_) => checksum ^= u128::from(reader.read_u8().unwrap()),
            UlebField::U16(_) => checksum ^= u128::from(reader.read_u16().unwrap()),
            UlebField::U32(_) => checksum ^= u128::from(reader.read_u32().unwrap()),
            UlebField::U64(_) => checksum ^= u128::from(reader.read_u64().unwrap()),
            UlebField::Usize(_) => checksum ^= reader.read_usize().unwrap() as u128,
            UlebField::U128(_) => checksum ^= reader.read_u128().unwrap(),
        }
    }

    criterion::black_box(checksum);
}

#[inline]
fn read_uleb_buffered_file(path: &Path, fields: &[UlebField]) {
    let file = File::open(path).expect("LEB128 buffered input file should open");
    let mut reader = BufferedLeb128Reader::<_, NonStrict>::new(file);
    let mut checksum = 0u128;

    for field in fields {
        match *field {
            UlebField::U8(_) => checksum ^= u128::from(reader.read_u8().unwrap()),
            UlebField::U16(_) => checksum ^= u128::from(reader.read_u16().unwrap()),
            UlebField::U32(_) => checksum ^= u128::from(reader.read_u32().unwrap()),
            UlebField::U64(_) => checksum ^= u128::from(reader.read_u64().unwrap()),
            UlebField::Usize(_) => checksum ^= reader.read_usize().unwrap() as u128,
            UlebField::U128(_) => checksum ^= reader.read_u128().unwrap(),
        }
    }

    criterion::black_box(checksum);
}

#[inline]
fn write_zigzag_ext_file(fields: &[ZigZagField], path: &Path) {
    let file = File::create(path).expect("ZigZag ext output file should be created");
    let mut writer = BufWriter::new(file);

    for field in fields {
        match *field {
            ZigZagField::I8(value) => writer.write_zig_zag_i8(value).unwrap(),
            ZigZagField::I16(value) => writer.write_zig_zag_i16(value).unwrap(),
            ZigZagField::I32(value) => writer.write_zig_zag_i32(value).unwrap(),
            ZigZagField::I64(value) => writer.write_zig_zag_i64(value).unwrap(),
            ZigZagField::Isize(value) => writer.write_zig_zag_isize(value).unwrap(),
            ZigZagField::I128(value) => writer.write_zig_zag_i128(value).unwrap(),
        }
    }

    writer.flush().expect("ZigZag ext output file should flush");
}

#[inline]
fn write_zigzag_wrapper_file(fields: &[ZigZagField], path: &Path) {
    let file = File::create(path).expect("ZigZag wrapper output file should be created");
    let buffer = BufWriter::new(file);
    let mut writer = ZigZagWriter::new(buffer);

    for field in fields {
        match *field {
            ZigZagField::I8(value) => writer.write_i8(value).unwrap(),
            ZigZagField::I16(value) => writer.write_i16(value).unwrap(),
            ZigZagField::I32(value) => writer.write_i32(value).unwrap(),
            ZigZagField::I64(value) => writer.write_i64(value).unwrap(),
            ZigZagField::Isize(value) => writer.write_isize(value).unwrap(),
            ZigZagField::I128(value) => writer.write_i128(value).unwrap(),
        }
    }

    writer
        .into_inner()
        .flush()
        .expect("ZigZag wrapper output file should flush");
}

#[inline]
fn write_zigzag_buffered_file(fields: &[ZigZagField], path: &Path) {
    let file = File::create(path).expect("ZigZag buffered output file should be created");
    let mut writer = BufferedZigZagWriter::new(file);

    for field in fields {
        match *field {
            ZigZagField::I8(value) => writer.write_i8(value).unwrap(),
            ZigZagField::I16(value) => writer.write_i16(value).unwrap(),
            ZigZagField::I32(value) => writer.write_i32(value).unwrap(),
            ZigZagField::I64(value) => writer.write_i64(value).unwrap(),
            ZigZagField::Isize(value) => writer.write_isize(value).unwrap(),
            ZigZagField::I128(value) => writer.write_i128(value).unwrap(),
        }
    }

    let mut file = writer.into_inner().expect("ZigZag buffered output file should flush");
    file.flush().expect("ZigZag buffered output file should flush");
}

#[inline]
fn read_zigzag_ext_file(path: &Path, fields: &[ZigZagField]) {
    let file = File::open(path).expect("ZigZag ext input file should open");
    let mut reader = BufReader::new(file);
    let mut checksum = 0i128;

    for field in fields {
        match *field {
            ZigZagField::I8(_) => checksum ^= i128::from(reader.read_zig_zag_i8().unwrap()),
            ZigZagField::I16(_) => checksum ^= i128::from(reader.read_zig_zag_i16().unwrap()),
            ZigZagField::I32(_) => checksum ^= i128::from(reader.read_zig_zag_i32().unwrap()),
            ZigZagField::I64(_) => checksum ^= i128::from(reader.read_zig_zag_i64().unwrap()),
            ZigZagField::Isize(_) => checksum ^= reader.read_zig_zag_isize().unwrap() as i128,
            ZigZagField::I128(_) => checksum ^= reader.read_zig_zag_i128().unwrap(),
        }
    }

    criterion::black_box(checksum);
}

#[inline]
fn read_zigzag_wrapper_file(path: &Path, fields: &[ZigZagField]) {
    let file = File::open(path).expect("ZigZag wrapper input file should open");
    let buffer = BufReader::new(file);
    let mut reader = ZigZagReader::<_, NonStrict>::new(buffer);
    let mut checksum = 0i128;

    for field in fields {
        match *field {
            ZigZagField::I8(_) => checksum ^= i128::from(reader.read_i8().unwrap()),
            ZigZagField::I16(_) => checksum ^= i128::from(reader.read_i16().unwrap()),
            ZigZagField::I32(_) => checksum ^= i128::from(reader.read_i32().unwrap()),
            ZigZagField::I64(_) => checksum ^= i128::from(reader.read_i64().unwrap()),
            ZigZagField::Isize(_) => checksum ^= reader.read_isize().unwrap() as i128,
            ZigZagField::I128(_) => checksum ^= reader.read_i128().unwrap(),
        }
    }

    criterion::black_box(checksum);
}

#[inline]
fn read_zigzag_buffered_file(path: &Path, fields: &[ZigZagField]) {
    let file = File::open(path).expect("ZigZag buffered input file should open");
    let mut reader = BufferedZigZagReader::<_, NonStrict>::new(file);
    let mut checksum = 0i128;

    for field in fields {
        match *field {
            ZigZagField::I8(_) => checksum ^= i128::from(reader.read_i8().unwrap()),
            ZigZagField::I16(_) => checksum ^= i128::from(reader.read_i16().unwrap()),
            ZigZagField::I32(_) => checksum ^= i128::from(reader.read_i32().unwrap()),
            ZigZagField::I64(_) => checksum ^= i128::from(reader.read_i64().unwrap()),
            ZigZagField::Isize(_) => checksum ^= reader.read_isize().unwrap() as i128,
            ZigZagField::I128(_) => checksum ^= reader.read_i128().unwrap(),
        }
    }

    criterion::black_box(checksum);
}

fn bench_prod_binary_pipeline(c: &mut Criterion) {
    let records = build_records();
    let files = BenchmarkFiles::new();
    let wrapper_source_path = files.path("binary-wrapper-source.bin");
    let ext_source_path = files.path("binary-ext-source.bin");
    let std_native_source_path = files.path("binary-std-native-source.bin");
    let buffered_source_path = files.path("binary-buffered-source.bin");
    let ext_write_path = files.path("binary-ext-write.bin");
    let std_native_write_path = files.path("binary-std-native-write.bin");
    let wrapper_write_path = files.path("binary-wrapper-write.bin");
    let buffered_write_path = files.path("binary-buffered-write.bin");

    write_records_wrapper_file(&records, &wrapper_source_path);
    write_records_ext_file(&records, &ext_source_path);
    write_records_std_native_file(&records, &std_native_source_path);
    write_records_buffered_file(&records, &buffered_source_path);
    assert_eq!(
        fs::read(&wrapper_source_path).expect("binary wrapper source should be readable"),
        fs::read(&ext_source_path).expect("binary ext source should be readable")
    );
    assert_eq!(
        fs::read(&wrapper_source_path).expect("binary wrapper source should be readable"),
        fs::read(&std_native_source_path).expect("binary std native source should be readable")
    );
    assert_eq!(
        fs::read(&wrapper_source_path).expect("binary wrapper source should be readable"),
        fs::read(&buffered_source_path).expect("binary buffered source should be readable")
    );
    let bytes_processed = (BINARY_BATCH * BINARY_REPEAT * BINARY_RECORD_BYTES) as u64;

    let mut group = c.benchmark_group("prod_binary_pipeline");
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(12);
    group.throughput(Throughput::Bytes(bytes_processed));

    group.bench_function(BenchmarkId::from_parameter("ext_write_record_batch"), |b| {
        b.iter(|| {
            for _ in 0..BINARY_REPEAT {
                write_records_ext_file(&records, &ext_write_path);
                criterion::black_box(BINARY_BATCH * BINARY_RECORD_BYTES);
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("std_native_write_record_batch"), |b| {
        b.iter(|| {
            for _ in 0..BINARY_REPEAT {
                write_records_std_native_file(&records, &std_native_write_path);
                criterion::black_box(BINARY_BATCH * BINARY_RECORD_BYTES);
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("wrapper_write_record_batch"), |b| {
        b.iter(|| {
            for _ in 0..BINARY_REPEAT {
                write_records_wrapper_file(&records, &wrapper_write_path);
                criterion::black_box(BINARY_BATCH * BINARY_RECORD_BYTES);
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("buffered_write_record_batch"), |b| {
        b.iter(|| {
            for _ in 0..BINARY_REPEAT {
                write_records_buffered_file(&records, &buffered_write_path);
                criterion::black_box(BINARY_BATCH * BINARY_RECORD_BYTES);
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("ext_read_record_batch"), |b| {
        b.iter(|| {
            for _ in 0..BINARY_REPEAT {
                read_records_ext_file(&wrapper_source_path);
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("std_native_read_record_batch"), |b| {
        b.iter(|| {
            for _ in 0..BINARY_REPEAT {
                read_records_std_native_file(&wrapper_source_path);
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("wrapper_read_record_batch"), |b| {
        b.iter(|| {
            for _ in 0..BINARY_REPEAT {
                read_records_wrapper_file(&wrapper_source_path);
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("buffered_read_record_batch"), |b| {
        b.iter(|| {
            for _ in 0..BINARY_REPEAT {
                read_records_buffered_file(&wrapper_source_path);
            }
        })
    });

    group.finish();
}

fn bench_prod_varints(c: &mut Criterion) {
    let fields = build_uleb_fields();
    let files = BenchmarkFiles::new();
    let ext_source_path = files.path("uleb-ext-source.bin");
    let wrapper_source_path = files.path("uleb-wrapper-source.bin");
    let buffered_source_path = files.path("uleb-buffered-source.bin");
    let ext_write_path = files.path("uleb-ext-write.bin");
    let wrapper_write_path = files.path("uleb-wrapper-write.bin");
    let buffered_write_path = files.path("uleb-buffered-write.bin");

    write_uleb_ext_file(&fields, &ext_source_path);
    write_uleb_wrapper_file(&fields, &wrapper_source_path);
    write_uleb_buffered_file(&fields, &buffered_source_path);
    let encoded = fs::read(&ext_source_path).expect("LEB128 ext source should be readable");
    assert_eq!(
        encoded,
        fs::read(&wrapper_source_path).expect("LEB128 wrapper source should be readable")
    );
    assert_eq!(
        encoded,
        fs::read(&buffered_source_path).expect("LEB128 buffered source should be readable")
    );
    let bytes_processed = (encoded.len() * VARINT_REPEAT) as u64;

    let mut group = c.benchmark_group("prod_varints");
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(12);
    group.throughput(Throughput::Bytes(bytes_processed));

    group.bench_function(BenchmarkId::from_parameter("ext_leb128_write_mixed_batch"), |b| {
        b.iter(|| {
            for _ in 0..VARINT_REPEAT {
                write_uleb_ext_file(&fields, &ext_write_path);
                criterion::black_box(encoded.len());
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("wrapper_leb128_write_mixed_batch"), |b| {
        b.iter(|| {
            for _ in 0..VARINT_REPEAT {
                write_uleb_wrapper_file(&fields, &wrapper_write_path);
                criterion::black_box(encoded.len());
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("buffered_leb128_write_mixed_batch"), |b| {
        b.iter(|| {
            for _ in 0..VARINT_REPEAT {
                write_uleb_buffered_file(&fields, &buffered_write_path);
                criterion::black_box(encoded.len());
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("ext_leb128_read_mixed_batch"), |b| {
        b.iter(|| {
            for _ in 0..VARINT_REPEAT {
                read_uleb_ext_file(&ext_source_path, &fields);
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("wrapper_leb128_read_mixed_batch"), |b| {
        b.iter(|| {
            for _ in 0..VARINT_REPEAT {
                read_uleb_wrapper_file(&ext_source_path, &fields);
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("buffered_leb128_read_mixed_batch"), |b| {
        b.iter(|| {
            for _ in 0..VARINT_REPEAT {
                read_uleb_buffered_file(&ext_source_path, &fields);
            }
        })
    });

    group.finish();
}

fn bench_prod_signed_varints(c: &mut Criterion) {
    let fields = build_zigzag_fields();
    let files = BenchmarkFiles::new();
    let ext_source_path = files.path("zigzag-ext-source.bin");
    let wrapper_source_path = files.path("zigzag-wrapper-source.bin");
    let buffered_source_path = files.path("zigzag-buffered-source.bin");
    let ext_write_path = files.path("zigzag-ext-write.bin");
    let wrapper_write_path = files.path("zigzag-wrapper-write.bin");
    let buffered_write_path = files.path("zigzag-buffered-write.bin");

    write_zigzag_ext_file(&fields, &ext_source_path);
    write_zigzag_wrapper_file(&fields, &wrapper_source_path);
    write_zigzag_buffered_file(&fields, &buffered_source_path);
    let encoded = fs::read(&ext_source_path).expect("ZigZag ext source should be readable");
    assert_eq!(
        encoded,
        fs::read(&wrapper_source_path).expect("ZigZag wrapper source should be readable")
    );
    assert_eq!(
        encoded,
        fs::read(&buffered_source_path).expect("ZigZag buffered source should be readable")
    );
    let bytes_processed = (encoded.len() * VARINT_REPEAT) as u64;

    let mut group = c.benchmark_group("prod_signed_varints");
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(12);
    group.throughput(Throughput::Bytes(bytes_processed));

    group.bench_function(BenchmarkId::from_parameter("ext_zigzag_write_mixed_batch"), |b| {
        b.iter(|| {
            for _ in 0..VARINT_REPEAT {
                write_zigzag_ext_file(&fields, &ext_write_path);
                criterion::black_box(encoded.len());
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("wrapper_zigzag_write_mixed_batch"), |b| {
        b.iter(|| {
            for _ in 0..VARINT_REPEAT {
                write_zigzag_wrapper_file(&fields, &wrapper_write_path);
                criterion::black_box(encoded.len());
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("buffered_zigzag_write_mixed_batch"), |b| {
        b.iter(|| {
            for _ in 0..VARINT_REPEAT {
                write_zigzag_buffered_file(&fields, &buffered_write_path);
                criterion::black_box(encoded.len());
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("ext_zigzag_read_mixed_batch"), |b| {
        b.iter(|| {
            for _ in 0..VARINT_REPEAT {
                read_zigzag_ext_file(&ext_source_path, &fields);
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("wrapper_zigzag_read_mixed_batch"), |b| {
        b.iter(|| {
            for _ in 0..VARINT_REPEAT {
                read_zigzag_wrapper_file(&ext_source_path, &fields);
            }
        })
    });

    group.bench_function(BenchmarkId::from_parameter("buffered_zigzag_read_mixed_batch"), |b| {
        b.iter(|| {
            for _ in 0..VARINT_REPEAT {
                read_zigzag_buffered_file(&ext_source_path, &fields);
            }
        })
    });

    group.finish();
}

fn bench_selected_stream_group(c: &mut Criterion) {
    match selected_stream_bench_group() {
        StreamBenchGroup::BinaryPipeline => bench_prod_binary_pipeline(c),
        StreamBenchGroup::Varints => bench_prod_varints(c),
        StreamBenchGroup::SignedVarints => bench_prod_signed_varints(c),
    }
}

criterion_group!(benches, bench_selected_stream_group);
criterion_main!(benches);
