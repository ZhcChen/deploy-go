// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'deployment_preview_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeploymentPreviewResponse extends DeploymentPreviewResponse {
  @override
  final String applicationId;
  @override
  final String applicationName;
  @override
  final String? deploymentBranch;
  @override
  final String environment;
  @override
  final String executionMode;
  @override
  final JsonObject? imageSpec;
  @override
  final BuiltList<String>? modules;
  @override
  final String nodeId;
  @override
  final String nodeName;
  @override
  final JsonObject? parameters;
  @override
  final String? previewExpiresAt;
  @override
  final String releaseStrategy;
  @override
  final String? releaseVersion;
  @override
  final String? resolvedCommitSha;
  @override
  final String scriptPath;
  @override
  final String snapshotHash;
  @override
  final String? sourcePolicy;
  @override
  final String targetCode;
  @override
  final String targetId;

  factory _$DeploymentPreviewResponse([
    void Function(DeploymentPreviewResponseBuilder)? updates,
  ]) => (DeploymentPreviewResponseBuilder()..update(updates))._build();

  _$DeploymentPreviewResponse._({
    required this.applicationId,
    required this.applicationName,
    this.deploymentBranch,
    required this.environment,
    required this.executionMode,
    this.imageSpec,
    this.modules,
    required this.nodeId,
    required this.nodeName,
    this.parameters,
    this.previewExpiresAt,
    required this.releaseStrategy,
    this.releaseVersion,
    this.resolvedCommitSha,
    required this.scriptPath,
    required this.snapshotHash,
    this.sourcePolicy,
    required this.targetCode,
    required this.targetId,
  }) : super._();
  @override
  DeploymentPreviewResponse rebuild(
    void Function(DeploymentPreviewResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  DeploymentPreviewResponseBuilder toBuilder() =>
      DeploymentPreviewResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeploymentPreviewResponse &&
        applicationId == other.applicationId &&
        applicationName == other.applicationName &&
        deploymentBranch == other.deploymentBranch &&
        environment == other.environment &&
        executionMode == other.executionMode &&
        imageSpec == other.imageSpec &&
        modules == other.modules &&
        nodeId == other.nodeId &&
        nodeName == other.nodeName &&
        parameters == other.parameters &&
        previewExpiresAt == other.previewExpiresAt &&
        releaseStrategy == other.releaseStrategy &&
        releaseVersion == other.releaseVersion &&
        resolvedCommitSha == other.resolvedCommitSha &&
        scriptPath == other.scriptPath &&
        snapshotHash == other.snapshotHash &&
        sourcePolicy == other.sourcePolicy &&
        targetCode == other.targetCode &&
        targetId == other.targetId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationId.hashCode);
    _$hash = $jc(_$hash, applicationName.hashCode);
    _$hash = $jc(_$hash, deploymentBranch.hashCode);
    _$hash = $jc(_$hash, environment.hashCode);
    _$hash = $jc(_$hash, executionMode.hashCode);
    _$hash = $jc(_$hash, imageSpec.hashCode);
    _$hash = $jc(_$hash, modules.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, nodeName.hashCode);
    _$hash = $jc(_$hash, parameters.hashCode);
    _$hash = $jc(_$hash, previewExpiresAt.hashCode);
    _$hash = $jc(_$hash, releaseStrategy.hashCode);
    _$hash = $jc(_$hash, releaseVersion.hashCode);
    _$hash = $jc(_$hash, resolvedCommitSha.hashCode);
    _$hash = $jc(_$hash, scriptPath.hashCode);
    _$hash = $jc(_$hash, snapshotHash.hashCode);
    _$hash = $jc(_$hash, sourcePolicy.hashCode);
    _$hash = $jc(_$hash, targetCode.hashCode);
    _$hash = $jc(_$hash, targetId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'DeploymentPreviewResponse')
          ..add('applicationId', applicationId)
          ..add('applicationName', applicationName)
          ..add('deploymentBranch', deploymentBranch)
          ..add('environment', environment)
          ..add('executionMode', executionMode)
          ..add('imageSpec', imageSpec)
          ..add('modules', modules)
          ..add('nodeId', nodeId)
          ..add('nodeName', nodeName)
          ..add('parameters', parameters)
          ..add('previewExpiresAt', previewExpiresAt)
          ..add('releaseStrategy', releaseStrategy)
          ..add('releaseVersion', releaseVersion)
          ..add('resolvedCommitSha', resolvedCommitSha)
          ..add('scriptPath', scriptPath)
          ..add('snapshotHash', snapshotHash)
          ..add('sourcePolicy', sourcePolicy)
          ..add('targetCode', targetCode)
          ..add('targetId', targetId))
        .toString();
  }
}

