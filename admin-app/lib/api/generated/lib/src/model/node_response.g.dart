// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'node_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$NodeResponse extends NodeResponse {
  @override
  final String? checkedAt;
  @override
  final String createdAt;
  @override
  final String? host;
  @override
  final String id;
  @override
  final String name;
  @override
  final int? port;
  @override
  final String? secretsRoot;
  @override
  final String? sshCredentialId;
  @override
  final String status;
  @override
  final String? trustedHostFingerprint;
  @override
  final String updatedAt;
  @override
  final String? username;
  @override
  final int version;
  @override
  final String? workRoot;

  factory _$NodeResponse([void Function(NodeResponseBuilder)? updates]) =>
      (NodeResponseBuilder()..update(updates))._build();

  _$NodeResponse._({
    this.checkedAt,
    required this.createdAt,
    this.host,
    required this.id,
    required this.name,
    this.port,
    this.secretsRoot,
    this.sshCredentialId,
    required this.status,
    this.trustedHostFingerprint,
    required this.updatedAt,
    this.username,
    required this.version,
    this.workRoot,
  }) : super._();
  @override
  NodeResponse rebuild(void Function(NodeResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  NodeResponseBuilder toBuilder() => NodeResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is NodeResponse &&
        checkedAt == other.checkedAt &&
        createdAt == other.createdAt &&
        host == other.host &&
        id == other.id &&
        name == other.name &&
        port == other.port &&
        secretsRoot == other.secretsRoot &&
        sshCredentialId == other.sshCredentialId &&
        status == other.status &&
        trustedHostFingerprint == other.trustedHostFingerprint &&
        updatedAt == other.updatedAt &&
        username == other.username &&
        version == other.version &&
        workRoot == other.workRoot;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, checkedAt.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, host.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, port.hashCode);
    _$hash = $jc(_$hash, secretsRoot.hashCode);
    _$hash = $jc(_$hash, sshCredentialId.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, trustedHostFingerprint.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jc(_$hash, username.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jc(_$hash, workRoot.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'NodeResponse')
          ..add('checkedAt', checkedAt)
          ..add('createdAt', createdAt)
          ..add('host', host)
          ..add('id', id)
          ..add('name', name)
          ..add('port', port)
          ..add('secretsRoot', secretsRoot)
          ..add('sshCredentialId', sshCredentialId)
          ..add('status', status)
          ..add('trustedHostFingerprint', trustedHostFingerprint)
          ..add('updatedAt', updatedAt)
          ..add('username', username)
          ..add('version', version)
          ..add('workRoot', workRoot))
        .toString();
  }
}

class NodeResponseBuilder
    implements Builder<NodeResponse, NodeResponseBuilder> {
  _$NodeResponse? _$v;

  String? _checkedAt;
  String? get checkedAt => _$this._checkedAt;
  set checkedAt(String? checkedAt) => _$this._checkedAt = checkedAt;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _host;
  String? get host => _$this._host;
  set host(String? host) => _$this._host = host;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  int? _port;
  int? get port => _$this._port;
  set port(int? port) => _$this._port = port;

  String? _secretsRoot;
  String? get secretsRoot => _$this._secretsRoot;
  set secretsRoot(String? secretsRoot) => _$this._secretsRoot = secretsRoot;

  String? _sshCredentialId;
  String? get sshCredentialId => _$this._sshCredentialId;
  set sshCredentialId(String? sshCredentialId) =>
      _$this._sshCredentialId = sshCredentialId;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _trustedHostFingerprint;
  String? get trustedHostFingerprint => _$this._trustedHostFingerprint;
  set trustedHostFingerprint(String? trustedHostFingerprint) =>
      _$this._trustedHostFingerprint = trustedHostFingerprint;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  String? _username;
  String? get username => _$this._username;
  set username(String? username) => _$this._username = username;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  String? _workRoot;
  String? get workRoot => _$this._workRoot;
  set workRoot(String? workRoot) => _$this._workRoot = workRoot;

  NodeResponseBuilder() {
    NodeResponse._defaults(this);
  }

  NodeResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _checkedAt = $v.checkedAt;
      _createdAt = $v.createdAt;
      _host = $v.host;
      _id = $v.id;
      _name = $v.name;
      _port = $v.port;
      _secretsRoot = $v.secretsRoot;
      _sshCredentialId = $v.sshCredentialId;
      _status = $v.status;
      _trustedHostFingerprint = $v.trustedHostFingerprint;
      _updatedAt = $v.updatedAt;
      _username = $v.username;
      _version = $v.version;
      _workRoot = $v.workRoot;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(NodeResponse other) {
    _$v = other as _$NodeResponse;
  }

  @override
  void update(void Function(NodeResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  NodeResponse build() => _build();

  _$NodeResponse _build() {
    final _$result =
        _$v ??
        _$NodeResponse._(
          checkedAt: checkedAt,
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'NodeResponse',
            'createdAt',
          ),
          host: host,
          id: BuiltValueNullFieldError.checkNotNull(id, r'NodeResponse', 'id'),
          name: BuiltValueNullFieldError.checkNotNull(
            name,
            r'NodeResponse',
            'name',
          ),
          port: port,
          secretsRoot: secretsRoot,
          sshCredentialId: sshCredentialId,
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'NodeResponse',
            'status',
          ),
          trustedHostFingerprint: trustedHostFingerprint,
          updatedAt: BuiltValueNullFieldError.checkNotNull(
            updatedAt,
            r'NodeResponse',
            'updatedAt',
          ),
          username: username,
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'NodeResponse',
            'version',
          ),
          workRoot: workRoot,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
