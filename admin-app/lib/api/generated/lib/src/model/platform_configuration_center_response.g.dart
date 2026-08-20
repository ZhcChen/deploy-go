// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'platform_configuration_center_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$PlatformConfigurationCenterResponse
    extends PlatformConfigurationCenterResponse {
  @override
  final String? checkedAt;
  @override
  final BuiltList<String> endpoints;
  @override
  final bool passwordConfigured;
  @override
  final String provider;
  @override
  final String status;
  @override
  final String updatedAt;
  @override
  final String username;
  @override
  final int version;

  factory _$PlatformConfigurationCenterResponse([
    void Function(PlatformConfigurationCenterResponseBuilder)? updates,
  ]) =>
      (PlatformConfigurationCenterResponseBuilder()..update(updates))._build();

  _$PlatformConfigurationCenterResponse._({
    this.checkedAt,
    required this.endpoints,
    required this.passwordConfigured,
    required this.provider,
    required this.status,
    required this.updatedAt,
    required this.username,
    required this.version,
  }) : super._();
  @override
  PlatformConfigurationCenterResponse rebuild(
    void Function(PlatformConfigurationCenterResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  PlatformConfigurationCenterResponseBuilder toBuilder() =>
      PlatformConfigurationCenterResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is PlatformConfigurationCenterResponse &&
        checkedAt == other.checkedAt &&
        endpoints == other.endpoints &&
        passwordConfigured == other.passwordConfigured &&
        provider == other.provider &&
        status == other.status &&
        updatedAt == other.updatedAt &&
        username == other.username &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, checkedAt.hashCode);
    _$hash = $jc(_$hash, endpoints.hashCode);
    _$hash = $jc(_$hash, passwordConfigured.hashCode);
    _$hash = $jc(_$hash, provider.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jc(_$hash, username.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'PlatformConfigurationCenterResponse')
          ..add('checkedAt', checkedAt)
          ..add('endpoints', endpoints)
          ..add('passwordConfigured', passwordConfigured)
          ..add('provider', provider)
          ..add('status', status)
          ..add('updatedAt', updatedAt)
          ..add('username', username)
          ..add('version', version))
        .toString();
  }
}

class PlatformConfigurationCenterResponseBuilder
    implements
        Builder<
          PlatformConfigurationCenterResponse,
          PlatformConfigurationCenterResponseBuilder
        > {
  _$PlatformConfigurationCenterResponse? _$v;

  String? _checkedAt;
  String? get checkedAt => _$this._checkedAt;
  set checkedAt(String? checkedAt) => _$this._checkedAt = checkedAt;

  ListBuilder<String>? _endpoints;
  ListBuilder<String> get endpoints =>
      _$this._endpoints ??= ListBuilder<String>();
  set endpoints(ListBuilder<String>? endpoints) =>
      _$this._endpoints = endpoints;

  bool? _passwordConfigured;
  bool? get passwordConfigured => _$this._passwordConfigured;
  set passwordConfigured(bool? passwordConfigured) =>
      _$this._passwordConfigured = passwordConfigured;

  String? _provider;
  String? get provider => _$this._provider;
  set provider(String? provider) => _$this._provider = provider;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  String? _username;
  String? get username => _$this._username;
  set username(String? username) => _$this._username = username;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  PlatformConfigurationCenterResponseBuilder() {
    PlatformConfigurationCenterResponse._defaults(this);
  }

  PlatformConfigurationCenterResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _checkedAt = $v.checkedAt;
      _endpoints = $v.endpoints.toBuilder();
      _passwordConfigured = $v.passwordConfigured;
      _provider = $v.provider;
      _status = $v.status;
      _updatedAt = $v.updatedAt;
      _username = $v.username;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(PlatformConfigurationCenterResponse other) {
    _$v = other as _$PlatformConfigurationCenterResponse;
  }

  @override
  void update(
    void Function(PlatformConfigurationCenterResponseBuilder)? updates,
  ) {
    if (updates != null) updates(this);
  }

  @override
  PlatformConfigurationCenterResponse build() => _build();

  _$PlatformConfigurationCenterResponse _build() {
    _$PlatformConfigurationCenterResponse _$result;
    try {
      _$result =
          _$v ??
          _$PlatformConfigurationCenterResponse._(
            checkedAt: checkedAt,
            endpoints: endpoints.build(),
            passwordConfigured: BuiltValueNullFieldError.checkNotNull(
              passwordConfigured,
              r'PlatformConfigurationCenterResponse',
              'passwordConfigured',
            ),
            provider: BuiltValueNullFieldError.checkNotNull(
              provider,
              r'PlatformConfigurationCenterResponse',
              'provider',
            ),
            status: BuiltValueNullFieldError.checkNotNull(
              status,
              r'PlatformConfigurationCenterResponse',
              'status',
            ),
            updatedAt: BuiltValueNullFieldError.checkNotNull(
              updatedAt,
              r'PlatformConfigurationCenterResponse',
              'updatedAt',
            ),
            username: BuiltValueNullFieldError.checkNotNull(
              username,
              r'PlatformConfigurationCenterResponse',
              'username',
            ),
            version: BuiltValueNullFieldError.checkNotNull(
              version,
              r'PlatformConfigurationCenterResponse',
              'version',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'endpoints';
        endpoints.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'PlatformConfigurationCenterResponse',
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
