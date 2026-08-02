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
  final String environment;
  @override
  final String nodeId;
  @override
  final String nodeName;
  @override
  final JsonObject? parameters;
  @override
  final String scriptPath;
  @override
  final String snapshotHash;
  @override
  final String targetId;

  factory _$DeploymentPreviewResponse(
          [void Function(DeploymentPreviewResponseBuilder)? updates]) =>
      (DeploymentPreviewResponseBuilder()..update(updates))._build();

  _$DeploymentPreviewResponse._(
      {required this.applicationId,
      required this.applicationName,
      required this.environment,
      required this.nodeId,
      required this.nodeName,
      this.parameters,
      required this.scriptPath,
      required this.snapshotHash,
      required this.targetId})
      : super._();
  @override
  DeploymentPreviewResponse rebuild(
          void Function(DeploymentPreviewResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  DeploymentPreviewResponseBuilder toBuilder() =>
      DeploymentPreviewResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeploymentPreviewResponse &&
        applicationId == other.applicationId &&
        applicationName == other.applicationName &&
        environment == other.environment &&
        nodeId == other.nodeId &&
        nodeName == other.nodeName &&
        parameters == other.parameters &&
        scriptPath == other.scriptPath &&
        snapshotHash == other.snapshotHash &&
        targetId == other.targetId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationId.hashCode);
    _$hash = $jc(_$hash, applicationName.hashCode);
    _$hash = $jc(_$hash, environment.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, nodeName.hashCode);
    _$hash = $jc(_$hash, parameters.hashCode);
    _$hash = $jc(_$hash, scriptPath.hashCode);
    _$hash = $jc(_$hash, snapshotHash.hashCode);
    _$hash = $jc(_$hash, targetId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'DeploymentPreviewResponse')
          ..add('applicationId', applicationId)
          ..add('applicationName', applicationName)
          ..add('environment', environment)
          ..add('nodeId', nodeId)
          ..add('nodeName', nodeName)
          ..add('parameters', parameters)
          ..add('scriptPath', scriptPath)
          ..add('snapshotHash', snapshotHash)
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

  String? _environment;
  String? get environment => _$this._environment;
  set environment(String? environment) => _$this._environment = environment;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  String? _nodeName;
  String? get nodeName => _$this._nodeName;
  set nodeName(String? nodeName) => _$this._nodeName = nodeName;

  JsonObject? _parameters;
  JsonObject? get parameters => _$this._parameters;
  set parameters(JsonObject? parameters) => _$this._parameters = parameters;

  String? _scriptPath;
  String? get scriptPath => _$this._scriptPath;
  set scriptPath(String? scriptPath) => _$this._scriptPath = scriptPath;

  String? _snapshotHash;
  String? get snapshotHash => _$this._snapshotHash;
  set snapshotHash(String? snapshotHash) => _$this._snapshotHash = snapshotHash;

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
      _environment = $v.environment;
      _nodeId = $v.nodeId;
      _nodeName = $v.nodeName;
      _parameters = $v.parameters;
      _scriptPath = $v.scriptPath;
      _snapshotHash = $v.snapshotHash;
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
    final _$result = _$v ??
        _$DeploymentPreviewResponse._(
          applicationId: BuiltValueNullFieldError.checkNotNull(
              applicationId, r'DeploymentPreviewResponse', 'applicationId'),
          applicationName: BuiltValueNullFieldError.checkNotNull(
              applicationName, r'DeploymentPreviewResponse', 'applicationName'),
          environment: BuiltValueNullFieldError.checkNotNull(
              environment, r'DeploymentPreviewResponse', 'environment'),
          nodeId: BuiltValueNullFieldError.checkNotNull(
              nodeId, r'DeploymentPreviewResponse', 'nodeId'),
          nodeName: BuiltValueNullFieldError.checkNotNull(
              nodeName, r'DeploymentPreviewResponse', 'nodeName'),
          parameters: parameters,
          scriptPath: BuiltValueNullFieldError.checkNotNull(
              scriptPath, r'DeploymentPreviewResponse', 'scriptPath'),
          snapshotHash: BuiltValueNullFieldError.checkNotNull(
              snapshotHash, r'DeploymentPreviewResponse', 'snapshotHash'),
          targetId: BuiltValueNullFieldError.checkNotNull(
              targetId, r'DeploymentPreviewResponse', 'targetId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
