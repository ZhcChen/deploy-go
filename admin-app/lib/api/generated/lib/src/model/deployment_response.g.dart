// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'deployment_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeploymentResponse extends DeploymentResponse {
  @override
  final String applicationId;
  @override
  final String applicationName;
  @override
  final String? cancelRequestedAt;
  @override
  final String createdAt;
  @override
  final String? deploymentBranch;
  @override
  final String executionMode;
  @override
  final int? exitCode;
  @override
  final String? finishedAt;
  @override
  final String id;
  @override
  final JsonObject? imageSpec;
  @override
  final BuiltList<String>? modules;
  @override
  final String phase;
  @override
  final bool protocolComplete;
  @override
  final String queuedAt;
  @override
  final String releaseStrategy;
  @override
  final String? releaseVersion;
  @override
  final String requestedBy;
  @override
  final String? resolvedCommitSha;
  @override
  final String? resultSummary;
  @override
  final String? retryOfId;
  @override
  final String snapshotHash;
  @override
  final BuiltList<DeploymentStageTaskSummary> stageTasks;
  @override
  final String? startedAt;
  @override
  final String status;
  @override
  final String targetId;
  @override
  final BuiltList<DeploymentTargetRunResponse> targetRuns;
  @override
  final String updatedAt;
  @override
  final int version;

  factory _$DeploymentResponse([
    void Function(DeploymentResponseBuilder)? updates,
  ]) => (DeploymentResponseBuilder()..update(updates))._build();

  _$DeploymentResponse._({
    required this.applicationId,
    required this.applicationName,
    this.cancelRequestedAt,
    required this.createdAt,
    this.deploymentBranch,
    required this.executionMode,
    this.exitCode,
    this.finishedAt,
    required this.id,
    this.imageSpec,
    this.modules,
    required this.phase,
    required this.protocolComplete,
    required this.queuedAt,
    required this.releaseStrategy,
    this.releaseVersion,
    required this.requestedBy,
    this.resolvedCommitSha,
    this.resultSummary,
    this.retryOfId,
    required this.snapshotHash,
    required this.stageTasks,
    this.startedAt,
    required this.status,
    required this.targetId,
    required this.targetRuns,
    required this.updatedAt,
    required this.version,
  }) : super._();
  @override
  DeploymentResponse rebuild(
    void Function(DeploymentResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  DeploymentResponseBuilder toBuilder() =>
      DeploymentResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeploymentResponse &&
        applicationId == other.applicationId &&
        applicationName == other.applicationName &&
        cancelRequestedAt == other.cancelRequestedAt &&
        createdAt == other.createdAt &&
        deploymentBranch == other.deploymentBranch &&
        executionMode == other.executionMode &&
        exitCode == other.exitCode &&
        finishedAt == other.finishedAt &&
        id == other.id &&
        imageSpec == other.imageSpec &&
        modules == other.modules &&
        phase == other.phase &&
        protocolComplete == other.protocolComplete &&
        queuedAt == other.queuedAt &&
        releaseStrategy == other.releaseStrategy &&
        releaseVersion == other.releaseVersion &&
        requestedBy == other.requestedBy &&
        resolvedCommitSha == other.resolvedCommitSha &&
        resultSummary == other.resultSummary &&
        retryOfId == other.retryOfId &&
        snapshotHash == other.snapshotHash &&
        stageTasks == other.stageTasks &&
        startedAt == other.startedAt &&
        status == other.status &&
        targetId == other.targetId &&
        targetRuns == other.targetRuns &&
        updatedAt == other.updatedAt &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationId.hashCode);
    _$hash = $jc(_$hash, applicationName.hashCode);
    _$hash = $jc(_$hash, cancelRequestedAt.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, deploymentBranch.hashCode);
    _$hash = $jc(_$hash, executionMode.hashCode);
    _$hash = $jc(_$hash, exitCode.hashCode);
    _$hash = $jc(_$hash, finishedAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, imageSpec.hashCode);
    _$hash = $jc(_$hash, modules.hashCode);
    _$hash = $jc(_$hash, phase.hashCode);
    _$hash = $jc(_$hash, protocolComplete.hashCode);
    _$hash = $jc(_$hash, queuedAt.hashCode);
    _$hash = $jc(_$hash, releaseStrategy.hashCode);
    _$hash = $jc(_$hash, releaseVersion.hashCode);
    _$hash = $jc(_$hash, requestedBy.hashCode);
    _$hash = $jc(_$hash, resolvedCommitSha.hashCode);
    _$hash = $jc(_$hash, resultSummary.hashCode);
    _$hash = $jc(_$hash, retryOfId.hashCode);
    _$hash = $jc(_$hash, snapshotHash.hashCode);
    _$hash = $jc(_$hash, stageTasks.hashCode);
    _$hash = $jc(_$hash, startedAt.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, targetId.hashCode);
    _$hash = $jc(_$hash, targetRuns.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'DeploymentResponse')
          ..add('applicationId', applicationId)
          ..add('applicationName', applicationName)
          ..add('cancelRequestedAt', cancelRequestedAt)
          ..add('createdAt', createdAt)
          ..add('deploymentBranch', deploymentBranch)
          ..add('executionMode', executionMode)
          ..add('exitCode', exitCode)
          ..add('finishedAt', finishedAt)
          ..add('id', id)
          ..add('imageSpec', imageSpec)
          ..add('modules', modules)
          ..add('phase', phase)
          ..add('protocolComplete', protocolComplete)
          ..add('queuedAt', queuedAt)
          ..add('releaseStrategy', releaseStrategy)
          ..add('releaseVersion', releaseVersion)
          ..add('requestedBy', requestedBy)
          ..add('resolvedCommitSha', resolvedCommitSha)
          ..add('resultSummary', resultSummary)
          ..add('retryOfId', retryOfId)
          ..add('snapshotHash', snapshotHash)
          ..add('stageTasks', stageTasks)
          ..add('startedAt', startedAt)
          ..add('status', status)
          ..add('targetId', targetId)
          ..add('targetRuns', targetRuns)
          ..add('updatedAt', updatedAt)
          ..add('version', version))
        .toString();
  }
}

