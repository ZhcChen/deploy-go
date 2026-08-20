// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'config_reveal_grant_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ConfigRevealGrantResponse extends ConfigRevealGrantResponse {
  @override
  final ConfigGrantAction action;
  @override
  final String expiresAt;
  @override
  final String grantToken;

  factory _$ConfigRevealGrantResponse([
    void Function(ConfigRevealGrantResponseBuilder)? updates,
  ]) => (ConfigRevealGrantResponseBuilder()..update(updates))._build();

  _$ConfigRevealGrantResponse._({
    required this.action,
    required this.expiresAt,
    required this.grantToken,
  }) : super._();
  @override
  ConfigRevealGrantResponse rebuild(
    void Function(ConfigRevealGrantResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ConfigRevealGrantResponseBuilder toBuilder() =>
      ConfigRevealGrantResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ConfigRevealGrantResponse &&
        action == other.action &&
        expiresAt == other.expiresAt &&
        grantToken == other.grantToken;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, action.hashCode);
    _$hash = $jc(_$hash, expiresAt.hashCode);
    _$hash = $jc(_$hash, grantToken.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ConfigRevealGrantResponse')
          ..add('action', action)
          ..add('expiresAt', expiresAt)
          ..add('grantToken', grantToken))
        .toString();
  }
}

class ConfigRevealGrantResponseBuilder
    implements
        Builder<ConfigRevealGrantResponse, ConfigRevealGrantResponseBuilder> {
  _$ConfigRevealGrantResponse? _$v;

  ConfigGrantAction? _action;
  ConfigGrantAction? get action => _$this._action;
  set action(ConfigGrantAction? action) => _$this._action = action;

  String? _expiresAt;
  String? get expiresAt => _$this._expiresAt;
  set expiresAt(String? expiresAt) => _$this._expiresAt = expiresAt;

  String? _grantToken;
  String? get grantToken => _$this._grantToken;
  set grantToken(String? grantToken) => _$this._grantToken = grantToken;

  ConfigRevealGrantResponseBuilder() {
    ConfigRevealGrantResponse._defaults(this);
  }

  ConfigRevealGrantResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _action = $v.action;
      _expiresAt = $v.expiresAt;
      _grantToken = $v.grantToken;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ConfigRevealGrantResponse other) {
    _$v = other as _$ConfigRevealGrantResponse;
  }

  @override
  void update(void Function(ConfigRevealGrantResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ConfigRevealGrantResponse build() => _build();

  _$ConfigRevealGrantResponse _build() {
    final _$result =
        _$v ??
        _$ConfigRevealGrantResponse._(
          action: BuiltValueNullFieldError.checkNotNull(
            action,
            r'ConfigRevealGrantResponse',
            'action',
          ),
          expiresAt: BuiltValueNullFieldError.checkNotNull(
            expiresAt,
            r'ConfigRevealGrantResponse',
            'expiresAt',
          ),
          grantToken: BuiltValueNullFieldError.checkNotNull(
            grantToken,
            r'ConfigRevealGrantResponse',
            'grantToken',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
