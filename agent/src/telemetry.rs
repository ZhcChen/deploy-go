use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use deploy_go_agent_protocol::{
    CpuTelemetry, DiskIoTelemetry, DiskTelemetry, GpuTelemetry, MemoryTelemetry, NetworkTelemetry,
    NodeTelemetrySnapshot, TelemetryMetricReason, TelemetryMetricStatus,
};

const SECTOR_BYTES: u64 = 512;
const MAX_SAMPLE_GAP: Duration = Duration::from_secs(600);
const NVIDIA_TIMEOUT: Duration = Duration::from_secs(5);
const NVIDIA_OUTPUT_LIMIT: u64 = 8 * 1024;
const PROC_OUTPUT_LIMIT: u64 = 1024 * 1024;
const SYSFS_OUTPUT_LIMIT: u64 = 16 * 1024;
const MAX_BLOCK_DEVICES: usize = 256;

pub trait TelemetryCollector: Send {
    fn collect(&mut self) -> NodeTelemetrySnapshot;
}

pub trait TelemetryFactory: Send + Sync {
    fn create(&self) -> Box<dyn TelemetryCollector>;
}

pub trait GpuReader: Send {
    fn collect(&mut self) -> GpuCollection;
}

#[derive(Clone, Debug, PartialEq)]
pub enum GpuCollection {
    Unsupported(TelemetryMetricReason),
    Error(TelemetryMetricReason),
    Available(Vec<GpuTelemetry>),
}

#[derive(Clone)]
pub struct LinuxTelemetryFactory {
    work_root: PathBuf,
}

impl LinuxTelemetryFactory {
    pub fn new(work_root: PathBuf) -> Self {
        Self { work_root }
    }
}

impl TelemetryFactory for LinuxTelemetryFactory {
    fn create(&self) -> Box<dyn TelemetryCollector> {
        #[cfg(target_os = "linux")]
        {
            Box::new(LinuxTelemetryCollector::new(self.work_root.clone()))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = &self.work_root;
            Box::new(UnsupportedTelemetryCollector)
        }
    }
}

struct UnsupportedTelemetryCollector;

impl TelemetryCollector for UnsupportedTelemetryCollector {
    fn collect(&mut self) -> NodeTelemetrySnapshot {
        NodeTelemetrySnapshot {
            cpu: CpuTelemetry {
                status: TelemetryMetricStatus::Unsupported,
                usage_percent: None,
            },
            memory: MemoryTelemetry {
                status: TelemetryMetricStatus::Unsupported,
                total_bytes: None,
                used_bytes: None,
                usage_percent: None,
            },
            work_root_disk: DiskTelemetry {
                status: TelemetryMetricStatus::Unsupported,
                total_bytes: None,
                used_bytes: None,
                usage_percent: None,
            },
            disk_io: DiskIoTelemetry {
                status: TelemetryMetricStatus::Unsupported,
                read_bytes_per_second: None,
                write_bytes_per_second: None,
                busy_percent: None,
            },
            network: NetworkTelemetry {
                status: TelemetryMetricStatus::Unsupported,
                receive_bytes_per_second: None,
                transmit_bytes_per_second: None,
            },
            gpu_status: TelemetryMetricStatus::Unsupported,
            gpu_reason: Some(TelemetryMetricReason::UnsupportedPlatform),
            gpus: Vec::new(),
        }
    }
}

pub struct LinuxTelemetryCollector {
    proc_root: PathBuf,
    sys_root: PathBuf,
    work_root: PathBuf,
    gpu_reader: Box<dyn GpuReader>,
    baseline: Option<CounterBaseline>,
}

impl LinuxTelemetryCollector {
    pub fn new(work_root: PathBuf) -> Self {
        Self::with_paths(
            work_root,
            PathBuf::from("/proc"),
            PathBuf::from("/sys"),
            Box::new(NvidiaSmiReader::new(
                PathBuf::from("/sys"),
                PathBuf::from("/usr/bin/nvidia-smi"),
            )),
        )
    }

    pub fn with_paths(
        work_root: PathBuf,
        proc_root: PathBuf,
        sys_root: PathBuf,
        gpu_reader: Box<dyn GpuReader>,
    ) -> Self {
        Self {
            proc_root,
            sys_root,
            work_root,
            gpu_reader,
            baseline: None,
        }
    }

