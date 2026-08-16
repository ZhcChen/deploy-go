// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'latest_telemetry.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$LatestTelemetry extends LatestTelemetry {
  @override
  final MetricValue cpuUsageRatio;
  @override
  final MetricValue diskBusyRatio;
  @override
  final MetricValue diskReadBytesPerSecond;
  @override
  final MetricValue diskWriteBytesPerSecond;
  @override
  final String? gpuReason;
  @override
  final String gpuStatus;
  @override
  final JsonObject? gpus;
  @override
  final MetricValue memoryTotalBytes;
  @override
  final MetricValue memoryUsedBytes;
  @override
  final MetricValue networkReceiveBytesPerSecond;
  @override
  final MetricValue networkTransmitBytesPerSecond;
  @override
  final MetricValue workRootTotalBytes;
  @override
  final MetricValue workRootUsedBytes;

  factory _$LatestTelemetry([void Function(LatestTelemetryBuilder)? updates]) =>
      (LatestTelemetryBuilder()..update(updates))._build();

  _$LatestTelemetry._({
    required this.cpuUsageRatio,
    required this.diskBusyRatio,
    required this.diskReadBytesPerSecond,
    required this.diskWriteBytesPerSecond,
    this.gpuReason,
    required this.gpuStatus,
    this.gpus,
    required this.memoryTotalBytes,
    required this.memoryUsedBytes,
    required this.networkReceiveBytesPerSecond,
    required this.networkTransmitBytesPerSecond,
    required this.workRootTotalBytes,
    required this.workRootUsedBytes,
  }) : super._();
  @override
  LatestTelemetry rebuild(void Function(LatestTelemetryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  LatestTelemetryBuilder toBuilder() => LatestTelemetryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is LatestTelemetry &&
        cpuUsageRatio == other.cpuUsageRatio &&
        diskBusyRatio == other.diskBusyRatio &&
        diskReadBytesPerSecond == other.diskReadBytesPerSecond &&
        diskWriteBytesPerSecond == other.diskWriteBytesPerSecond &&
        gpuReason == other.gpuReason &&
        gpuStatus == other.gpuStatus &&
        gpus == other.gpus &&
        memoryTotalBytes == other.memoryTotalBytes &&
        memoryUsedBytes == other.memoryUsedBytes &&
        networkReceiveBytesPerSecond == other.networkReceiveBytesPerSecond &&
        networkTransmitBytesPerSecond == other.networkTransmitBytesPerSecond &&
        workRootTotalBytes == other.workRootTotalBytes &&
        workRootUsedBytes == other.workRootUsedBytes;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, cpuUsageRatio.hashCode);
    _$hash = $jc(_$hash, diskBusyRatio.hashCode);
    _$hash = $jc(_$hash, diskReadBytesPerSecond.hashCode);
    _$hash = $jc(_$hash, diskWriteBytesPerSecond.hashCode);
    _$hash = $jc(_$hash, gpuReason.hashCode);
    _$hash = $jc(_$hash, gpuStatus.hashCode);
    _$hash = $jc(_$hash, gpus.hashCode);
    _$hash = $jc(_$hash, memoryTotalBytes.hashCode);
    _$hash = $jc(_$hash, memoryUsedBytes.hashCode);
    _$hash = $jc(_$hash, networkReceiveBytesPerSecond.hashCode);
    _$hash = $jc(_$hash, networkTransmitBytesPerSecond.hashCode);
    _$hash = $jc(_$hash, workRootTotalBytes.hashCode);
    _$hash = $jc(_$hash, workRootUsedBytes.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'LatestTelemetry')
          ..add('cpuUsageRatio', cpuUsageRatio)
          ..add('diskBusyRatio', diskBusyRatio)
          ..add('diskReadBytesPerSecond', diskReadBytesPerSecond)
          ..add('diskWriteBytesPerSecond', diskWriteBytesPerSecond)
          ..add('gpuReason', gpuReason)
          ..add('gpuStatus', gpuStatus)
          ..add('gpus', gpus)
          ..add('memoryTotalBytes', memoryTotalBytes)
          ..add('memoryUsedBytes', memoryUsedBytes)
          ..add('networkReceiveBytesPerSecond', networkReceiveBytesPerSecond)
          ..add('networkTransmitBytesPerSecond', networkTransmitBytesPerSecond)
          ..add('workRootTotalBytes', workRootTotalBytes)
          ..add('workRootUsedBytes', workRootUsedBytes))
        .toString();
  }
}

