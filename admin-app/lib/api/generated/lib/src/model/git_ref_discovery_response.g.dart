// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'git_ref_discovery_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$GitRefDiscoveryResponse extends GitRefDiscoveryResponse {
  @override
  final String applicationSourceId;
  @override
  final String createdAt;
  @override
  final String? errorCode;
  @override
  final String? expiresAt;
  @override
  final String? finishedAt;
  @override
  final String id;
  @override
  final BuiltList<GitRefResponse> refs;
  @override
  final int sourceVersion;
  @override
  final String status;
  @override
  final String taskId;

  factory _$GitRefDiscoveryResponse([
    void Function(GitRefDiscoveryResponseBuilder)? updates,
  ]) => (GitRefDiscoveryResponseBuilder()..update(updates))._build();

  _$GitRefDiscoveryResponse._({
    required this.applicationSourceId,
    required this.createdAt,
    this.errorCode,
    this.expiresAt,
    this.finishedAt,
    required this.id,
    required this.refs,
    required this.sourceVersion,
    required this.status,
    required this.taskId,
  }) : super._();
  @override
  GitRefDiscoveryResponse rebuild(
    void Function(GitRefDiscoveryResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  GitRefDiscoveryResponseBuilder toBuilder() =>
      GitRefDiscoveryResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is GitRefDiscoveryResponse &&
        applicationSourceId == other.applicationSourceId &&
        createdAt == other.createdAt &&
        errorCode == other.errorCode &&
        expiresAt == other.expiresAt &&
        finishedAt == other.finishedAt &&
        id == other.id &&
        refs == other.refs &&
        sourceVersion == other.sourceVersion &&
        status == other.status &&
        taskId == other.taskId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationSourceId.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, errorCode.hashCode);
    _$hash = $jc(_$hash, expiresAt.hashCode);
    _$hash = $jc(_$hash, finishedAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, refs.hashCode);
    _$hash = $jc(_$hash, sourceVersion.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, taskId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'GitRefDiscoveryResponse')
          ..add('applicationSourceId', applicationSourceId)
          ..add('createdAt', createdAt)
          ..add('errorCode', errorCode)
          ..add('expiresAt', expiresAt)
          ..add('finishedAt', finishedAt)
          ..add('id', id)
          ..add('refs', refs)
          ..add('sourceVersion', sourceVersion)
          ..add('status', status)
          ..add('taskId', taskId))
        .toString();
  }
}

class GitRefDiscoveryResponseBuilder
    implements
        Builder<GitRefDiscoveryResponse, GitRefDiscoveryResponseBuilder> {
  _$GitRefDiscoveryResponse? _$v;

  String? _applicationSourceId;
  String? get applicationSourceId => _$this._applicationSourceId;
  set applicationSourceId(String? applicationSourceId) =>
      _$this._applicationSourceId = applicationSourceId;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _errorCode;
  String? get errorCode => _$this._errorCode;
  set errorCode(String? errorCode) => _$this._errorCode = errorCode;

  String? _expiresAt;
  String? get expiresAt => _$this._expiresAt;
  set expiresAt(String? expiresAt) => _$this._expiresAt = expiresAt;

  String? _finishedAt;
  String? get finishedAt => _$this._finishedAt;
  set finishedAt(String? finishedAt) => _$this._finishedAt = finishedAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  ListBuilder<GitRefResponse>? _refs;
  ListBuilder<GitRefResponse> get refs =>
      _$this._refs ??= ListBuilder<GitRefResponse>();
  set refs(ListBuilder<GitRefResponse>? refs) => _$this._refs = refs;

  int? _sourceVersion;
  int? get sourceVersion => _$this._sourceVersion;
  set sourceVersion(int? sourceVersion) =>
      _$this._sourceVersion = sourceVersion;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _taskId;
  String? get taskId => _$this._taskId;
  set taskId(String? taskId) => _$this._taskId = taskId;

  GitRefDiscoveryResponseBuilder() {
    GitRefDiscoveryResponse._defaults(this);
  }

  GitRefDiscoveryResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationSourceId = $v.applicationSourceId;
      _createdAt = $v.createdAt;
      _errorCode = $v.errorCode;
      _expiresAt = $v.expiresAt;
      _finishedAt = $v.finishedAt;
      _id = $v.id;
      _refs = $v.refs.toBuilder();
      _sourceVersion = $v.sourceVersion;
      _status = $v.status;
      _taskId = $v.taskId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(GitRefDiscoveryResponse other) {
    _$v = other as _$GitRefDiscoveryResponse;
  }

  @override
  void update(void Function(GitRefDiscoveryResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  GitRefDiscoveryResponse build() => _build();

  _$GitRefDiscoveryResponse _build() {
    _$GitRefDiscoveryResponse _$result;
    try {
      _$result =
          _$v ??
          _$GitRefDiscoveryResponse._(
            applicationSourceId: BuiltValueNullFieldError.checkNotNull(
              applicationSourceId,
              r'GitRefDiscoveryResponse',
              'applicationSourceId',
            ),
            createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt,
              r'GitRefDiscoveryResponse',
              'createdAt',
            ),
            errorCode: errorCode,
            expiresAt: expiresAt,
            finishedAt: finishedAt,
            id: BuiltValueNullFieldError.checkNotNull(
              id,
              r'GitRefDiscoveryResponse',
              'id',
            ),
            refs: refs.build(),
            sourceVersion: BuiltValueNullFieldError.checkNotNull(
              sourceVersion,
              r'GitRefDiscoveryResponse',
              'sourceVersion',
            ),
            status: BuiltValueNullFieldError.checkNotNull(
              status,
              r'GitRefDiscoveryResponse',
              'status',
            ),
            taskId: BuiltValueNullFieldError.checkNotNull(
              taskId,
              r'GitRefDiscoveryResponse',
              'taskId',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'refs';
        refs.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'GitRefDiscoveryResponse',
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