    pub fn collect_at(&mut self, now: Instant) -> NodeTelemetrySnapshot {
        let cpu = read_cpu(&self.proc_root.join("stat"));
        let disk_io = read_disk_io(&self.sys_root.join("block"));
        let network = read_network(&self.proc_root.join("net/dev"));
        let current = CounterBaseline {
            captured_at: now,
            cpu: cpu.ok(),
            disk_io: disk_io.ok(),
            network: network.ok(),
        };
        let previous = self.baseline.replace(current);
        let elapsed = previous
            .map(|previous| now.saturating_duration_since(previous.captured_at))
            .filter(|elapsed| !elapsed.is_zero() && *elapsed <= MAX_SAMPLE_GAP);
        let gpu = self.gpu_reader.collect();
        let (gpu_status, gpu_reason, gpus) = match gpu {
            GpuCollection::Unsupported(reason) => {
                (TelemetryMetricStatus::Unsupported, Some(reason), Vec::new())
            }
            GpuCollection::Error(reason) => (
                TelemetryMetricStatus::CollectionError,
                Some(reason),
                Vec::new(),
            ),
            GpuCollection::Available(gpus) => (TelemetryMetricStatus::Available, None, gpus),
        };

        NodeTelemetrySnapshot {
            cpu: cpu_metric(current.cpu, previous.and_then(|item| item.cpu), elapsed),
            memory: memory_metric(read_memory(&self.proc_root.join("meminfo"))),
            work_root_disk: filesystem_metric(&self.work_root),
            disk_io: disk_io_metric(
                current.disk_io,
                previous.and_then(|item| item.disk_io),
                elapsed,
            ),
            network: network_metric(
                current.network,
                previous.and_then(|item| item.network),
                elapsed,
            ),
            gpu_status,
            gpu_reason,
            gpus,
        }
    }
}

impl TelemetryCollector for LinuxTelemetryCollector {
    fn collect(&mut self) -> NodeTelemetrySnapshot {
        self.collect_at(Instant::now())
    }
}

#[derive(Clone, Copy)]
struct CounterBaseline {
    captured_at: Instant,
    cpu: Option<CpuCounters>,
    disk_io: Option<DiskIoCounters>,
    network: Option<NetworkCounters>,
}

#[derive(Clone, Copy)]
struct CpuCounters {
    total: u64,
    idle: u64,
}

#[derive(Clone, Copy)]
struct DiskIoCounters {
    read_bytes: u64,
    write_bytes: u64,
    busy_milliseconds: u64,
}

#[derive(Clone, Copy)]
struct NetworkCounters {
    receive_bytes: u64,
    transmit_bytes: u64,
}

fn read_cpu(path: &Path) -> Result<CpuCounters, ()> {
    let contents = read_limited(path, PROC_OUTPUT_LIMIT)?;
    let mut fields = contents.lines().next().ok_or(())?.split_whitespace();
    if fields.next() != Some("cpu") {
        return Err(());
    }
    let values = fields
        .map(|value| value.parse::<u64>().map_err(|_| ()))
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() < 5 {
        return Err(());
    }
    let total = values
        .iter()
        .try_fold(0_u64, |sum, value| sum.checked_add(*value))
        .ok_or(())?;
    let idle = values[3].checked_add(values[4]).ok_or(())?;
    Ok(CpuCounters { total, idle })
}

fn read_memory(path: &Path) -> Result<(u64, u64), ()> {
    let contents = read_limited(path, PROC_OUTPUT_LIMIT)?;
    let mut total = None;
    let mut available = None;
    for line in contents.lines() {
        let mut fields = line.split_whitespace();
        let key = fields.next().ok_or(())?;
        if !matches!(key, "MemTotal:" | "MemAvailable:") {
            continue;
        }
        let value = fields.next().ok_or(())?.parse::<u64>().map_err(|_| ())?;
        if fields.next() != Some("kB") || fields.next().is_some() {
            return Err(());
        }
        let bytes = value.checked_mul(1024).ok_or(())?;
        match key {
            "MemTotal:" => total = Some(bytes),
            "MemAvailable:" => available = Some(bytes),
            _ => {}
        }
    }
    let (total, available) = (total.ok_or(())?, available.ok_or(())?);
    if total == 0 || available > total {
        return Err(());
    }
    Ok((total, total - available))
}

fn read_network(path: &Path) -> Result<NetworkCounters, ()> {
    let contents = read_limited(path, PROC_OUTPUT_LIMIT)?;
    let mut receive_bytes = 0_u64;
    let mut transmit_bytes = 0_u64;
    let mut count = 0_u32;
    for line in contents.lines().skip(2) {
        let (name, counters) = line.split_once(':').ok_or(())?;
        if name.trim() == "lo" {
            continue;
        }
        let counters = counters
            .split_whitespace()
            .map(|value| value.parse::<u64>().map_err(|_| ()))
            .collect::<Result<Vec<_>, _>>()?;
        if counters.len() < 16 {
            return Err(());
        }
        receive_bytes = receive_bytes.checked_add(counters[0]).ok_or(())?;
        transmit_bytes = transmit_bytes.checked_add(counters[8]).ok_or(())?;
        count += 1;
    }
    if count == 0 {
        return Err(());
    }
    Ok(NetworkCounters {
        receive_bytes,
        transmit_bytes,
    })
}

