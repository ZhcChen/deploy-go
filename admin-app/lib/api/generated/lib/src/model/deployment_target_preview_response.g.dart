// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'deployment_target_preview_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeploymentTargetPreviewResponse
    extends DeploymentTargetPreviewResponse {
  @override
  final String agentId;
  @override
  final bool agentOnline;
  @override
  final String envGateStatus;
  @override
  final String nodeId;
  @override
  final String nodeName;
  @override
  final String scriptPath;
  @override
  final String targetId;

  factory _$DeploymentTargetPreviewResponse([
    void Function(DeploymentTargetPreviewResponseBuilder)? updates,
  ]) => (DeploymentTargetPreviewResponseBuilder()..update(updates))._build();

  _$DeploymentTargetPreviewResponse._({
    required this.agentId,
    required this.agentOnline,
    required this.envGateStatus,
    required this.nodeId,
    required this.nodeName,
    required this.scriptPath,
    required this.targetId,
  }) : super._();
  @override
  DeploymentTargetPreviewResponse rebuild(
    void Function(DeploymentTargetPreviewResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  DeploymentTargetPreviewResponseBuilder toBuilder() =>
      DeploymentTargetPreviewResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeploymentTargetPreviewResponse &&
        agentId == other.agentId &&
        agentOnline == other.agentOnline &&
        envGateStatus == other.envGateStatus &&
        nodeId == other.nodeId &&
        nodeName == other.nodeName &&
        scriptPath == other.scriptPath &&
        targetId == other.targetId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, agentId.hashCode);
    _$hash = $jc(_$hash, agentOnline.hashCode);
    _$hash = $jc(_$hash, envGateStatus.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, nodeName.hashCode);
    _$hash = $jc(_$hash, scriptPath.hashCode);
    _$hash = $jc(_$hash, targetId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'DeploymentTargetPreviewResponse')
          ..add('agentId', agentId)
          ..add('agentOnline', agentOnline)
          ..add('envGateStatus', envGateStatus)
          ..add('nodeId', nodeId)
          ..add('nodeName', nodeName)
          ..add('scriptPath', scriptPath)
          ..add('targetId', targetId))
        .toString();
  }
}

class DeploymentTargetPreviewResponseBuilder
    implements
        Builder<
          DeploymentTargetPreviewResponse,
          DeploymentTargetPreviewResponseBuilder
        > {
  _$DeploymentTargetPreviewResponse? _$v;

  String? _agentId;
  String? get agentId => _$this._agentId;
  set agentId(String? agentId) => _$this._agentId = agentId;

  bool? _agentOnline;
  bool? get agentOnline => _$this._agentOnline;
  set agentOnline(bool? agentOnline) => _$this._agentOnline = agentOnline;

  String? _envGateStatus;
  String? get envGateStatus => _$this._envGateStatus;
  set envGateStatus(String? envGateStatus) =>
      _$this._envGateStatus = envGateStatus;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  String? _nodeName;
  String? get nodeName => _$this._nodeName;
  set nodeName(String? nodeName) => _$this._nodeName = nodeName;

  String? _scriptPath;
  String? get scriptPath => _$this._scriptPath;
  set scriptPath(String? scriptPath) => _$this._scriptPath = scriptPath;

  String? _targetId;
  String? get targetId => _$this._targetId;
  set targetId(String? targetId) => _$this._targetId = targetId;

  DeploymentTargetPreviewResponseBuilder() {
    DeploymentTargetPreviewResponse._defaults(this);
  }

  DeploymentTargetPreviewResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _agentId = $v.agentId;
      _agentOnline = $v.agentOnline;
      _envGateStatus = $v.envGateStatus;
      _nodeId = $v.nodeId;
      _nodeName = $v.nodeName;
      _scriptPath = $v.scriptPath;
      _targetId = $v.targetId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeploymentTargetPreviewResponse other) {
    _$v = other as _$DeploymentTargetPreviewResponse;
  }

  @override
  void update(void Function(DeploymentTargetPreviewResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  DeploymentTargetPreviewResponse build() => _build();

  _$DeploymentTargetPreviewResponse _build() {
    final _$result =
        _$v ??
        _$DeploymentTargetPreviewResponse._(
          agentId: BuiltValueNullFieldError.checkNotNull(
            agentId,
            r'DeploymentTargetPreviewResponse',
            'agentId',
          ),
          agentOnline: BuiltValueNullFieldError.checkNotNull(
            agentOnline,
            r'DeploymentTargetPreviewResponse',
            'agentOnline',
          ),
          envGateStatus: BuiltValueNullFieldError.checkNotNull(
            envGateStatus,
            r'DeploymentTargetPreviewResponse',
            'envGateStatus',
          ),
          nodeId: BuiltValueNullFieldError.checkNotNull(
            nodeId,
            r'DeploymentTargetPreviewResponse',
            'nodeId',
          ),
          nodeName: BuiltValueNullFieldError.checkNotNull(
            nodeName,
            r'DeploymentTargetPreviewResponse',
            'nodeName',
          ),
          scriptPath: BuiltValueNullFieldError.checkNotNull(
            scriptPath,
            r'DeploymentTargetPreviewResponse',
            'scriptPath',
          ),
          targetId: BuiltValueNullFieldError.checkNotNull(
            targetId,
            r'DeploymentTargetPreviewResponse',
            'targetId',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
