// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'token_pair_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$TokenPairResponse extends TokenPairResponse {
  @override
  final String accessExpiresAt;
  @override
  final String accessToken;
  @override
  final String agentId;
  @override
  final String refreshExpiresAt;
  @override
  final String refreshToken;

  factory _$TokenPairResponse([
    void Function(TokenPairResponseBuilder)? updates,
  ]) => (TokenPairResponseBuilder()..update(updates))._build();

  _$TokenPairResponse._({
    required this.accessExpiresAt,
    required this.accessToken,
    required this.agentId,
    required this.refreshExpiresAt,
    required this.refreshToken,
  }) : super._();
  @override
  TokenPairResponse rebuild(void Function(TokenPairResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  TokenPairResponseBuilder toBuilder() =>
      TokenPairResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is TokenPairResponse &&
        accessExpiresAt == other.accessExpiresAt &&
        accessToken == other.accessToken &&
        agentId == other.agentId &&
        refreshExpiresAt == other.refreshExpiresAt &&
        refreshToken == other.refreshToken;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, accessExpiresAt.hashCode);
    _$hash = $jc(_$hash, accessToken.hashCode);
    _$hash = $jc(_$hash, agentId.hashCode);
    _$hash = $jc(_$hash, refreshExpiresAt.hashCode);
    _$hash = $jc(_$hash, refreshToken.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'TokenPairResponse')
          ..add('accessExpiresAt', accessExpiresAt)
          ..add('accessToken', accessToken)
          ..add('agentId', agentId)
          ..add('refreshExpiresAt', refreshExpiresAt)
          ..add('refreshToken', refreshToken))
        .toString();
  }
}

class TokenPairResponseBuilder
    implements Builder<TokenPairResponse, TokenPairResponseBuilder> {
  _$TokenPairResponse? _$v;

  String? _accessExpiresAt;
  String? get accessExpiresAt => _$this._accessExpiresAt;
  set accessExpiresAt(String? accessExpiresAt) =>
      _$this._accessExpiresAt = accessExpiresAt;

  String? _accessToken;
  String? get accessToken => _$this._accessToken;
  set accessToken(String? accessToken) => _$this._accessToken = accessToken;

  String? _agentId;
  String? get agentId => _$this._agentId;
  set agentId(String? agentId) => _$this._agentId = agentId;

  String? _refreshExpiresAt;
  String? get refreshExpiresAt => _$this._refreshExpiresAt;
  set refreshExpiresAt(String? refreshExpiresAt) =>
      _$this._refreshExpiresAt = refreshExpiresAt;

  String? _refreshToken;
  String? get refreshToken => _$this._refreshToken;
  set refreshToken(String? refreshToken) => _$this._refreshToken = refreshToken;

  TokenPairResponseBuilder() {
    TokenPairResponse._defaults(this);
  }

  TokenPairResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _accessExpiresAt = $v.accessExpiresAt;
      _accessToken = $v.accessToken;
      _agentId = $v.agentId;
      _refreshExpiresAt = $v.refreshExpiresAt;
      _refreshToken = $v.refreshToken;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(TokenPairResponse other) {
    _$v = other as _$TokenPairResponse;
  }

  @override
  void update(void Function(TokenPairResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  TokenPairResponse build() => _build();

  _$TokenPairResponse _build() {
    final _$result =
        _$v ??
        _$TokenPairResponse._(
          accessExpiresAt: BuiltValueNullFieldError.checkNotNull(
            accessExpiresAt,
            r'TokenPairResponse',
            'accessExpiresAt',
          ),
          accessToken: BuiltValueNullFieldError.checkNotNull(
            accessToken,
            r'TokenPairResponse',
            'accessToken',
          ),
          agentId: BuiltValueNullFieldError.checkNotNull(
            agentId,
            r'TokenPairResponse',
            'agentId',
          ),
          refreshExpiresAt: BuiltValueNullFieldError.checkNotNull(
            refreshExpiresAt,
            r'TokenPairResponse',
            'refreshExpiresAt',
          ),
          refreshToken: BuiltValueNullFieldError.checkNotNull(
            refreshToken,
            r'TokenPairResponse',
            'refreshToken',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
