// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'workspace_source_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$WorkspaceSourceResponse extends WorkspaceSourceResponse {
  @override
  final String applicationId;
  @override
  final String buildAgentId;
  @override
  final String? buildAgentName;
  @override
  final String createdAt;
  @override
  final String? createdBy;
  @override
  final String id;
  @override
  final String status;
  @override
  final String updatedAt;
  @override
  final int version;
  @override
  final String workspacePath;
  @override
  final int workspaceVersion;

  factory _$WorkspaceSourceResponse([
    void Function(WorkspaceSourceResponseBuilder)? updates,
  ]) => (WorkspaceSourceResponseBuilder()..update(updates))._build();

  _$WorkspaceSourceResponse._({
    required this.applicationId,
    required this.buildAgentId,
    this.buildAgentName,
    required this.createdAt,
    this.createdBy,
    required this.id,
    required this.status,
    required this.updatedAt,
    required this.version,
    required this.workspacePath,
    required this.workspaceVersion,
  }) : super._();
  @override
  WorkspaceSourceResponse rebuild(
    void Function(WorkspaceSourceResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  WorkspaceSourceResponseBuilder toBuilder() =>
      WorkspaceSourceResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is WorkspaceSourceResponse &&
        applicationId == other.applicationId &&
        buildAgentId == other.buildAgentId &&
        buildAgentName == other.buildAgentName &&
        createdAt == other.createdAt &&
        createdBy == other.createdBy &&
        id == other.id &&
        status == other.status &&
        updatedAt == other.updatedAt &&
        version == other.version &&
        workspacePath == other.workspacePath &&
        workspaceVersion == other.workspaceVersion;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationId.hashCode);
    _$hash = $jc(_$hash, buildAgentId.hashCode);
    _$hash = $jc(_$hash, buildAgentName.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, createdBy.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jc(_$hash, workspacePath.hashCode);
    _$hash = $jc(_$hash, workspaceVersion.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'WorkspaceSourceResponse')
          ..add('applicationId', applicationId)
          ..add('buildAgentId', buildAgentId)
          ..add('buildAgentName', buildAgentName)
          ..add('createdAt', createdAt)
          ..add('createdBy', createdBy)
          ..add('id', id)
          ..add('status', status)
          ..add('updatedAt', updatedAt)
          ..add('version', version)
          ..add('workspacePath', workspacePath)
          ..add('workspaceVersion', workspaceVersion))
        .toString();
  }
}

class WorkspaceSourceResponseBuilder
    implements
        Builder<WorkspaceSourceResponse, WorkspaceSourceResponseBuilder> {
  _$WorkspaceSourceResponse? _$v;

  String? _applicationId;
  String? get applicationId => _$this._applicationId;
  set applicationId(String? applicationId) =>
      _$this._applicationId = applicationId;

  String? _buildAgentId;
  String? get buildAgentId => _$this._buildAgentId;
  set buildAgentId(String? buildAgentId) => _$this._buildAgentId = buildAgentId;

  String? _buildAgentName;
  String? get buildAgentName => _$this._buildAgentName;
  set buildAgentName(String? buildAgentName) =>
      _$this._buildAgentName = buildAgentName;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _createdBy;
  String? get createdBy => _$this._createdBy;
  set createdBy(String? createdBy) => _$this._createdBy = createdBy;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  String? _workspacePath;
  String? get workspacePath => _$this._workspacePath;
  set workspacePath(String? workspacePath) =>
      _$this._workspacePath = workspacePath;

  int? _workspaceVersion;
  int? get workspaceVersion => _$this._workspaceVersion;
  set workspaceVersion(int? workspaceVersion) =>
      _$this._workspaceVersion = workspaceVersion;

  WorkspaceSourceResponseBuilder() {
    WorkspaceSourceResponse._defaults(this);
  }

  WorkspaceSourceResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationId = $v.applicationId;
      _buildAgentId = $v.buildAgentId;
      _buildAgentName = $v.buildAgentName;
      _createdAt = $v.createdAt;
      _createdBy = $v.createdBy;
      _id = $v.id;
      _status = $v.status;
      _updatedAt = $v.updatedAt;
      _version = $v.version;
      _workspacePath = $v.workspacePath;
      _workspaceVersion = $v.workspaceVersion;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(WorkspaceSourceResponse other) {
    _$v = other as _$WorkspaceSourceResponse;
  }

  @override
  void update(void Function(WorkspaceSourceResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  WorkspaceSourceResponse build() => _build();

  _$WorkspaceSourceResponse _build() {
    final _$result =
        _$v ??
        _$WorkspaceSourceResponse._(
          applicationId: BuiltValueNullFieldError.checkNotNull(
            applicationId,
            r'WorkspaceSourceResponse',
            'applicationId',
          ),
          buildAgentId: BuiltValueNullFieldError.checkNotNull(
            buildAgentId,
            r'WorkspaceSourceResponse',
            'buildAgentId',
          ),
          buildAgentName: buildAgentName,
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'WorkspaceSourceResponse',
            'createdAt',
          ),
          createdBy: createdBy,
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'WorkspaceSourceResponse',
            'id',
          ),
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'WorkspaceSourceResponse',
            'status',
          ),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
            updatedAt,
            r'WorkspaceSourceResponse',
            'updatedAt',
          ),
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'WorkspaceSourceResponse',
            'version',
          ),
          workspacePath: BuiltValueNullFieldError.checkNotNull(
            workspacePath,
            r'WorkspaceSourceResponse',
            'workspacePath',
          ),
          workspaceVersion: BuiltValueNullFieldError.checkNotNull(
            workspaceVersion,
            r'WorkspaceSourceResponse',
            'workspaceVersion',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
