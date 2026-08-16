// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'telemetry_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$TelemetryResponse extends TelemetryResponse {
  @override
  final String capability;
  @override
  final String? capabilityReason;
  @override
  final String? capturedAt;
  @override
  final String connectivity;
  @override
  final String freshness;
  @override
  final BuiltList<HistoryPoint> history;
  @override
  final LatestTelemetry? latest;
  @override
  final String nodeId;
  @override
  final String? receivedAt;

  factory _$TelemetryResponse([
    void Function(TelemetryResponseBuilder)? updates,
  ]) => (TelemetryResponseBuilder()..update(updates))._build();

  _$TelemetryResponse._({
    required this.capability,
    this.capabilityReason,
    this.capturedAt,
    required this.connectivity,
    required this.freshness,
    required this.history,
    this.latest,
    required this.nodeId,
    this.receivedAt,
  }) : super._();
  @override
  TelemetryResponse rebuild(void Function(TelemetryResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  TelemetryResponseBuilder toBuilder() =>
      TelemetryResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is TelemetryResponse &&
        capability == other.capability &&
        capabilityReason == other.capabilityReason &&
        capturedAt == other.capturedAt &&
        connectivity == other.connectivity &&
        freshness == other.freshness &&
        history == other.history &&
        latest == other.latest &&
        nodeId == other.nodeId &&
        receivedAt == other.receivedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, capability.hashCode);
    _$hash = $jc(_$hash, capabilityReason.hashCode);
    _$hash = $jc(_$hash, capturedAt.hashCode);
    _$hash = $jc(_$hash, connectivity.hashCode);
    _$hash = $jc(_$hash, freshness.hashCode);
    _$hash = $jc(_$hash, history.hashCode);
    _$hash = $jc(_$hash, latest.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, receivedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'TelemetryResponse')
          ..add('capability', capability)
          ..add('capabilityReason', capabilityReason)
          ..add('capturedAt', capturedAt)
          ..add('connectivity', connectivity)
          ..add('freshness', freshness)
          ..add('history', history)
          ..add('latest', latest)
          ..add('nodeId', nodeId)
          ..add('receivedAt', receivedAt))
        .toString();
  }
}

class TelemetryResponseBuilder
    implements Builder<TelemetryResponse, TelemetryResponseBuilder> {
  _$TelemetryResponse? _$v;

  String? _capability;
  String? get capability => _$this._capability;
  set capability(String? capability) => _$this._capability = capability;

  String? _capabilityReason;
  String? get capabilityReason => _$this._capabilityReason;
  set capabilityReason(String? capabilityReason) =>
      _$this._capabilityReason = capabilityReason;

  String? _capturedAt;
  String? get capturedAt => _$this._capturedAt;
  set capturedAt(String? capturedAt) => _$this._capturedAt = capturedAt;

  String? _connectivity;
  String? get connectivity => _$this._connectivity;
  set connectivity(String? connectivity) => _$this._connectivity = connectivity;

  String? _freshness;
  String? get freshness => _$this._freshness;
  set freshness(String? freshness) => _$this._freshness = freshness;

  ListBuilder<HistoryPoint>? _history;
  ListBuilder<HistoryPoint> get history =>
      _$this._history ??= ListBuilder<HistoryPoint>();
  set history(ListBuilder<HistoryPoint>? history) => _$this._history = history;

  LatestTelemetryBuilder? _latest;
  LatestTelemetryBuilder get latest =>
      _$this._latest ??= LatestTelemetryBuilder();
  set latest(LatestTelemetryBuilder? latest) => _$this._latest = latest;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  String? _receivedAt;
  String? get receivedAt => _$this._receivedAt;
  set receivedAt(String? receivedAt) => _$this._receivedAt = receivedAt;

  TelemetryResponseBuilder() {
    TelemetryResponse._defaults(this);
  }

  TelemetryResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _capability = $v.capability;
      _capabilityReason = $v.capabilityReason;
      _capturedAt = $v.capturedAt;
      _connectivity = $v.connectivity;
      _freshness = $v.freshness;
      _history = $v.history.toBuilder();
      _latest = $v.latest?.toBuilder();
      _nodeId = $v.nodeId;
      _receivedAt = $v.receivedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(TelemetryResponse other) {
    _$v = other as _$TelemetryResponse;
  }

  @override
  void update(void Function(TelemetryResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  TelemetryResponse build() => _build();

  _$TelemetryResponse _build() {
    _$TelemetryResponse _$result;
    try {
      _$result =
          _$v ??
          _$TelemetryResponse._(
            capability: BuiltValueNullFieldError.checkNotNull(
              capability,
              r'TelemetryResponse',
              'capability',
            ),
            capabilityReason: capabilityReason,
            capturedAt: capturedAt,
            connectivity: BuiltValueNullFieldError.checkNotNull(
              connectivity,
              r'TelemetryResponse',
              'connectivity',
            ),
            freshness: BuiltValueNullFieldError.checkNotNull(
              freshness,
              r'TelemetryResponse',
              'freshness',
            ),
            history: history.build(),
            latest: _latest?.build(),
            nodeId: BuiltValueNullFieldError.checkNotNull(
              nodeId,
              r'TelemetryResponse',
              'nodeId',
            ),
            receivedAt: receivedAt,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'history';
        history.build();
        _$failedField = 'latest';
        _latest?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'TelemetryResponse',
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
