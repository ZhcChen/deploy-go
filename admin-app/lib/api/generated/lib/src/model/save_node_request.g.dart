// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'save_node_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SaveNodeRequest extends SaveNodeRequest {
  @override
  final String host;
  @override
  final String name;
  @override
  final int port;
  @override
  final String secretsRoot;
  @override
  final String? sshCredentialId;
  @override
  final String username;
  @override
  final int? version;
  @override
  final String workRoot;

  factory _$SaveNodeRequest([void Function(SaveNodeRequestBuilder)? updates]) =>
      (SaveNodeRequestBuilder()..update(updates))._build();

  _$SaveNodeRequest._({
    required this.host,
    required this.name,
    required this.port,
    required this.secretsRoot,
    this.sshCredentialId,
    required this.username,
    this.version,
    required this.workRoot,
  }) : super._();
  @override
  SaveNodeRequest rebuild(void Function(SaveNodeRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SaveNodeRequestBuilder toBuilder() => SaveNodeRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SaveNodeRequest &&
        host == other.host &&
        name == other.name &&
        port == other.port &&
        secretsRoot == other.secretsRoot &&
        sshCredentialId == other.sshCredentialId &&
        username == other.username &&
        version == other.version &&
        workRoot == other.workRoot;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, host.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, port.hashCode);
    _$hash = $jc(_$hash, secretsRoot.hashCode);
    _$hash = $jc(_$hash, sshCredentialId.hashCode);
    _$hash = $jc(_$hash, username.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jc(_$hash, workRoot.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SaveNodeRequest')
          ..add('host', host)
          ..add('name', name)
          ..add('port', port)
          ..add('secretsRoot', secretsRoot)
          ..add('sshCredentialId', sshCredentialId)
          ..add('username', username)
          ..add('version', version)
          ..add('workRoot', workRoot))
        .toString();
  }
}

class SaveNodeRequestBuilder
    implements Builder<SaveNodeRequest, SaveNodeRequestBuilder> {
  _$SaveNodeRequest? _$v;

  String? _host;
  String? get host => _$this._host;
  set host(String? host) => _$this._host = host;

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

  String? _username;
  String? get username => _$this._username;
  set username(String? username) => _$this._username = username;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  String? _workRoot;
  String? get workRoot => _$this._workRoot;
  set workRoot(String? workRoot) => _$this._workRoot = workRoot;

  SaveNodeRequestBuilder() {
    SaveNodeRequest._defaults(this);
  }

  SaveNodeRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _host = $v.host;
      _name = $v.name;
      _port = $v.port;
      _secretsRoot = $v.secretsRoot;
      _sshCredentialId = $v.sshCredentialId;
      _username = $v.username;
      _version = $v.version;
      _workRoot = $v.workRoot;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SaveNodeRequest other) {
    _$v = other as _$SaveNodeRequest;
  }

  @override
  void update(void Function(SaveNodeRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SaveNodeRequest build() => _build();

  _$SaveNodeRequest _build() {
    final _$result =
        _$v ??
        _$SaveNodeRequest._(
          host: BuiltValueNullFieldError.checkNotNull(
            host,
            r'SaveNodeRequest',
            'host',
          ),
          name: BuiltValueNullFieldError.checkNotNull(
            name,
            r'SaveNodeRequest',
            'name',
          ),
          port: BuiltValueNullFieldError.checkNotNull(
            port,
            r'SaveNodeRequest',
            'port',
          ),
          secretsRoot: BuiltValueNullFieldError.checkNotNull(
            secretsRoot,
            r'SaveNodeRequest',
            'secretsRoot',
          ),
          sshCredentialId: sshCredentialId,
          username: BuiltValueNullFieldError.checkNotNull(
            username,
            r'SaveNodeRequest',
            'username',
          ),
          version: version,
          workRoot: BuiltValueNullFieldError.checkNotNull(
            workRoot,
            r'SaveNodeRequest',
            'workRoot',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
