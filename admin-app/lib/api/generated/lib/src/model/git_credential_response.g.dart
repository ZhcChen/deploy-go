// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'git_credential_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$GitCredentialResponse extends GitCredentialResponse {
  @override
  final String algorithm;
  @override
  final String createdAt;
  @override
  final String fingerprint;
  @override
  final String id;
  @override
  final String name;
  @override
  final String publicKey;
  @override
  final String status;
  @override
  final String updatedAt;
  @override
  final int version;

  factory _$GitCredentialResponse([
    void Function(GitCredentialResponseBuilder)? updates,
  ]) => (GitCredentialResponseBuilder()..update(updates))._build();

  _$GitCredentialResponse._({
    required this.algorithm,
    required this.createdAt,
    required this.fingerprint,
    required this.id,
    required this.name,
    required this.publicKey,
    required this.status,
    required this.updatedAt,
    required this.version,
  }) : super._();
  @override
  GitCredentialResponse rebuild(
    void Function(GitCredentialResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  GitCredentialResponseBuilder toBuilder() =>
      GitCredentialResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is GitCredentialResponse &&
        algorithm == other.algorithm &&
        createdAt == other.createdAt &&
        fingerprint == other.fingerprint &&
        id == other.id &&
        name == other.name &&
        publicKey == other.publicKey &&
        status == other.status &&
        updatedAt == other.updatedAt &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, algorithm.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, fingerprint.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, publicKey.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'GitCredentialResponse')
          ..add('algorithm', algorithm)
          ..add('createdAt', createdAt)
          ..add('fingerprint', fingerprint)
          ..add('id', id)
          ..add('name', name)
          ..add('publicKey', publicKey)
          ..add('status', status)
          ..add('updatedAt', updatedAt)
          ..add('version', version))
        .toString();
  }
}

class GitCredentialResponseBuilder
    implements Builder<GitCredentialResponse, GitCredentialResponseBuilder> {
  _$GitCredentialResponse? _$v;

  String? _algorithm;
  String? get algorithm => _$this._algorithm;
  set algorithm(String? algorithm) => _$this._algorithm = algorithm;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _fingerprint;
  String? get fingerprint => _$this._fingerprint;
  set fingerprint(String? fingerprint) => _$this._fingerprint = fingerprint;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _publicKey;
  String? get publicKey => _$this._publicKey;
  set publicKey(String? publicKey) => _$this._publicKey = publicKey;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  GitCredentialResponseBuilder() {
    GitCredentialResponse._defaults(this);
  }

  GitCredentialResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _algorithm = $v.algorithm;
      _createdAt = $v.createdAt;
      _fingerprint = $v.fingerprint;
      _id = $v.id;
      _name = $v.name;
      _publicKey = $v.publicKey;
      _status = $v.status;
      _updatedAt = $v.updatedAt;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(GitCredentialResponse other) {
    _$v = other as _$GitCredentialResponse;
  }

  @override
  void update(void Function(GitCredentialResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  GitCredentialResponse build() => _build();

  _$GitCredentialResponse _build() {
    final _$result =
        _$v ??
        _$GitCredentialResponse._(
          algorithm: BuiltValueNullFieldError.checkNotNull(
            algorithm,
            r'GitCredentialResponse',
            'algorithm',
          ),
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'GitCredentialResponse',
            'createdAt',
          ),
          fingerprint: BuiltValueNullFieldError.checkNotNull(
            fingerprint,
            r'GitCredentialResponse',
            'fingerprint',
          ),
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'GitCredentialResponse',
            'id',
          ),
          name: BuiltValueNullFieldError.checkNotNull(
            name,
            r'GitCredentialResponse',
            'name',
          ),
          publicKey: BuiltValueNullFieldError.checkNotNull(
            publicKey,
            r'GitCredentialResponse',
            'publicKey',
          ),
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'GitCredentialResponse',
            'status',
          ),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
            updatedAt,
            r'GitCredentialResponse',
            'updatedAt',
          ),
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'GitCredentialResponse',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