class DeploymentPreviewResponseBuilder
    implements
        Builder<DeploymentPreviewResponse, DeploymentPreviewResponseBuilder> {
  _$DeploymentPreviewResponse? _$v;

  String? _applicationId;
  String? get applicationId => _$this._applicationId;
  set applicationId(String? applicationId) =>
      _$this._applicationId = applicationId;

  String? _applicationName;
  String? get applicationName => _$this._applicationName;
  set applicationName(String? applicationName) =>
      _$this._applicationName = applicationName;

  String? _deploymentBranch;
  String? get deploymentBranch => _$this._deploymentBranch;
  set deploymentBranch(String? deploymentBranch) =>
      _$this._deploymentBranch = deploymentBranch;

  String? _environment;
  String? get environment => _$this._environment;
  set environment(String? environment) => _$this._environment = environment;

  String? _executionMode;
  String? get executionMode => _$this._executionMode;
  set executionMode(String? executionMode) =>
      _$this._executionMode = executionMode;

  JsonObject? _imageSpec;
  JsonObject? get imageSpec => _$this._imageSpec;
  set imageSpec(JsonObject? imageSpec) => _$this._imageSpec = imageSpec;

  ListBuilder<String>? _modules;
  ListBuilder<String> get modules => _$this._modules ??= ListBuilder<String>();
  set modules(ListBuilder<String>? modules) => _$this._modules = modules;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  String? _nodeName;
  String? get nodeName => _$this._nodeName;
  set nodeName(String? nodeName) => _$this._nodeName = nodeName;

  JsonObject? _parameters;
  JsonObject? get parameters => _$this._parameters;
  set parameters(JsonObject? parameters) => _$this._parameters = parameters;

  String? _previewExpiresAt;
  String? get previewExpiresAt => _$this._previewExpiresAt;
  set previewExpiresAt(String? previewExpiresAt) =>
      _$this._previewExpiresAt = previewExpiresAt;

  String? _releaseStrategy;
  String? get releaseStrategy => _$this._releaseStrategy;
  set releaseStrategy(String? releaseStrategy) =>
      _$this._releaseStrategy = releaseStrategy;

  String? _releaseVersion;
  String? get releaseVersion => _$this._releaseVersion;
  set releaseVersion(String? releaseVersion) =>
      _$this._releaseVersion = releaseVersion;

  String? _resolvedCommitSha;
  String? get resolvedCommitSha => _$this._resolvedCommitSha;
  set resolvedCommitSha(String? resolvedCommitSha) =>
      _$this._resolvedCommitSha = resolvedCommitSha;

  String? _scriptPath;
  String? get scriptPath => _$this._scriptPath;
  set scriptPath(String? scriptPath) => _$this._scriptPath = scriptPath;

  String? _snapshotHash;
  String? get snapshotHash => _$this._snapshotHash;
  set snapshotHash(String? snapshotHash) => _$this._snapshotHash = snapshotHash;

  String? _sourcePolicy;
  String? get sourcePolicy => _$this._sourcePolicy;
  set sourcePolicy(String? sourcePolicy) => _$this._sourcePolicy = sourcePolicy;

  String? _targetCode;
  String? get targetCode => _$this._targetCode;
  set targetCode(String? targetCode) => _$this._targetCode = targetCode;

  String? _targetId;
  String? get targetId => _$this._targetId;
  set targetId(String? targetId) => _$this._targetId = targetId;

  DeploymentPreviewResponseBuilder() {
    DeploymentPreviewResponse._defaults(this);
  }

  DeploymentPreviewResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationId = $v.applicationId;
      _applicationName = $v.applicationName;
      _deploymentBranch = $v.deploymentBranch;
      _environment = $v.environment;
      _executionMode = $v.executionMode;
      _imageSpec = $v.imageSpec;
      _modules = $v.modules?.toBuilder();
      _nodeId = $v.nodeId;
      _nodeName = $v.nodeName;
      _parameters = $v.parameters;
      _previewExpiresAt = $v.previewExpiresAt;
      _releaseStrategy = $v.releaseStrategy;
      _releaseVersion = $v.releaseVersion;
      _resolvedCommitSha = $v.resolvedCommitSha;
      _scriptPath = $v.scriptPath;
      _snapshotHash = $v.snapshotHash;
      _sourcePolicy = $v.sourcePolicy;
      _targetCode = $v.targetCode;
      _targetId = $v.targetId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeploymentPreviewResponse other) {
    _$v = other as _$DeploymentPreviewResponse;
  }

  @override
  void update(void Function(DeploymentPreviewResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  DeploymentPreviewResponse build() => _build();

  _$DeploymentPreviewResponse _build() {
    _$DeploymentPreviewResponse _$result;
    try {
      _$result =
          _$v ??
          _$DeploymentPreviewResponse._(
            applicationId: BuiltValueNullFieldError.checkNotNull(
              applicationId,
              r'DeploymentPreviewResponse',
              'applicationId',
            ),
            applicationName: BuiltValueNullFieldError.checkNotNull(
              applicationName,
              r'DeploymentPreviewResponse',
              'applicationName',
            ),
            deploymentBranch: deploymentBranch,
            environment: BuiltValueNullFieldError.checkNotNull(
              environment,
              r'DeploymentPreviewResponse',
              'environment',
            ),
            executionMode: BuiltValueNullFieldError.checkNotNull(
              executionMode,
              r'DeploymentPreviewResponse',
              'executionMode',
            ),
            imageSpec: imageSpec,
            modules: _modules?.build(),
            nodeId: BuiltValueNullFieldError.checkNotNull(
              nodeId,
              r'DeploymentPreviewResponse',
              'nodeId',
            ),
            nodeName: BuiltValueNullFieldError.checkNotNull(
              nodeName,
              r'DeploymentPreviewResponse',
              'nodeName',
            ),
            parameters: parameters,
            previewExpiresAt: previewExpiresAt,
            releaseStrategy: BuiltValueNullFieldError.checkNotNull(
              releaseStrategy,
              r'DeploymentPreviewResponse',
              'releaseStrategy',
            ),
            releaseVersion: releaseVersion,
            resolvedCommitSha: resolvedCommitSha,
            scriptPath: BuiltValueNullFieldError.checkNotNull(
              scriptPath,
              r'DeploymentPreviewResponse',
              'scriptPath',
            ),
            snapshotHash: BuiltValueNullFieldError.checkNotNull(
              snapshotHash,
              r'DeploymentPreviewResponse',
              'snapshotHash',
            ),
            sourcePolicy: sourcePolicy,
            targetCode: BuiltValueNullFieldError.checkNotNull(
              targetCode,
              r'DeploymentPreviewResponse',
              'targetCode',
            ),
            targetId: BuiltValueNullFieldError.checkNotNull(
              targetId,
              r'DeploymentPreviewResponse',
              'targetId',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'modules';
        _modules?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'DeploymentPreviewResponse',
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
