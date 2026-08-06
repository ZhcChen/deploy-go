// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'deployment_target_run_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeploymentTargetRunResponse extends DeploymentTargetRunResponse {
  @override
  final String? agentId;
  @override
  final String createdAt;
  @override
  final String envGateStatus;
  @override
  final String? errorCode;
  @override
  final String? finishedAt;
  @override
  final String id;
  @override
  final String nodeId;
  @override
  final String phase;
  @override
  final String? resultSummary;
  @override
  final String? sourceRunId;
  @override
  final String? startedAt;
  @override
  final String status;
  @override
  final String targetId;
  @override
  final String updatedAt;

  factory _$DeploymentTargetRunResponse([
    void Function(DeploymentTargetRunResponseBuilder)? updates,
  ]) => (DeploymentTargetRunResponseBuilder()..update(updates))._build();

  _$DeploymentTargetRunResponse._({
    this.agentId,
    required this.createdAt,
    required this.envGateStatus,
    this.errorCode,
    this.finishedAt,
    required this.id,
    required this.nodeId,
    required this.phase,
    this.resultSummary,
    this.sourceRunId,
    this.startedAt,
    required this.status,
    required this.targetId,
    required this.updatedAt,
  }) : super._();
  @override
  DeploymentTargetRunResponse rebuild(
    void Function(DeploymentTargetRunResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  DeploymentTargetRunResponseBuilder toBuilder() =>
      DeploymentTargetRunResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeploymentTargetRunResponse &&
        agentId == other.agentId &&
        createdAt == other.createdAt &&
        envGateStatus == other.envGateStatus &&
        errorCode == other.errorCode &&
        finishedAt == other.finishedAt &&
        id == other.id &&
        nodeId == other.nodeId &&
        phase == other.phase &&
        resultSummary == other.resultSummary &&
        sourceRunId == other.sourceRunId &&
        startedAt == other.startedAt &&
        status == other.status &&
        targetId == other.targetId &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, agentId.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, envGateStatus.hashCode);
    _$hash = $jc(_$hash, errorCode.hashCode);
    _$hash = $jc(_$hash, finishedAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, phase.hashCode);
    _$hash = $jc(_$hash, resultSummary.hashCode);
    _$hash = $jc(_$hash, sourceRunId.hashCode);
    _$hash = $jc(_$hash, startedAt.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, targetId.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'DeploymentTargetRunResponse')
          ..add('agentId', agentId)
          ..add('createdAt', createdAt)
          ..add('envGateStatus', envGateStatus)
          ..add('errorCode', errorCode)
          ..add('finishedAt', finishedAt)
          ..add('id', id)
          ..add('nodeId', nodeId)
          ..add('phase', phase)
          ..add('resultSummary', resultSummary)
          ..add('sourceRunId', sourceRunId)
          ..add('startedAt', startedAt)
          ..add('status', status)
          ..add('targetId', targetId)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class DeploymentTargetRunResponseBuilder
    implements
        Builder<
          DeploymentTargetRunResponse,
          DeploymentTargetRunResponseBuilder
        > {
  _$DeploymentTargetRunResponse? _$v;

  String? _agentId;
  String? get agentId => _$this._agentId;
  set agentId(String? agentId) => _$this._agentId = agentId;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _envGateStatus;
  String? get envGateStatus => _$this._envGateStatus;
  set envGateStatus(String? envGateStatus) =>
      _$this._envGateStatus = envGateStatus;

  String? _errorCode;
  String? get errorCode => _$this._errorCode;
  set errorCode(String? errorCode) => _$this._errorCode = errorCode;

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

  String? _resultSummary;
  String? get resultSummary => _$this._resultSummary;
  set resultSummary(String? resultSummary) =>
      _$this._resultSummary = resultSummary;

  String? _sourceRunId;
  String? get sourceRunId => _$this._sourceRunId;
  set sourceRunId(String? sourceRunId) => _$this._sourceRunId = sourceRunId;

  String? _startedAt;
  String? get startedAt => _$this._startedAt;
  set startedAt(String? startedAt) => _$this._startedAt = startedAt;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _targetId;
  String? get targetId => _$this._targetId;
  set targetId(String? targetId) => _$this._targetId = targetId;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  DeploymentTargetRunResponseBuilder() {
    DeploymentTargetRunResponse._defaults(this);
  }

  DeploymentTargetRunResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _agentId = $v.agentId;
      _createdAt = $v.createdAt;
      _envGateStatus = $v.envGateStatus;
      _errorCode = $v.errorCode;
      _finishedAt = $v.finishedAt;
      _id = $v.id;
      _nodeId = $v.nodeId;
      _phase = $v.phase;
      _resultSummary = $v.resultSummary;
      _sourceRunId = $v.sourceRunId;
      _startedAt = $v.startedAt;
      _status = $v.status;
      _targetId = $v.targetId;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeploymentTargetRunResponse other) {
    _$v = other as _$DeploymentTargetRunResponse;
  }

  @override
  void update(void Function(DeploymentTargetRunResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  DeploymentTargetRunResponse build() => _build();

  _$DeploymentTargetRunResponse _build() {
    final _$result =
        _$v ??
        _$DeploymentTargetRunResponse._(
          agentId: agentId,
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'DeploymentTargetRunResponse',
            'createdAt',
          ),
          envGateStatus: BuiltValueNullFieldError.checkNotNull(
            envGateStatus,
            r'DeploymentTargetRunResponse',
            'envGateStatus',
          ),
          errorCode: errorCode,
          finishedAt: finishedAt,
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'DeploymentTargetRunResponse',
            'id',
          ),
          nodeId: BuiltValueNullFieldError.checkNotNull(
            nodeId,
            r'DeploymentTargetRunResponse',
            'nodeId',
          ),
          phase: BuiltValueNullFieldError.checkNotNull(
            phase,
            r'DeploymentTargetRunResponse',
            'phase',
          ),
          resultSummary: resultSummary,
          sourceRunId: sourceRunId,
          startedAt: startedAt,
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'DeploymentTargetRunResponse',
            'status',
          ),
          targetId: BuiltValueNullFieldError.checkNotNull(
            targetId,
            r'DeploymentTargetRunResponse',
            'targetId',
          ),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
            updatedAt,
            r'DeploymentTargetRunResponse',
            'updatedAt',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