class LatestTelemetryBuilder
    implements Builder<LatestTelemetry, LatestTelemetryBuilder> {
  _$LatestTelemetry? _$v;

  MetricValueBuilder? _cpuUsageRatio;
  MetricValueBuilder get cpuUsageRatio =>
      _$this._cpuUsageRatio ??= MetricValueBuilder();
  set cpuUsageRatio(MetricValueBuilder? cpuUsageRatio) =>
      _$this._cpuUsageRatio = cpuUsageRatio;

  MetricValueBuilder? _diskBusyRatio;
  MetricValueBuilder get diskBusyRatio =>
      _$this._diskBusyRatio ??= MetricValueBuilder();
  set diskBusyRatio(MetricValueBuilder? diskBusyRatio) =>
      _$this._diskBusyRatio = diskBusyRatio;

  MetricValueBuilder? _diskReadBytesPerSecond;
  MetricValueBuilder get diskReadBytesPerSecond =>
      _$this._diskReadBytesPerSecond ??= MetricValueBuilder();
  set diskReadBytesPerSecond(MetricValueBuilder? diskReadBytesPerSecond) =>
      _$this._diskReadBytesPerSecond = diskReadBytesPerSecond;

  MetricValueBuilder? _diskWriteBytesPerSecond;
  MetricValueBuilder get diskWriteBytesPerSecond =>
      _$this._diskWriteBytesPerSecond ??= MetricValueBuilder();
  set diskWriteBytesPerSecond(MetricValueBuilder? diskWriteBytesPerSecond) =>
      _$this._diskWriteBytesPerSecond = diskWriteBytesPerSecond;

  String? _gpuReason;
  String? get gpuReason => _$this._gpuReason;
  set gpuReason(String? gpuReason) => _$this._gpuReason = gpuReason;

  String? _gpuStatus;
  String? get gpuStatus => _$this._gpuStatus;
  set gpuStatus(String? gpuStatus) => _$this._gpuStatus = gpuStatus;

  JsonObject? _gpus;
  JsonObject? get gpus => _$this._gpus;
  set gpus(JsonObject? gpus) => _$this._gpus = gpus;

  MetricValueBuilder? _memoryTotalBytes;
  MetricValueBuilder get memoryTotalBytes =>
      _$this._memoryTotalBytes ??= MetricValueBuilder();
  set memoryTotalBytes(MetricValueBuilder? memoryTotalBytes) =>
      _$this._memoryTotalBytes = memoryTotalBytes;

  MetricValueBuilder? _memoryUsedBytes;
  MetricValueBuilder get memoryUsedBytes =>
      _$this._memoryUsedBytes ??= MetricValueBuilder();
  set memoryUsedBytes(MetricValueBuilder? memoryUsedBytes) =>
      _$this._memoryUsedBytes = memoryUsedBytes;

  MetricValueBuilder? _networkReceiveBytesPerSecond;
  MetricValueBuilder get networkReceiveBytesPerSecond =>
      _$this._networkReceiveBytesPerSecond ??= MetricValueBuilder();
  set networkReceiveBytesPerSecond(
    MetricValueBuilder? networkReceiveBytesPerSecond,
  ) => _$this._networkReceiveBytesPerSecond = networkReceiveBytesPerSecond;

  MetricValueBuilder? _networkTransmitBytesPerSecond;
  MetricValueBuilder get networkTransmitBytesPerSecond =>
      _$this._networkTransmitBytesPerSecond ??= MetricValueBuilder();
  set networkTransmitBytesPerSecond(
    MetricValueBuilder? networkTransmitBytesPerSecond,
  ) => _$this._networkTransmitBytesPerSecond = networkTransmitBytesPerSecond;

  MetricValueBuilder? _workRootTotalBytes;
  MetricValueBuilder get workRootTotalBytes =>
      _$this._workRootTotalBytes ??= MetricValueBuilder();
  set workRootTotalBytes(MetricValueBuilder? workRootTotalBytes) =>
      _$this._workRootTotalBytes = workRootTotalBytes;

  MetricValueBuilder? _workRootUsedBytes;
  MetricValueBuilder get workRootUsedBytes =>
      _$this._workRootUsedBytes ??= MetricValueBuilder();
  set workRootUsedBytes(MetricValueBuilder? workRootUsedBytes) =>
      _$this._workRootUsedBytes = workRootUsedBytes;

  LatestTelemetryBuilder() {
    LatestTelemetry._defaults(this);
  }

  LatestTelemetryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _cpuUsageRatio = $v.cpuUsageRatio.toBuilder();
      _diskBusyRatio = $v.diskBusyRatio.toBuilder();
      _diskReadBytesPerSecond = $v.diskReadBytesPerSecond.toBuilder();
      _diskWriteBytesPerSecond = $v.diskWriteBytesPerSecond.toBuilder();
      _gpuReason = $v.gpuReason;
      _gpuStatus = $v.gpuStatus;
      _gpus = $v.gpus;
      _memoryTotalBytes = $v.memoryTotalBytes.toBuilder();
      _memoryUsedBytes = $v.memoryUsedBytes.toBuilder();
      _networkReceiveBytesPerSecond = $v.networkReceiveBytesPerSecond
          .toBuilder();
      _networkTransmitBytesPerSecond = $v.networkTransmitBytesPerSecond
          .toBuilder();
      _workRootTotalBytes = $v.workRootTotalBytes.toBuilder();
      _workRootUsedBytes = $v.workRootUsedBytes.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(LatestTelemetry other) {
    _$v = other as _$LatestTelemetry;
  }

  @override
  void update(void Function(LatestTelemetryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  LatestTelemetry build() => _build();

  _$LatestTelemetry _build() {
    _$LatestTelemetry _$result;
    try {
      _$result =
          _$v ??
          _$LatestTelemetry._(
            cpuUsageRatio: cpuUsageRatio.build(),
            diskBusyRatio: diskBusyRatio.build(),
            diskReadBytesPerSecond: diskReadBytesPerSecond.build(),
            diskWriteBytesPerSecond: diskWriteBytesPerSecond.build(),
            gpuReason: gpuReason,
            gpuStatus: BuiltValueNullFieldError.checkNotNull(
              gpuStatus,
              r'LatestTelemetry',
              'gpuStatus',
            ),
            gpus: gpus,
            memoryTotalBytes: memoryTotalBytes.build(),
            memoryUsedBytes: memoryUsedBytes.build(),
            networkReceiveBytesPerSecond: networkReceiveBytesPerSecond.build(),
            networkTransmitBytesPerSecond: networkTransmitBytesPerSecond
                .build(),
            workRootTotalBytes: workRootTotalBytes.build(),
            workRootUsedBytes: workRootUsedBytes.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'cpuUsageRatio';
        cpuUsageRatio.build();
        _$failedField = 'diskBusyRatio';
        diskBusyRatio.build();
        _$failedField = 'diskReadBytesPerSecond';
        diskReadBytesPerSecond.build();
        _$failedField = 'diskWriteBytesPerSecond';
        diskWriteBytesPerSecond.build();

        _$failedField = 'memoryTotalBytes';
        memoryTotalBytes.build();
        _$failedField = 'memoryUsedBytes';
        memoryUsedBytes.build();
        _$failedField = 'networkReceiveBytesPerSecond';
        networkReceiveBytesPerSecond.build();
        _$failedField = 'networkTransmitBytesPerSecond';
        networkTransmitBytesPerSecond.build();
        _$failedField = 'workRootTotalBytes';
        workRootTotalBytes.build();
        _$failedField = 'workRootUsedBytes';
        workRootUsedBytes.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'LatestTelemetry',
          _$failedField,
          e.toString(),
        );
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