class DeploymentResponseBuilder
    implements Builder<DeploymentResponse, DeploymentResponseBuilder> {
  _$DeploymentResponse? _$v;

  String? _applicationId;
  String? get applicationId => _$this._applicationId;
  set applicationId(String? applicationId) =>
      _$this._applicationId = applicationId;

  String? _applicationName;
  String? get applicationName => _$this._applicationName;
  set applicationName(String? applicationName) =>
      _$this._applicationName = applicationName;

  String? _cancelRequestedAt;
  String? get cancelRequestedAt => _$this._cancelRequestedAt;
  set cancelRequestedAt(String? cancelRequestedAt) =>
      _$this._cancelRequestedAt = cancelRequestedAt;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _deploymentBranch;
  String? get deploymentBranch => _$this._deploymentBranch;
  set deploymentBranch(String? deploymentBranch) =>
      _$this._deploymentBranch = deploymentBranch;

  String? _executionMode;
  String? get executionMode => _$this._executionMode;
  set executionMode(String? executionMode) =>
      _$this._executionMode = executionMode;

  int? _exitCode;
  int? get exitCode => _$this._exitCode;
  set exitCode(int? exitCode) => _$this._exitCode = exitCode;

  String? _finishedAt;
  String? get finishedAt => _$this._finishedAt;
  set finishedAt(String? finishedAt) => _$this._finishedAt = finishedAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  JsonObject? _imageSpec;
  JsonObject? get imageSpec => _$this._imageSpec;
  set imageSpec(JsonObject? imageSpec) => _$this._imageSpec = imageSpec;

  ListBuilder<String>? _modules;
  ListBuilder<String> get modules => _$this._modules ??= ListBuilder<String>();
  set modules(ListBuilder<String>? modules) => _$this._modules = modules;

  String? _phase;
  String? get phase => _$this._phase;
  set phase(String? phase) => _$this._phase = phase;

  bool? _protocolComplete;
  bool? get protocolComplete => _$this._protocolComplete;
  set protocolComplete(bool? protocolComplete) =>
      _$this._protocolComplete = protocolComplete;

  String? _queuedAt;
  String? get queuedAt => _$this._queuedAt;
  set queuedAt(String? queuedAt) => _$this._queuedAt = queuedAt;

  String? _releaseStrategy;
  String? get releaseStrategy => _$this._releaseStrategy;
  set releaseStrategy(String? releaseStrategy) =>
      _$this._releaseStrategy = releaseStrategy;

  String? _releaseVersion;
  String? get releaseVersion => _$this._releaseVersion;
  set releaseVersion(String? releaseVersion) =>
      _$this._releaseVersion = releaseVersion;

  String? _requestedBy;
  String? get requestedBy => _$this._requestedBy;
  set requestedBy(String? requestedBy) => _$this._requestedBy = requestedBy;

  String? _resolvedCommitSha;
  String? get resolvedCommitSha => _$this._resolvedCommitSha;
  set resolvedCommitSha(String? resolvedCommitSha) =>
      _$this._resolvedCommitSha = resolvedCommitSha;

  String? _resultSummary;
  String? get resultSummary => _$this._resultSummary;
  set resultSummary(String? resultSummary) =>
      _$this._resultSummary = resultSummary;

  String? _retryOfId;
  String? get retryOfId => _$this._retryOfId;
  set retryOfId(String? retryOfId) => _$this._retryOfId = retryOfId;

  String? _snapshotHash;
  String? get snapshotHash => _$this._snapshotHash;
  set snapshotHash(String? snapshotHash) => _$this._snapshotHash = snapshotHash;

  ListBuilder<DeploymentStageTaskSummary>? _stageTasks;
  ListBuilder<DeploymentStageTaskSummary> get stageTasks =>
      _$this._stageTasks ??= ListBuilder<DeploymentStageTaskSummary>();
  set stageTasks(ListBuilder<DeploymentStageTaskSummary>? stageTasks) =>
      _$this._stageTasks = stageTasks;

  String? _startedAt;
  String? get startedAt => _$this._startedAt;
  set startedAt(String? startedAt) => _$this._startedAt = startedAt;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _targetId;
  String? get targetId => _$this._targetId;
  set targetId(String? targetId) => _$this._targetId = targetId;

  ListBuilder<DeploymentTargetRunResponse>? _targetRuns;
  ListBuilder<DeploymentTargetRunResponse> get targetRuns =>
      _$this._targetRuns ??= ListBuilder<DeploymentTargetRunResponse>();
  set targetRuns(ListBuilder<DeploymentTargetRunResponse>? targetRuns) =>
      _$this._targetRuns = targetRuns;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  DeploymentResponseBuilder() {
    DeploymentResponse._defaults(this);
  }

  DeploymentResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationId = $v.applicationId;
      _applicationName = $v.applicationName;
      _cancelRequestedAt = $v.cancelRequestedAt;
      _createdAt = $v.createdAt;
      _deploymentBranch = $v.deploymentBranch;
      _executionMode = $v.executionMode;
      _exitCode = $v.exitCode;
      _finishedAt = $v.finishedAt;
      _id = $v.id;
      _imageSpec = $v.imageSpec;
      _modules = $v.modules?.toBuilder();
      _phase = $v.phase;
      _protocolComplete = $v.protocolComplete;
      _queuedAt = $v.queuedAt;
      _releaseStrategy = $v.releaseStrategy;
      _releaseVersion = $v.releaseVersion;
      _requestedBy = $v.requestedBy;
      _resolvedCommitSha = $v.resolvedCommitSha;
      _resultSummary = $v.resultSummary;
      _retryOfId = $v.retryOfId;
      _snapshotHash = $v.snapshotHash;
      _stageTasks = $v.stageTasks.toBuilder();
      _startedAt = $v.startedAt;
      _status = $v.status;
      _targetId = $v.targetId;
      _targetRuns = $v.targetRuns.toBuilder();
      _updatedAt = $v.updatedAt;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeploymentResponse other) {
    _$v = other as _$DeploymentResponse;
  }

  @override
  void update(void Function(DeploymentResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  DeploymentResponse build() => _build();

  _$DeploymentResponse _build() {
    _$DeploymentResponse _$result;
    try {
      _$result =
          _$v ??
          _$DeploymentResponse._(
            applicationId: BuiltValueNullFieldError.checkNotNull(
              applicationId,
              r'DeploymentResponse',
              'applicationId',
            ),
            applicationName: BuiltValueNullFieldError.checkNotNull(
              applicationName,
              r'DeploymentResponse',
              'applicationName',
            ),
            cancelRequestedAt: cancelRequestedAt,
            createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt,
              r'DeploymentResponse',
              'createdAt',
            ),
            deploymentBranch: deploymentBranch,
            executionMode: BuiltValueNullFieldError.checkNotNull(
              executionMode,
              r'DeploymentResponse',
              'executionMode',
            ),
            exitCode: exitCode,
            finishedAt: finishedAt,
            id: BuiltValueNullFieldError.checkNotNull(
              id,
              r'DeploymentResponse',
              'id',
            ),
            imageSpec: imageSpec,
            modules: _modules?.build(),
            phase: BuiltValueNullFieldError.checkNotNull(
              phase,
              r'DeploymentResponse',
              'phase',
            ),
            protocolComplete: BuiltValueNullFieldError.checkNotNull(
              protocolComplete,
              r'DeploymentResponse',
              'protocolComplete',
            ),
            queuedAt: BuiltValueNullFieldError.checkNotNull(
              queuedAt,
              r'DeploymentResponse',
              'queuedAt',
            ),
            releaseStrategy: BuiltValueNullFieldError.checkNotNull(
              releaseStrategy,
              r'DeploymentResponse',
              'releaseStrategy',
            ),
            releaseVersion: releaseVersion,
            requestedBy: BuiltValueNullFieldError.checkNotNull(
              requestedBy,
              r'DeploymentResponse',
              'requestedBy',
            ),
            resolvedCommitSha: resolvedCommitSha,
            resultSummary: resultSummary,
            retryOfId: retryOfId,
            snapshotHash: BuiltValueNullFieldError.checkNotNull(
              snapshotHash,
              r'DeploymentResponse',
              'snapshotHash',
            ),
            stageTasks: stageTasks.build(),
            startedAt: startedAt,
            status: BuiltValueNullFieldError.checkNotNull(
              status,
              r'DeploymentResponse',
              'status',
            ),
            targetId: BuiltValueNullFieldError.checkNotNull(
              targetId,
              r'DeploymentResponse',
              'targetId',
            ),
            targetRuns: targetRuns.build(),
            updatedAt: BuiltValueNullFieldError.checkNotNull(
              updatedAt,
              r'DeploymentResponse',
              'updatedAt',
            ),
            version: BuiltValueNullFieldError.checkNotNull(
              version,
              r'DeploymentResponse',
              'version',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'modules';
        _modules?.build();

        _$failedField = 'stageTasks';
        stageTasks.build();

        _$failedField = 'targetRuns';
        targetRuns.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'DeploymentResponse',
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
