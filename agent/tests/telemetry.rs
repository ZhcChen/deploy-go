use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

use deploy_go_agent::telemetry::{GpuCollection, GpuReader, LinuxTelemetryCollector};
use deploy_go_agent_protocol::{GpuTelemetry, TelemetryMetricReason, TelemetryMetricStatus};
use tempfile::tempdir;

struct FixedGpu(GpuCollection);

impl GpuReader for FixedGpu {
    fn collect(&mut self) -> GpuCollection {
        self.0.clone()
    }
}

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn collector(gpu: GpuCollection) -> (tempfile::TempDir, LinuxTelemetryCollector) {
    let root = tempdir().unwrap();
    let proc_root = root.path().join("proc");
    let sys_root = root.path().join("sys");
    let work_root = root.path().join("work");
    fs::create_dir_all(&work_root).unwrap();
    write(&proc_root.join("stat"), "cpu 100 0 50 800 50 0 0 0\n");
    write(
        &proc_root.join("meminfo"),
        "MemTotal: 1000 kB\nMemAvailable: 400 kB\n",
    );
    write(
        &proc_root.join("net/dev"),
        "Inter-| Receive | Transmit\n face |bytes packets errs drop fifo frame compressed multicast|bytes packets errs drop fifo colls carrier compressed\n lo: 50 0 0 0 0 0 0 0 50 0 0 0 0 0 0 0\n eth0: 100 0 0 0 0 0 0 0 200 0 0 0 0 0 0 0\n",
    );
    let block = sys_root.join("block/sda");
    fs::create_dir_all(block.join("device")).unwrap();
    write(&block.join("stat"), "1 0 2 0 1 0 4 0 0 5 0\n");
    let loop_block = sys_root.join("block/loop0");
    fs::create_dir_all(loop_block.join("device")).unwrap();
    write(&loop_block.join("stat"), "1 0 999 0 1 0 999 0 0 999 0\n");
    let collector = LinuxTelemetryCollector::with_paths(
        work_root,
        proc_root,
        sys_root,
        Box::new(FixedGpu(gpu)),
    );
    (root, collector)
}

#[test]
fn linux_fixtures_collect_static_metrics_and_counter_rates() {
    let gpu = GpuTelemetry {
        index: 0,
        status: TelemetryMetricStatus::Available,
        model: Some("Test GPU".to_owned()),
        utilization_percent: Some(25.0),
        memory_total_bytes: Some(1024),
        memory_used_bytes: Some(512),
        temperature_celsius: Some(50.0),
    };
    let (root, mut collector) = collector(GpuCollection::Available(vec![gpu]));
    let started = Instant::now();
    let first = collector.collect_at(started);
    assert_eq!(first.cpu.status, TelemetryMetricStatus::WarmingUp);
    assert_eq!(first.disk_io.status, TelemetryMetricStatus::WarmingUp);
    assert_eq!(first.network.status, TelemetryMetricStatus::WarmingUp);
    assert_eq!(first.memory.total_bytes, Some(1_024_000));
    assert_eq!(first.memory.used_bytes, Some(614_400));
    assert_eq!(first.gpu_status, TelemetryMetricStatus::Available);
    assert_eq!(first.gpus.len(), 1);

    write(
        &root.path().join("proc/stat"),
        "cpu 120 0 60 900 60 0 0 0\n",
    );
    write(
        &root.path().join("proc/net/dev"),
        "Inter-| Receive | Transmit\n face |bytes packets errs drop fifo frame compressed multicast|bytes packets errs drop fifo colls carrier compressed\n lo: 90 0 0 0 0 0 0 0 90 0 0 0 0 0 0 0\n eth0: 500 0 0 0 0 0 0 0 800 0 0 0 0 0 0 0\n",
    );
    write(
        &root.path().join("sys/block/sda/stat"),
        "1 0 6 0 1 0 10 0 0 25 0\n",
    );
    let second = collector.collect_at(started + Duration::from_secs(2));
    assert_eq!(second.cpu.status, TelemetryMetricStatus::Available);
    assert!((second.cpu.usage_percent.unwrap() - 21.428).abs() < 0.01);
    assert_eq!(second.disk_io.read_bytes_per_second, Some(1024.0));
    assert_eq!(second.disk_io.write_bytes_per_second, Some(1536.0));
    assert_eq!(second.disk_io.busy_percent, Some(1.0));
    assert_eq!(second.network.receive_bytes_per_second, Some(200.0));
    assert_eq!(second.network.transmit_bytes_per_second, Some(300.0));
}

