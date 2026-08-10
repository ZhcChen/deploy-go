// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'external_deployment_target.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ExternalDeploymentTarget extends ExternalDeploymentTarget {
  @override
  final String environment;
  @override
  final String executionMode;
  @override
  final String id;
  @override
  final String nodeId;
  @override
  final String nodeName;
  @override
  final bool privilegedRelease;
  @override
  final String status;

  factory _$ExternalDeploymentTarget([
    void Function(ExternalDeploymentTargetBuilder)? updates,
  ]) => (ExternalDeploymentTargetBuilder()..update(updates))._build();

  _$ExternalDeploymentTarget._({
    required this.environment,
    required this.executionMode,
    required this.id,
    required this.nodeId,
    required this.nodeName,
    required this.privilegedRelease,
    required this.status,
  }) : super._();
  @override
  ExternalDeploymentTarget rebuild(
    void Function(ExternalDeploymentTargetBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ExternalDeploymentTargetBuilder toBuilder() =>
      ExternalDeploymentTargetBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ExternalDeploymentTarget &&
        environment == other.environment &&
        executionMode == other.executionMode &&
        id == other.id &&
        nodeId == other.nodeId &&
        nodeName == other.nodeName &&
        privilegedRelease == other.privilegedRelease &&
        status == other.status;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, environment.hashCode);
    _$hash = $jc(_$hash, executionMode.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, nodeName.hashCode);
    _$hash = $jc(_$hash, privilegedRelease.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ExternalDeploymentTarget')
          ..add('environment', environment)
          ..add('executionMode', executionMode)
          ..add('id', id)
          ..add('nodeId', nodeId)
          ..add('nodeName', nodeName)
          ..add('privilegedRelease', privilegedRelease)
          ..add('status', status))
        .toString();
  }
}

class ExternalDeploymentTargetBuilder
    implements
        Builder<ExternalDeploymentTarget, ExternalDeploymentTargetBuilder> {
  _$ExternalDeploymentTarget? _$v;

  String? _environment;
  String? get environment => _$this._environment;
  set environment(String? environment) => _$this._environment = environment;

  String? _executionMode;
  String? get executionMode => _$this._executionMode;
  set executionMode(String? executionMode) =>
      _$this._executionMode = executionMode;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  String? _nodeName;
  String? get nodeName => _$this._nodeName;
  set nodeName(String? nodeName) => _$this._nodeName = nodeName;

  bool? _privilegedRelease;
  bool? get privilegedRelease => _$this._privilegedRelease;
  set privilegedRelease(bool? privilegedRelease) =>
      _$this._privilegedRelease = privilegedRelease;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  ExternalDeploymentTargetBuilder() {
    ExternalDeploymentTarget._defaults(this);
  }

  ExternalDeploymentTargetBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _environment = $v.environment;
      _executionMode = $v.executionMode;
      _id = $v.id;
      _nodeId = $v.nodeId;
      _nodeName = $v.nodeName;
      _privilegedRelease = $v.privilegedRelease;
      _status = $v.status;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ExternalDeploymentTarget other) {
    _$v = other as _$ExternalDeploymentTarget;
  }

  @override
  void update(void Function(ExternalDeploymentTargetBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ExternalDeploymentTarget build() => _build();

  _$ExternalDeploymentTarget _build() {
    final _$result =
        _$v ??
        _$ExternalDeploymentTarget._(
          environment: BuiltValueNullFieldError.checkNotNull(
            environment,
            r'ExternalDeploymentTarget',
            'environment',
          ),
          executionMode: BuiltValueNullFieldError.checkNotNull(
            executionMode,
            r'ExternalDeploymentTarget',
            'executionMode',
          ),
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'ExternalDeploymentTarget',
            'id',
          ),
          nodeId: BuiltValueNullFieldError.checkNotNull(
            nodeId,
            r'ExternalDeploymentTarget',
            'nodeId',
          ),
          nodeName: BuiltValueNullFieldError.checkNotNull(
            nodeName,
            r'ExternalDeploymentTarget',
            'nodeName',
          ),
          privilegedRelease: BuiltValueNullFieldError.checkNotNull(
            privilegedRelease,
            r'ExternalDeploymentTarget',
            'privilegedRelease',
          ),
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'ExternalDeploymentTarget',
            'status',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