fn read_disk_io(block_root: &Path) -> Result<DiskIoCounters, ()> {
    let mut read_bytes = 0_u64;
    let mut write_bytes = 0_u64;
    let mut busy_milliseconds = 0_u64;
    let mut count = 0_u32;
    for (index, entry) in fs::read_dir(block_root).map_err(|_| ())?.enumerate() {
        if index >= MAX_BLOCK_DEVICES {
            return Err(());
        }
        let entry = entry.map_err(|_| ())?;
        let name = entry.file_name();
        let name = name.to_str().ok_or(())?;
        if virtual_block_device(name) || !entry.path().join("device").exists() {
            continue;
        }
        let contents = read_limited(&entry.path().join("stat"), SYSFS_OUTPUT_LIMIT)?;
        let fields = contents
            .split_whitespace()
            .map(|value| value.parse::<u64>().map_err(|_| ()))
            .collect::<Result<Vec<_>, _>>()?;
        if fields.len() < 11 {
            return Err(());
        }
        read_bytes = read_bytes
            .checked_add(fields[2].checked_mul(SECTOR_BYTES).ok_or(())?)
            .ok_or(())?;
        write_bytes = write_bytes
            .checked_add(fields[6].checked_mul(SECTOR_BYTES).ok_or(())?)
            .ok_or(())?;
        busy_milliseconds = busy_milliseconds.checked_add(fields[9]).ok_or(())?;
        count += 1;
    }
    if count == 0 {
        return Err(());
    }
    Ok(DiskIoCounters {
        read_bytes,
        write_bytes,
        busy_milliseconds,
    })
}