#[test]
fn disk_busy_uses_the_busiest_physical_device_while_summing_throughput() {
    let (root, mut collector) = collector(GpuCollection::Unsupported(
        TelemetryMetricReason::HardwareNotPresent,
    ));
    let second_disk = root.path().join("sys/block/sdb");
    fs::create_dir_all(second_disk.join("device")).unwrap();
    write(&second_disk.join("stat"), "1 0 4 0 1 0 8 0 0 20 0\n");

    let started = Instant::now();
    let first = collector.collect_at(started);
    assert_eq!(first.disk_io.status, TelemetryMetricStatus::WarmingUp);

    write(
        &root.path().join("sys/block/sda/stat"),
        "1 0 6 0 1 0 10 0 0 25 0\n",
    );
    write(
        &root.path().join("sys/block/sdb/stat"),
        "1 0 8 0 1 0 12 0 0 25 0\n",
    );
    let second = collector.collect_at(started + Duration::from_secs(1));

    assert_eq!(second.disk_io.read_bytes_per_second, Some(8.0 * 512.0));
    assert_eq!(second.disk_io.write_bytes_per_second, Some(10.0 * 512.0));
    assert_eq!(second.disk_io.busy_percent, Some(2.0));
}

#[test]
fn broken_sources_and_counter_rollbacks_are_isolated() {
    let (root, mut collector) = collector(GpuCollection::Error(TelemetryMetricReason::ParseError));
    let started = Instant::now();
    let first = collector.collect_at(started);
    assert_eq!(first.gpu_status, TelemetryMetricStatus::CollectionError);
    assert_eq!(first.gpu_reason, Some(TelemetryMetricReason::ParseError));

    write(&root.path().join("proc/stat"), "cpu broken\n");
    write(&root.path().join("proc/meminfo"), "MemTotal: 1000 bytes\n");
    fs::remove_dir_all(root.path().join("sys/block")).unwrap();
    let broken = collector.collect_at(started + Duration::from_secs(1));
    assert_eq!(broken.cpu.status, TelemetryMetricStatus::CollectionError);
    assert_eq!(broken.memory.status, TelemetryMetricStatus::CollectionError);
    assert_eq!(
        broken.disk_io.status,
        TelemetryMetricStatus::CollectionError
    );
    assert_eq!(
        broken.work_root_disk.status,
        TelemetryMetricStatus::Available
    );
}

#[test]
fn counter_rollbacks_return_to_warming_up_without_negative_rates() {
    let (root, mut collector) = collector(GpuCollection::Unsupported(
        TelemetryMetricReason::HardwareNotPresent,
    ));
    let started = Instant::now();
    let _ = collector.collect_at(started);
    write(&root.path().join("proc/stat"), "cpu 1 0 1 1 1 0 0 0\n");
    write(
        &root.path().join("proc/net/dev"),
        "Inter-| Receive | Transmit\n face |bytes packets errs drop fifo frame compressed multicast|bytes packets errs drop fifo colls carrier compressed\n eth0: 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0\n",
    );
    write(
        &root.path().join("sys/block/sda/stat"),
        "1 0 1 0 1 0 1 0 0 1 0\n",
    );
    let snapshot = collector.collect_at(started + Duration::from_secs(1));
    assert_eq!(snapshot.cpu.status, TelemetryMetricStatus::WarmingUp);
    assert_eq!(snapshot.disk_io.status, TelemetryMetricStatus::WarmingUp);
    assert_eq!(snapshot.network.status, TelemetryMetricStatus::WarmingUp);
    assert_eq!(snapshot.network.receive_bytes_per_second, None);
}

#[test]
fn a_new_collector_starts_rate_metrics_in_warming_up_state() {
    let (_first_root, mut first) = collector(GpuCollection::Unsupported(
        TelemetryMetricReason::HardwareNotPresent,
    ));
    let snapshot = first.collect_at(Instant::now());
    assert_eq!(snapshot.gpu_status, TelemetryMetricStatus::Unsupported);
    assert_eq!(
        snapshot.gpu_reason,
        Some(TelemetryMetricReason::HardwareNotPresent)
    );
    assert_eq!(snapshot.cpu.status, TelemetryMetricStatus::WarmingUp);

    let (_reconnected_root, mut reconnected) = collector(GpuCollection::Unsupported(
        TelemetryMetricReason::HardwareNotPresent,
    ));
    let snapshot = reconnected.collect_at(Instant::now());
    assert_eq!(snapshot.cpu.status, TelemetryMetricStatus::WarmingUp);
    assert_eq!(snapshot.network.status, TelemetryMetricStatus::WarmingUp);
}
