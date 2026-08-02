// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'ssh_credential_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SshCredentialResponse extends SshCredentialResponse {
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
  final String updatedAt;
  @override
  final int version;

  factory _$SshCredentialResponse([
    void Function(SshCredentialResponseBuilder)? updates,
  ]) => (SshCredentialResponseBuilder()..update(updates))._build();

  _$SshCredentialResponse._({
    required this.algorithm,
    required this.createdAt,
    required this.fingerprint,
    required this.id,
    required this.name,
    required this.publicKey,
    required this.updatedAt,
    required this.version,
  }) : super._();
  @override
  SshCredentialResponse rebuild(
    void Function(SshCredentialResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  SshCredentialResponseBuilder toBuilder() =>
      SshCredentialResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SshCredentialResponse &&
        algorithm == other.algorithm &&
        createdAt == other.createdAt &&
        fingerprint == other.fingerprint &&
        id == other.id &&
        name == other.name &&
        publicKey == other.publicKey &&
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
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SshCredentialResponse')
          ..add('algorithm', algorithm)
          ..add('createdAt', createdAt)
          ..add('fingerprint', fingerprint)
          ..add('id', id)
          ..add('name', name)
          ..add('publicKey', publicKey)
          ..add('updatedAt', updatedAt)
          ..add('version', version))
        .toString();
  }
}

class SshCredentialResponseBuilder
    implements Builder<SshCredentialResponse, SshCredentialResponseBuilder> {
  _$SshCredentialResponse? _$v;

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

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  SshCredentialResponseBuilder() {
    SshCredentialResponse._defaults(this);
  }

  SshCredentialResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _algorithm = $v.algorithm;
      _createdAt = $v.createdAt;
      _fingerprint = $v.fingerprint;
      _id = $v.id;
      _name = $v.name;
      _publicKey = $v.publicKey;
      _updatedAt = $v.updatedAt;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SshCredentialResponse other) {
    _$v = other as _$SshCredentialResponse;
  }

  @override
  void update(void Function(SshCredentialResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SshCredentialResponse build() => _build();

  _$SshCredentialResponse _build() {
    final _$result =
        _$v ??
        _$SshCredentialResponse._(
          algorithm: BuiltValueNullFieldError.checkNotNull(
            algorithm,
            r'SshCredentialResponse',
            'algorithm',
          ),
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'SshCredentialResponse',
            'createdAt',
          ),
          fingerprint: BuiltValueNullFieldError.checkNotNull(
            fingerprint,
            r'SshCredentialResponse',
            'fingerprint',
          ),
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'SshCredentialResponse',
            'id',
          ),
          name: BuiltValueNullFieldError.checkNotNull(
            name,
            r'SshCredentialResponse',
            'name',
          ),
          publicKey: BuiltValueNullFieldError.checkNotNull(
            publicKey,
            r'SshCredentialResponse',
            'publicKey',
          ),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
            updatedAt,
            r'SshCredentialResponse',
            'updatedAt',
          ),
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'SshCredentialResponse',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