fn virtual_block_device(name: &str) -> bool {
    ["loop", "ram", "zram", "dm-", "md", "sr", "fd"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

fn cpu_metric(
    current: Option<CpuCounters>,
    previous: Option<CpuCounters>,
    elapsed: Option<Duration>,
) -> CpuTelemetry {
    let value = match (current, previous, elapsed) {
        (Some(current), Some(previous), Some(_))
            if current.total > previous.total && current.idle >= previous.idle =>
        {
            let total = current.total - previous.total;
            let idle = current.idle - previous.idle;
            (idle <= total).then_some((total - idle) as f64 * 100.0 / total as f64)
        }
        _ => None,
    };
    CpuTelemetry {
        status: rate_status(current.is_some(), value.is_some()),
        usage_percent: value,
    }
}

fn memory_metric(value: Result<(u64, u64), ()>) -> MemoryTelemetry {
    match value {
        Ok((total, used)) => MemoryTelemetry {
            status: TelemetryMetricStatus::Available,
            total_bytes: Some(total),
            used_bytes: Some(used),
            usage_percent: Some(used as f64 * 100.0 / total as f64),
        },
        Err(()) => MemoryTelemetry {
            status: TelemetryMetricStatus::CollectionError,
            total_bytes: None,
            used_bytes: None,
            usage_percent: None,
        },
    }
}

fn filesystem_metric(path: &Path) -> DiskTelemetry {
    let Ok(filesystem) = nix::sys::statvfs::statvfs(path) else {
        return DiskTelemetry {
            status: TelemetryMetricStatus::CollectionError,
            total_bytes: None,
            used_bytes: None,
            usage_percent: None,
        };
    };
    let block_size = filesystem.fragment_size();
    let total = u64::from(filesystem.blocks()).saturating_mul(block_size);
    let available = u64::from(filesystem.blocks_available()).saturating_mul(block_size);
    if total == 0 || available > total {
        return DiskTelemetry {
            status: TelemetryMetricStatus::CollectionError,
            total_bytes: None,
            used_bytes: None,
            usage_percent: None,
        };
    }
    let used = total - available;
    DiskTelemetry {
        status: TelemetryMetricStatus::Available,
        total_bytes: Some(total),
        used_bytes: Some(used),
        usage_percent: Some(used as f64 * 100.0 / total as f64),
    }
}

fn disk_io_metric(
    current: Option<DiskIoCounters>,
    previous: Option<DiskIoCounters>,
    elapsed: Option<Duration>,
) -> DiskIoTelemetry {
    let value = match (current, previous, elapsed) {
        (Some(current), Some(previous), Some(elapsed)) => (|| {
            Some((
                current.read_bytes.checked_sub(previous.read_bytes)? as f64 / elapsed.as_secs_f64(),
                current.write_bytes.checked_sub(previous.write_bytes)? as f64
                    / elapsed.as_secs_f64(),
                (current
                    .busy_milliseconds
                    .checked_sub(previous.busy_milliseconds)? as f64
                    / (elapsed.as_secs_f64() * 1000.0)
                    * 100.0)
                    .min(100.0),
            ))
        })(),
        _ => None,
    };
    DiskIoTelemetry {
        status: rate_status(current.is_some(), value.is_some()),
        read_bytes_per_second: value.map(|value| value.0),
        write_bytes_per_second: value.map(|value| value.1),
        busy_percent: value.map(|value| value.2),
    }
}

fn network_metric(
    current: Option<NetworkCounters>,
    previous: Option<NetworkCounters>,
    elapsed: Option<Duration>,
) -> NetworkTelemetry {
    let value = match (current, previous, elapsed) {
        (Some(current), Some(previous), Some(elapsed)) => (|| {
            Some((
                current.receive_bytes.checked_sub(previous.receive_bytes)? as f64
                    / elapsed.as_secs_f64(),
                current
                    .transmit_bytes
                    .checked_sub(previous.transmit_bytes)? as f64
                    / elapsed.as_secs_f64(),
            ))
        })(),
        _ => None,
    };
    NetworkTelemetry {
        status: rate_status(current.is_some(), value.is_some()),
        receive_bytes_per_second: value.map(|value| value.0),
        transmit_bytes_per_second: value.map(|value| value.1),
    }
}

fn rate_status(has_current: bool, has_value: bool) -> TelemetryMetricStatus {
    if !has_current {
        TelemetryMetricStatus::CollectionError
    } else if has_value {
        TelemetryMetricStatus::Available
    } else {
        TelemetryMetricStatus::WarmingUp
    }
}

pub struct NvidiaSmiReader {
    sys_root: PathBuf,
    executable: PathBuf,
}

impl NvidiaSmiReader {
    pub fn new(sys_root: PathBuf, executable: PathBuf) -> Self {
        Self {
            sys_root,
            executable,
        }
    }
}

impl GpuReader for NvidiaSmiReader {
    fn collect(&mut self) -> GpuCollection {
        match detect_nvidia_hardware(&self.sys_root) {
            Ok(true) => {}
            Ok(false) => {
                return GpuCollection::Unsupported(TelemetryMetricReason::HardwareNotPresent);
            }
            Err(reason) => return GpuCollection::Error(reason),
        }
        read_nvidia_smi(&self.executable)
    }
}

fn detect_nvidia_hardware(sys_root: &Path) -> Result<bool, TelemetryMetricReason> {
    let entries = fs::read_dir(sys_root.join("bus/pci/devices")).map_err(io_reason)?;
    for entry in entries {
        let entry = entry.map_err(io_reason)?;
        let vendor = read_limited(&entry.path().join("vendor"), SYSFS_OUTPUT_LIMIT)
            .map_err(|_| TelemetryMetricReason::SourceUnavailable)?;
        if vendor.trim().eq_ignore_ascii_case("0x10de") {
            return Ok(true);
        }
    }
    Ok(false)
}

fn read_nvidia_smi(executable: &Path) -> GpuCollection {
    if !executable.is_absolute() {
        return GpuCollection::Error(TelemetryMetricReason::BackendUnavailable);
    }
    let child = Command::new(executable)
        .args([
            "--query-gpu=index,name,utilization.gpu,memory.total,memory.used,temperature.gpu",
            "--format=csv,noheader,nounits",
        ])
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) => return GpuCollection::Error(io_reason(error)),
    };
    let Some(stdout) = child.stdout.take() else {
        return GpuCollection::Error(TelemetryMetricReason::BackendUnavailable);
    };
    let reader = thread::spawn(move || {
        let mut output = Vec::new();
        stdout
            .take(NVIDIA_OUTPUT_LIMIT + 1)
            .read_to_end(&mut output)
            .map(|_| output)
    });
    let deadline = Instant::now() + NVIDIA_TIMEOUT;
    let status = loop {
        let status = match child.try_wait() {
            Ok(status) => status,
            Err(_) => return GpuCollection::Error(TelemetryMetricReason::SourceUnavailable),
        };
        if let Some(status) = status {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return GpuCollection::Error(TelemetryMetricReason::Timeout);
        }
        thread::sleep(Duration::from_millis(20));
    };
    let output = match reader.join() {
        Ok(Ok(output)) => output,
        _ => return GpuCollection::Error(TelemetryMetricReason::SourceUnavailable),
    };
    if !status.success() || output.len() > NVIDIA_OUTPUT_LIMIT as usize {
        return GpuCollection::Error(TelemetryMetricReason::BackendUnavailable);
    }
    let Ok(output) = std::str::from_utf8(&output) else {
        return GpuCollection::Error(TelemetryMetricReason::ParseError);
    };
    match parse_nvidia_csv(output) {
        Ok(gpus) => GpuCollection::Available(gpus),
        Err(()) => GpuCollection::Error(TelemetryMetricReason::ParseError),
    }
}

