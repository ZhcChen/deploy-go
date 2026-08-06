// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'env_reauthenticate_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$EnvReauthenticateRequest extends EnvReauthenticateRequest {
  @override
  final EnvGrantAction action;
  @override
  final String password;

  factory _$EnvReauthenticateRequest([
    void Function(EnvReauthenticateRequestBuilder)? updates,
  ]) => (EnvReauthenticateRequestBuilder()..update(updates))._build();

  _$EnvReauthenticateRequest._({required this.action, required this.password})
    : super._();
  @override
  EnvReauthenticateRequest rebuild(
    void Function(EnvReauthenticateRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  EnvReauthenticateRequestBuilder toBuilder() =>
      EnvReauthenticateRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is EnvReauthenticateRequest &&
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
    return (newBuiltValueToStringHelper(r'EnvReauthenticateRequest')
          ..add('action', action)
          ..add('password', password))
        .toString();
  }
}

class EnvReauthenticateRequestBuilder
    implements
        Builder<EnvReauthenticateRequest, EnvReauthenticateRequestBuilder> {
  _$EnvReauthenticateRequest? _$v;

  EnvGrantAction? _action;
  EnvGrantAction? get action => _$this._action;
  set action(EnvGrantAction? action) => _$this._action = action;

  String? _password;
  String? get password => _$this._password;
  set password(String? password) => _$this._password = password;

  EnvReauthenticateRequestBuilder() {
    EnvReauthenticateRequest._defaults(this);
  }

  EnvReauthenticateRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _action = $v.action;
      _password = $v.password;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(EnvReauthenticateRequest other) {
    _$v = other as _$EnvReauthenticateRequest;
  }

  @override
  void update(void Function(EnvReauthenticateRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  EnvReauthenticateRequest build() => _build();

  _$EnvReauthenticateRequest _build() {
    final _$result =
        _$v ??
        _$EnvReauthenticateRequest._(
          action: BuiltValueNullFieldError.checkNotNull(
            action,
            r'EnvReauthenticateRequest',
            'action',
          ),
          password: BuiltValueNullFieldError.checkNotNull(
            password,
            r'EnvReauthenticateRequest',
            'password',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
