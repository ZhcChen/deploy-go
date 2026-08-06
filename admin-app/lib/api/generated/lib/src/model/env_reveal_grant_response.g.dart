// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'env_reveal_grant_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$EnvRevealGrantResponse extends EnvRevealGrantResponse {
  @override
  final EnvGrantAction action;
  @override
  final String expiresAt;
  @override
  final String grantToken;

  factory _$EnvRevealGrantResponse([
    void Function(EnvRevealGrantResponseBuilder)? updates,
  ]) => (EnvRevealGrantResponseBuilder()..update(updates))._build();

  _$EnvRevealGrantResponse._({
    required this.action,
    required this.expiresAt,
    required this.grantToken,
  }) : super._();
  @override
  EnvRevealGrantResponse rebuild(
    void Function(EnvRevealGrantResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  EnvRevealGrantResponseBuilder toBuilder() =>
      EnvRevealGrantResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is EnvRevealGrantResponse &&
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
    return (newBuiltValueToStringHelper(r'EnvRevealGrantResponse')
          ..add('action', action)
          ..add('expiresAt', expiresAt)
          ..add('grantToken', grantToken))
        .toString();
  }
}

class EnvRevealGrantResponseBuilder
    implements Builder<EnvRevealGrantResponse, EnvRevealGrantResponseBuilder> {
  _$EnvRevealGrantResponse? _$v;

  EnvGrantAction? _action;
  EnvGrantAction? get action => _$this._action;
  set action(EnvGrantAction? action) => _$this._action = action;

  String? _expiresAt;
  String? get expiresAt => _$this._expiresAt;
  set expiresAt(String? expiresAt) => _$this._expiresAt = expiresAt;

  String? _grantToken;
  String? get grantToken => _$this._grantToken;
  set grantToken(String? grantToken) => _$this._grantToken = grantToken;

  EnvRevealGrantResponseBuilder() {
    EnvRevealGrantResponse._defaults(this);
  }

  EnvRevealGrantResponseBuilder get _$this {
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
  void replace(EnvRevealGrantResponse other) {
    _$v = other as _$EnvRevealGrantResponse;
  }

  @override
  void update(void Function(EnvRevealGrantResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  EnvRevealGrantResponse build() => _build();

  _$EnvRevealGrantResponse _build() {
    final _$result =
        _$v ??
        _$EnvRevealGrantResponse._(
          action: BuiltValueNullFieldError.checkNotNull(
            action,
            r'EnvRevealGrantResponse',
            'action',
          ),
          expiresAt: BuiltValueNullFieldError.checkNotNull(
            expiresAt,
            r'EnvRevealGrantResponse',
            'expiresAt',
          ),
          grantToken: BuiltValueNullFieldError.checkNotNull(
            grantToken,
            r'EnvRevealGrantResponse',
            'grantToken',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