fn io_reason(error: std::io::Error) -> TelemetryMetricReason {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => TelemetryMetricReason::PermissionDenied,
        std::io::ErrorKind::NotFound => TelemetryMetricReason::BackendUnavailable,
        _ => TelemetryMetricReason::SourceUnavailable,
    }
}

fn parse_nvidia_csv(output: &str) -> Result<Vec<GpuTelemetry>, ()> {
    let mut indexes = HashSet::new();
    let mut gpus = Vec::new();
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split(',').map(str::trim).collect::<Vec<_>>();
        if fields.len() != 6 || gpus.len() >= 8 {
            return Err(());
        }
        let index = fields[0].parse::<u8>().map_err(|_| ())?;
        let model = fields[1];
        if index >= 8
            || !indexes.insert(index)
            || model.is_empty()
            || model.len() > 128
            || model.chars().any(char::is_control)
        {
            return Err(());
        }
        let total_mib = fields[3].parse::<u64>().map_err(|_| ())?;
        let used_mib = fields[4].parse::<u64>().map_err(|_| ())?;
        if used_mib > total_mib {
            return Err(());
        }
        gpus.push(GpuTelemetry {
            index,
            status: TelemetryMetricStatus::Available,
            model: Some(model.to_owned()),
            utilization_percent: Some(parse_bounded(fields[2], 0.0, 100.0)?),
            memory_total_bytes: Some(total_mib.checked_mul(1024 * 1024).ok_or(())?),
            memory_used_bytes: Some(used_mib.checked_mul(1024 * 1024).ok_or(())?),
            temperature_celsius: Some(parse_bounded(fields[5], -100.0, 300.0)?),
        });
    }
    if gpus.is_empty() {
        return Err(());
    }
    Ok(gpus)
}

fn parse_bounded(value: &str, minimum: f64, maximum: f64) -> Result<f64, ()> {
    let value = value.parse::<f64>().map_err(|_| ())?;
    if value.is_finite() && (minimum..=maximum).contains(&value) {
        Ok(value)
    } else {
        Err(())
    }
}

fn read_limited(path: &Path, limit: u64) -> Result<String, ()> {
    let file = fs::File::open(path).map_err(|_| ())?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() > limit as usize {
        return Err(());
    }
    String::from_utf8(bytes).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvidia_csv_is_bounded_and_strict() {
        let gpus = parse_nvidia_csv("0, Test GPU, 25, 8192, 2048, 55\n").unwrap();
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].memory_total_bytes, Some(8192 * 1024 * 1024));

        assert!(parse_nvidia_csv("").is_err());
        assert!(parse_nvidia_csv("0, Test GPU, 101, 8192, 2048, 55").is_err());
        assert!(parse_nvidia_csv("0, Test GPU, 25, 1024, 2048, 55").is_err());
        assert!(parse_nvidia_csv("8, Test GPU, 25, 8192, 2048, 55").is_err());
        assert!(parse_nvidia_csv("0, Bad\nGPU, 25, 8192, 2048, 55").is_err());
        let nine = (0..9)
            .map(|index| format!("{index}, GPU {index}, 25, 8192, 2048, 55"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(parse_nvidia_csv(&nine).is_err());
    }

    #[test]
    fn nvidia_hardware_detection_distinguishes_absence_from_errors() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            detect_nvidia_hardware(root.path()),
            Err(TelemetryMetricReason::BackendUnavailable)
        );

        let devices = root.path().join("bus/pci/devices");
        fs::create_dir_all(&devices).unwrap();
        assert_eq!(detect_nvidia_hardware(root.path()), Ok(false));

        let device = devices.join("0000:01:00.0");
        fs::create_dir(&device).unwrap();
        fs::write(device.join("vendor"), "0x10de\n").unwrap();
        assert_eq!(detect_nvidia_hardware(root.path()), Ok(true));
    }
}
