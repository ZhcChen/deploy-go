// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_upgrade_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentUpgradeResponse extends AgentUpgradeResponse {
  @override
  final String agentId;
  @override
  final int attemptCount;
  @override
  final String? currentVersion;
  @override
  final String? errorCode;
  @override
  final String? errorSummary;
  @override
  final String? finishedAt;
  @override
  final String id;
  @override
  final String nodeId;
  @override
  final String? phase;
  @override
  final String queuedAt;
  @override
  final String? startedAt;
  @override
  final String status;
  @override
  final String targetArchitecture;
  @override
  final String targetVersion;
  @override
  final String updatedAt;

  factory _$AgentUpgradeResponse([
    void Function(AgentUpgradeResponseBuilder)? updates,
  ]) => (AgentUpgradeResponseBuilder()..update(updates))._build();

  _$AgentUpgradeResponse._({
    required this.agentId,
    required this.attemptCount,
    this.currentVersion,
    this.errorCode,
    this.errorSummary,
    this.finishedAt,
    required this.id,
    required this.nodeId,
    this.phase,
    required this.queuedAt,
    this.startedAt,
    required this.status,
    required this.targetArchitecture,
    required this.targetVersion,
    required this.updatedAt,
  }) : super._();
  @override
  AgentUpgradeResponse rebuild(
    void Function(AgentUpgradeResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  AgentUpgradeResponseBuilder toBuilder() =>
      AgentUpgradeResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentUpgradeResponse &&
        agentId == other.agentId &&
        attemptCount == other.attemptCount &&
        currentVersion == other.currentVersion &&
        errorCode == other.errorCode &&
        errorSummary == other.errorSummary &&
        finishedAt == other.finishedAt &&
        id == other.id &&
        nodeId == other.nodeId &&
        phase == other.phase &&
        queuedAt == other.queuedAt &&
        startedAt == other.startedAt &&
        status == other.status &&
        targetArchitecture == other.targetArchitecture &&
        targetVersion == other.targetVersion &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, agentId.hashCode);
    _$hash = $jc(_$hash, attemptCount.hashCode);
    _$hash = $jc(_$hash, currentVersion.hashCode);
    _$hash = $jc(_$hash, errorCode.hashCode);
    _$hash = $jc(_$hash, errorSummary.hashCode);
    _$hash = $jc(_$hash, finishedAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, phase.hashCode);
    _$hash = $jc(_$hash, queuedAt.hashCode);
    _$hash = $jc(_$hash, startedAt.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, targetArchitecture.hashCode);
    _$hash = $jc(_$hash, targetVersion.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentUpgradeResponse')
          ..add('agentId', agentId)
          ..add('attemptCount', attemptCount)
          ..add('currentVersion', currentVersion)
          ..add('errorCode', errorCode)
          ..add('errorSummary', errorSummary)
          ..add('finishedAt', finishedAt)
          ..add('id', id)
          ..add('nodeId', nodeId)
          ..add('phase', phase)
          ..add('queuedAt', queuedAt)
          ..add('startedAt', startedAt)
          ..add('status', status)
          ..add('targetArchitecture', targetArchitecture)
          ..add('targetVersion', targetVersion)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class AgentUpgradeResponseBuilder
    implements Builder<AgentUpgradeResponse, AgentUpgradeResponseBuilder> {
  _$AgentUpgradeResponse? _$v;

  String? _agentId;
  String? get agentId => _$this._agentId;
  set agentId(String? agentId) => _$this._agentId = agentId;

  int? _attemptCount;
  int? get attemptCount => _$this._attemptCount;
  set attemptCount(int? attemptCount) => _$this._attemptCount = attemptCount;

  String? _currentVersion;
  String? get currentVersion => _$this._currentVersion;
  set currentVersion(String? currentVersion) =>
      _$this._currentVersion = currentVersion;

  String? _errorCode;
  String? get errorCode => _$this._errorCode;
  set errorCode(String? errorCode) => _$this._errorCode = errorCode;

  String? _errorSummary;
  String? get errorSummary => _$this._errorSummary;
  set errorSummary(String? errorSummary) => _$this._errorSummary = errorSummary;

  String? _finishedAt;
  String? get finishedAt => _$this._finishedAt;
  set finishedAt(String? finishedAt) => _$this._finishedAt = finishedAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  String? _phase;
  String? get phase => _$this._phase;
  set phase(String? phase) => _$this._phase = phase;

  String? _queuedAt;
  String? get queuedAt => _$this._queuedAt;
  set queuedAt(String? queuedAt) => _$this._queuedAt = queuedAt;

  String? _startedAt;
  String? get startedAt => _$this._startedAt;
  set startedAt(String? startedAt) => _$this._startedAt = startedAt;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _targetArchitecture;
  String? get targetArchitecture => _$this._targetArchitecture;
  set targetArchitecture(String? targetArchitecture) =>
      _$this._targetArchitecture = targetArchitecture;

  String? _targetVersion;
  String? get targetVersion => _$this._targetVersion;
  set targetVersion(String? targetVersion) =>
      _$this._targetVersion = targetVersion;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  AgentUpgradeResponseBuilder() {
    AgentUpgradeResponse._defaults(this);
  }

  AgentUpgradeResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _agentId = $v.agentId;
      _attemptCount = $v.attemptCount;
      _currentVersion = $v.currentVersion;
      _errorCode = $v.errorCode;
      _errorSummary = $v.errorSummary;
      _finishedAt = $v.finishedAt;
      _id = $v.id;
      _nodeId = $v.nodeId;
      _phase = $v.phase;
      _queuedAt = $v.queuedAt;
      _startedAt = $v.startedAt;
      _status = $v.status;
      _targetArchitecture = $v.targetArchitecture;
      _targetVersion = $v.targetVersion;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentUpgradeResponse other) {
    _$v = other as _$AgentUpgradeResponse;
  }

  @override
  void update(void Function(AgentUpgradeResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentUpgradeResponse build() => _build();

  _$AgentUpgradeResponse _build() {
    final _$result =
        _$v ??
        _$AgentUpgradeResponse._(
          agentId: BuiltValueNullFieldError.checkNotNull(
            agentId,
            r'AgentUpgradeResponse',
            'agentId',
          ),
          attemptCount: BuiltValueNullFieldError.checkNotNull(
            attemptCount,
            r'AgentUpgradeResponse',
            'attemptCount',
          ),
          currentVersion: currentVersion,
          errorCode: errorCode,
          errorSummary: errorSummary,
          finishedAt: finishedAt,
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'AgentUpgradeResponse',
            'id',
          ),
          nodeId: BuiltValueNullFieldError.checkNotNull(
            nodeId,
            r'AgentUpgradeResponse',
            'nodeId',
          ),
          phase: phase,
          queuedAt: BuiltValueNullFieldError.checkNotNull(
            queuedAt,
            r'AgentUpgradeResponse',
            'queuedAt',
          ),
          startedAt: startedAt,
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'AgentUpgradeResponse',
            'status',
          ),
          targetArchitecture: BuiltValueNullFieldError.checkNotNull(
            targetArchitecture,
            r'AgentUpgradeResponse',
            'targetArchitecture',
          ),
          targetVersion: BuiltValueNullFieldError.checkNotNull(
            targetVersion,
            r'AgentUpgradeResponse',
            'targetVersion',
          ),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
            updatedAt,
            r'AgentUpgradeResponse',
            'updatedAt',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
