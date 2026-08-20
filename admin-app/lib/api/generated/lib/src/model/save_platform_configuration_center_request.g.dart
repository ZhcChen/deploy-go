// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'save_platform_configuration_center_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SavePlatformConfigurationCenterRequest
    extends SavePlatformConfigurationCenterRequest {
  @override
  final BuiltList<String> endpoints;
  @override
  final String? password;
  @override
  final String username;
  @override
  final int version;

  factory _$SavePlatformConfigurationCenterRequest([
    void Function(SavePlatformConfigurationCenterRequestBuilder)? updates,
  ]) => (SavePlatformConfigurationCenterRequestBuilder()..update(updates))
      ._build();

  _$SavePlatformConfigurationCenterRequest._({
    required this.endpoints,
    this.password,
    required this.username,
    required this.version,
  }) : super._();
  @override
  SavePlatformConfigurationCenterRequest rebuild(
    void Function(SavePlatformConfigurationCenterRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  SavePlatformConfigurationCenterRequestBuilder toBuilder() =>
      SavePlatformConfigurationCenterRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SavePlatformConfigurationCenterRequest &&
        endpoints == other.endpoints &&
        password == other.password &&
        username == other.username &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, endpoints.hashCode);
    _$hash = $jc(_$hash, password.hashCode);
    _$hash = $jc(_$hash, username.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
            r'SavePlatformConfigurationCenterRequest',
          )
          ..add('endpoints', endpoints)
          ..add('password', password)
          ..add('username', username)
          ..add('version', version))
        .toString();
  }
}

class SavePlatformConfigurationCenterRequestBuilder
    implements
        Builder<
          SavePlatformConfigurationCenterRequest,
          SavePlatformConfigurationCenterRequestBuilder
        > {
  _$SavePlatformConfigurationCenterRequest? _$v;

  ListBuilder<String>? _endpoints;
  ListBuilder<String> get endpoints =>
      _$this._endpoints ??= ListBuilder<String>();
  set endpoints(ListBuilder<String>? endpoints) =>
      _$this._endpoints = endpoints;

  String? _password;
  String? get password => _$this._password;
  set password(String? password) => _$this._password = password;

  String? _username;
  String? get username => _$this._username;
  set username(String? username) => _$this._username = username;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  SavePlatformConfigurationCenterRequestBuilder() {
    SavePlatformConfigurationCenterRequest._defaults(this);
  }

  SavePlatformConfigurationCenterRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _endpoints = $v.endpoints.toBuilder();
      _password = $v.password;
      _username = $v.username;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SavePlatformConfigurationCenterRequest other) {
    _$v = other as _$SavePlatformConfigurationCenterRequest;
  }

  @override
  void update(
    void Function(SavePlatformConfigurationCenterRequestBuilder)? updates,
  ) {
    if (updates != null) updates(this);
  }

  @override
  SavePlatformConfigurationCenterRequest build() => _build();

  _$SavePlatformConfigurationCenterRequest _build() {
    _$SavePlatformConfigurationCenterRequest _$result;
    try {
      _$result =
          _$v ??
          _$SavePlatformConfigurationCenterRequest._(
            endpoints: endpoints.build(),
            password: password,
            username: BuiltValueNullFieldError.checkNotNull(
              username,
              r'SavePlatformConfigurationCenterRequest',
              'username',
            ),
            version: BuiltValueNullFieldError.checkNotNull(
              version,
              r'SavePlatformConfigurationCenterRequest',
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
          r'SavePlatformConfigurationCenterRequest',
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
