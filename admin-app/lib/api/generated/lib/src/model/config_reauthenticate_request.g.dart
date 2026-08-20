// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'config_reauthenticate_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ConfigReauthenticateRequest extends ConfigReauthenticateRequest {
  @override
  final ConfigGrantAction? action;
  @override
  final String password;

  factory _$ConfigReauthenticateRequest([
    void Function(ConfigReauthenticateRequestBuilder)? updates,
  ]) => (ConfigReauthenticateRequestBuilder()..update(updates))._build();

  _$ConfigReauthenticateRequest._({this.action, required this.password})
    : super._();
  @override
  ConfigReauthenticateRequest rebuild(
    void Function(ConfigReauthenticateRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ConfigReauthenticateRequestBuilder toBuilder() =>
      ConfigReauthenticateRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ConfigReauthenticateRequest &&
        action == other.action &&
        password == other.password;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, action.hashCode);
    _$hash = $jc(_$hash, password.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ConfigReauthenticateRequest')
          ..add('action', action)
          ..add('password', password))
        .toString();
  }
}

class ConfigReauthenticateRequestBuilder
    implements
        Builder<
          ConfigReauthenticateRequest,
          ConfigReauthenticateRequestBuilder
        > {
  _$ConfigReauthenticateRequest? _$v;

  ConfigGrantAction? _action;
  ConfigGrantAction? get action => _$this._action;
  set action(ConfigGrantAction? action) => _$this._action = action;

  String? _password;
  String? get password => _$this._password;
  set password(String? password) => _$this._password = password;

  ConfigReauthenticateRequestBuilder() {
    ConfigReauthenticateRequest._defaults(this);
  }

  ConfigReauthenticateRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _action = $v.action;
      _password = $v.password;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ConfigReauthenticateRequest other) {
    _$v = other as _$ConfigReauthenticateRequest;
  }

  @override
  void update(void Function(ConfigReauthenticateRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ConfigReauthenticateRequest build() => _build();

  _$ConfigReauthenticateRequest _build() {
    final _$result =
        _$v ??
        _$ConfigReauthenticateRequest._(
          action: action,
          password: BuiltValueNullFieldError.checkNotNull(
            password,
            r'ConfigReauthenticateRequest',
            'password',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
