// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'history_point.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$HistoryPoint extends HistoryPoint {
  @override
  final double? cpuUsageRatio;
  @override
  final double? diskBusyRatio;
  @override
  final double? diskReadBytesPerSecond;
  @override
  final double? diskWriteBytesPerSecond;
  @override
  final double? memoryUsedBytes;
  @override
  final double? networkReceiveBytesPerSecond;
  @override
  final double? networkTransmitBytesPerSecond;
  @override
  final String receivedAt;
  @override
  final double? workRootUsedBytes;

  factory _$HistoryPoint([void Function(HistoryPointBuilder)? updates]) =>
      (HistoryPointBuilder()..update(updates))._build();

  _$HistoryPoint._({
    this.cpuUsageRatio,
    this.diskBusyRatio,
    this.diskReadBytesPerSecond,
    this.diskWriteBytesPerSecond,
    this.memoryUsedBytes,
    this.networkReceiveBytesPerSecond,
    this.networkTransmitBytesPerSecond,
    required this.receivedAt,
    this.workRootUsedBytes,
  }) : super._();
  @override
  HistoryPoint rebuild(void Function(HistoryPointBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  HistoryPointBuilder toBuilder() => HistoryPointBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is HistoryPoint &&
        cpuUsageRatio == other.cpuUsageRatio &&
        diskBusyRatio == other.diskBusyRatio &&
        diskReadBytesPerSecond == other.diskReadBytesPerSecond &&
        diskWriteBytesPerSecond == other.diskWriteBytesPerSecond &&
        memoryUsedBytes == other.memoryUsedBytes &&
        networkReceiveBytesPerSecond == other.networkReceiveBytesPerSecond &&
        networkTransmitBytesPerSecond == other.networkTransmitBytesPerSecond &&
        receivedAt == other.receivedAt &&
        workRootUsedBytes == other.workRootUsedBytes;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, cpuUsageRatio.hashCode);
    _$hash = $jc(_$hash, diskBusyRatio.hashCode);
    _$hash = $jc(_$hash, diskReadBytesPerSecond.hashCode);
    _$hash = $jc(_$hash, diskWriteBytesPerSecond.hashCode);
    _$hash = $jc(_$hash, memoryUsedBytes.hashCode);
    _$hash = $jc(_$hash, networkReceiveBytesPerSecond.hashCode);
    _$hash = $jc(_$hash, networkTransmitBytesPerSecond.hashCode);
    _$hash = $jc(_$hash, receivedAt.hashCode);
    _$hash = $jc(_$hash, workRootUsedBytes.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'HistoryPoint')
          ..add('cpuUsageRatio', cpuUsageRatio)
          ..add('diskBusyRatio', diskBusyRatio)
          ..add('diskReadBytesPerSecond', diskReadBytesPerSecond)
          ..add('diskWriteBytesPerSecond', diskWriteBytesPerSecond)
          ..add('memoryUsedBytes', memoryUsedBytes)
          ..add('networkReceiveBytesPerSecond', networkReceiveBytesPerSecond)
          ..add('networkTransmitBytesPerSecond', networkTransmitBytesPerSecond)
          ..add('receivedAt', receivedAt)
          ..add('workRootUsedBytes', workRootUsedBytes))
        .toString();
  }
}

class HistoryPointBuilder
    implements Builder<HistoryPoint, HistoryPointBuilder> {
  _$HistoryPoint? _$v;

  double? _cpuUsageRatio;
  double? get cpuUsageRatio => _$this._cpuUsageRatio;
  set cpuUsageRatio(double? cpuUsageRatio) =>
      _$this._cpuUsageRatio = cpuUsageRatio;

  double? _diskBusyRatio;
  double? get diskBusyRatio => _$this._diskBusyRatio;
  set diskBusyRatio(double? diskBusyRatio) =>
      _$this._diskBusyRatio = diskBusyRatio;

  double? _diskReadBytesPerSecond;
  double? get diskReadBytesPerSecond => _$this._diskReadBytesPerSecond;
  set diskReadBytesPerSecond(double? diskReadBytesPerSecond) =>
      _$this._diskReadBytesPerSecond = diskReadBytesPerSecond;

  double? _diskWriteBytesPerSecond;
  double? get diskWriteBytesPerSecond => _$this._diskWriteBytesPerSecond;
  set diskWriteBytesPerSecond(double? diskWriteBytesPerSecond) =>
      _$this._diskWriteBytesPerSecond = diskWriteBytesPerSecond;

  double? _memoryUsedBytes;
  double? get memoryUsedBytes => _$this._memoryUsedBytes;
  set memoryUsedBytes(double? memoryUsedBytes) =>
      _$this._memoryUsedBytes = memoryUsedBytes;

  double? _networkReceiveBytesPerSecond;
  double? get networkReceiveBytesPerSecond =>
      _$this._networkReceiveBytesPerSecond;
  set networkReceiveBytesPerSecond(double? networkReceiveBytesPerSecond) =>
      _$this._networkReceiveBytesPerSecond = networkReceiveBytesPerSecond;

  double? _networkTransmitBytesPerSecond;
  double? get networkTransmitBytesPerSecond =>
      _$this._networkTransmitBytesPerSecond;
  set networkTransmitBytesPerSecond(double? networkTransmitBytesPerSecond) =>
      _$this._networkTransmitBytesPerSecond = networkTransmitBytesPerSecond;

  String? _receivedAt;
  String? get receivedAt => _$this._receivedAt;
  set receivedAt(String? receivedAt) => _$this._receivedAt = receivedAt;

  double? _workRootUsedBytes;
  double? get workRootUsedBytes => _$this._workRootUsedBytes;
  set workRootUsedBytes(double? workRootUsedBytes) =>
      _$this._workRootUsedBytes = workRootUsedBytes;

  HistoryPointBuilder() {
    HistoryPoint._defaults(this);
  }

  HistoryPointBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _cpuUsageRatio = $v.cpuUsageRatio;
      _diskBusyRatio = $v.diskBusyRatio;
      _diskReadBytesPerSecond = $v.diskReadBytesPerSecond;
      _diskWriteBytesPerSecond = $v.diskWriteBytesPerSecond;
      _memoryUsedBytes = $v.memoryUsedBytes;
      _networkReceiveBytesPerSecond = $v.networkReceiveBytesPerSecond;
      _networkTransmitBytesPerSecond = $v.networkTransmitBytesPerSecond;
      _receivedAt = $v.receivedAt;
      _workRootUsedBytes = $v.workRootUsedBytes;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(HistoryPoint other) {
    _$v = other as _$HistoryPoint;
  }

  @override
  void update(void Function(HistoryPointBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  HistoryPoint build() => _build();

  _$HistoryPoint _build() {
    final _$result =
        _$v ??
        _$HistoryPoint._(
          cpuUsageRatio: cpuUsageRatio,
          diskBusyRatio: diskBusyRatio,
          diskReadBytesPerSecond: diskReadBytesPerSecond,
          diskWriteBytesPerSecond: diskWriteBytesPerSecond,
          memoryUsedBytes: memoryUsedBytes,
          networkReceiveBytesPerSecond: networkReceiveBytesPerSecond,
          networkTransmitBytesPerSecond: networkTransmitBytesPerSecond,
          receivedAt: BuiltValueNullFieldError.checkNotNull(
            receivedAt,
            r'HistoryPoint',
            'receivedAt',
          ),
          workRootUsedBytes: workRootUsedBytes,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
