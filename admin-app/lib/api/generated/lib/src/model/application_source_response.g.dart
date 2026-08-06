// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_source_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationSourceResponse extends ApplicationSourceResponse {
  @override
  final String applicationId;
  @override
  final String? branchVerifiedAt;
  @override
  final String buildAgentId;
  @override
  final String? buildAgentName;
  @override
  final String createdAt;
  @override
  final String? deploymentBranch;
  @override
  final String? gitCredentialId;
  @override
  final String? gitCredentialName;
  @override
  final String id;
  @override
  final String repositoryUrl;
  @override
  final String sourcePolicy;
  @override
  final String status;
  @override
  final String updatedAt;
  @override
  final int version;

  factory _$ApplicationSourceResponse([
    void Function(ApplicationSourceResponseBuilder)? updates,
  ]) => (ApplicationSourceResponseBuilder()..update(updates))._build();

  _$ApplicationSourceResponse._({
    required this.applicationId,
    this.branchVerifiedAt,
    required this.buildAgentId,
    this.buildAgentName,
    required this.createdAt,
    this.deploymentBranch,
    this.gitCredentialId,
    this.gitCredentialName,
    required this.id,
    required this.repositoryUrl,
    required this.sourcePolicy,
    required this.status,
    required this.updatedAt,
    required this.version,
  }) : super._();
  @override
  ApplicationSourceResponse rebuild(
    void Function(ApplicationSourceResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationSourceResponseBuilder toBuilder() =>
      ApplicationSourceResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationSourceResponse &&
        applicationId == other.applicationId &&
        branchVerifiedAt == other.branchVerifiedAt &&
        buildAgentId == other.buildAgentId &&
        buildAgentName == other.buildAgentName &&
        createdAt == other.createdAt &&
        deploymentBranch == other.deploymentBranch &&
        gitCredentialId == other.gitCredentialId &&
        gitCredentialName == other.gitCredentialName &&
        id == other.id &&
        repositoryUrl == other.repositoryUrl &&
        sourcePolicy == other.sourcePolicy &&
        status == other.status &&
        updatedAt == other.updatedAt &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationId.hashCode);
    _$hash = $jc(_$hash, branchVerifiedAt.hashCode);
    _$hash = $jc(_$hash, buildAgentId.hashCode);
    _$hash = $jc(_$hash, buildAgentName.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, deploymentBranch.hashCode);
    _$hash = $jc(_$hash, gitCredentialId.hashCode);
    _$hash = $jc(_$hash, gitCredentialName.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, repositoryUrl.hashCode);
    _$hash = $jc(_$hash, sourcePolicy.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApplicationSourceResponse')
          ..add('applicationId', applicationId)
          ..add('branchVerifiedAt', branchVerifiedAt)
          ..add('buildAgentId', buildAgentId)
          ..add('buildAgentName', buildAgentName)
          ..add('createdAt', createdAt)
          ..add('deploymentBranch', deploymentBranch)
          ..add('gitCredentialId', gitCredentialId)
          ..add('gitCredentialName', gitCredentialName)
          ..add('id', id)
          ..add('repositoryUrl', repositoryUrl)
          ..add('sourcePolicy', sourcePolicy)
          ..add('status', status)
          ..add('updatedAt', updatedAt)
          ..add('version', version))
        .toString();
  }
}

class ApplicationSourceResponseBuilder
    implements
        Builder<ApplicationSourceResponse, ApplicationSourceResponseBuilder> {
  _$ApplicationSourceResponse? _$v;

  String? _applicationId;
  String? get applicationId => _$this._applicationId;
  set applicationId(String? applicationId) =>
      _$this._applicationId = applicationId;

  String? _branchVerifiedAt;
  String? get branchVerifiedAt => _$this._branchVerifiedAt;
  set branchVerifiedAt(String? branchVerifiedAt) =>
      _$this._branchVerifiedAt = branchVerifiedAt;

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

  String? _deploymentBranch;
  String? get deploymentBranch => _$this._deploymentBranch;
  set deploymentBranch(String? deploymentBranch) =>
      _$this._deploymentBranch = deploymentBranch;

  String? _gitCredentialId;
  String? get gitCredentialId => _$this._gitCredentialId;
  set gitCredentialId(String? gitCredentialId) =>
      _$this._gitCredentialId = gitCredentialId;

  String? _gitCredentialName;
  String? get gitCredentialName => _$this._gitCredentialName;
  set gitCredentialName(String? gitCredentialName) =>
      _$this._gitCredentialName = gitCredentialName;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _repositoryUrl;
  String? get repositoryUrl => _$this._repositoryUrl;
  set repositoryUrl(String? repositoryUrl) =>
      _$this._repositoryUrl = repositoryUrl;

  String? _sourcePolicy;
  String? get sourcePolicy => _$this._sourcePolicy;
  set sourcePolicy(String? sourcePolicy) => _$this._sourcePolicy = sourcePolicy;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  ApplicationSourceResponseBuilder() {
    ApplicationSourceResponse._defaults(this);
  }

  ApplicationSourceResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationId = $v.applicationId;
      _branchVerifiedAt = $v.branchVerifiedAt;
      _buildAgentId = $v.buildAgentId;
      _buildAgentName = $v.buildAgentName;
      _createdAt = $v.createdAt;
      _deploymentBranch = $v.deploymentBranch;
      _gitCredentialId = $v.gitCredentialId;
      _gitCredentialName = $v.gitCredentialName;
      _id = $v.id;
      _repositoryUrl = $v.repositoryUrl;
      _sourcePolicy = $v.sourcePolicy;
      _status = $v.status;
      _updatedAt = $v.updatedAt;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationSourceResponse other) {
    _$v = other as _$ApplicationSourceResponse;
  }

  @override
  void update(void Function(ApplicationSourceResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationSourceResponse build() => _build();

  _$ApplicationSourceResponse _build() {
    final _$result =
        _$v ??
        _$ApplicationSourceResponse._(
          applicationId: BuiltValueNullFieldError.checkNotNull(
            applicationId,
            r'ApplicationSourceResponse',
            'applicationId',
          ),
          branchVerifiedAt: branchVerifiedAt,
          buildAgentId: BuiltValueNullFieldError.checkNotNull(
            buildAgentId,
            r'ApplicationSourceResponse',
            'buildAgentId',
          ),
          buildAgentName: buildAgentName,
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'ApplicationSourceResponse',
            'createdAt',
          ),
          deploymentBranch: deploymentBranch,
          gitCredentialId: gitCredentialId,
          gitCredentialName: gitCredentialName,
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'ApplicationSourceResponse',
            'id',
          ),
          repositoryUrl: BuiltValueNullFieldError.checkNotNull(
            repositoryUrl,
            r'ApplicationSourceResponse',
            'repositoryUrl',
          ),
          sourcePolicy: BuiltValueNullFieldError.checkNotNull(
            sourcePolicy,
            r'ApplicationSourceResponse',
            'sourcePolicy',
          ),
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'ApplicationSourceResponse',
            'status',
          ),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
            updatedAt,
            r'ApplicationSourceResponse',
            'updatedAt',
          ),
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'ApplicationSourceResponse',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
