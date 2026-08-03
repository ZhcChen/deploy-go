// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'refresh_token_pair_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RefreshTokenPairResponse extends RefreshTokenPairResponse {
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
  @override
  final String rotationId;

  factory _$RefreshTokenPairResponse([
    void Function(RefreshTokenPairResponseBuilder)? updates,
  ]) => (RefreshTokenPairResponseBuilder()..update(updates))._build();

  _$RefreshTokenPairResponse._({
    required this.accessExpiresAt,
    required this.accessToken,
    required this.agentId,
    required this.refreshExpiresAt,
    required this.refreshToken,
    required this.rotationId,
  }) : super._();
  @override
  RefreshTokenPairResponse rebuild(
    void Function(RefreshTokenPairResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RefreshTokenPairResponseBuilder toBuilder() =>
      RefreshTokenPairResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RefreshTokenPairResponse &&
        accessExpiresAt == other.accessExpiresAt &&
        accessToken == other.accessToken &&
        agentId == other.agentId &&
        refreshExpiresAt == other.refreshExpiresAt &&
        refreshToken == other.refreshToken &&
        rotationId == other.rotationId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, accessExpiresAt.hashCode);
    _$hash = $jc(_$hash, accessToken.hashCode);
    _$hash = $jc(_$hash, agentId.hashCode);
    _$hash = $jc(_$hash, refreshExpiresAt.hashCode);
    _$hash = $jc(_$hash, refreshToken.hashCode);
    _$hash = $jc(_$hash, rotationId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RefreshTokenPairResponse')
          ..add('accessExpiresAt', accessExpiresAt)
          ..add('accessToken', accessToken)
          ..add('agentId', agentId)
          ..add('refreshExpiresAt', refreshExpiresAt)
          ..add('refreshToken', refreshToken)
          ..add('rotationId', rotationId))
        .toString();
  }
}

class RefreshTokenPairResponseBuilder
    implements
        Builder<RefreshTokenPairResponse, RefreshTokenPairResponseBuilder> {
  _$RefreshTokenPairResponse? _$v;

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

  String? _rotationId;
  String? get rotationId => _$this._rotationId;
  set rotationId(String? rotationId) => _$this._rotationId = rotationId;

  RefreshTokenPairResponseBuilder() {
    RefreshTokenPairResponse._defaults(this);
  }

  RefreshTokenPairResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _accessExpiresAt = $v.accessExpiresAt;
      _accessToken = $v.accessToken;
      _agentId = $v.agentId;
      _refreshExpiresAt = $v.refreshExpiresAt;
      _refreshToken = $v.refreshToken;
      _rotationId = $v.rotationId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RefreshTokenPairResponse other) {
    _$v = other as _$RefreshTokenPairResponse;
  }

  @override
  void update(void Function(RefreshTokenPairResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RefreshTokenPairResponse build() => _build();

  _$RefreshTokenPairResponse _build() {
    final _$result =
        _$v ??
        _$RefreshTokenPairResponse._(
          accessExpiresAt: BuiltValueNullFieldError.checkNotNull(
            accessExpiresAt,
            r'RefreshTokenPairResponse',
            'accessExpiresAt',
          ),
          accessToken: BuiltValueNullFieldError.checkNotNull(
            accessToken,
            r'RefreshTokenPairResponse',
            'accessToken',
          ),
          agentId: BuiltValueNullFieldError.checkNotNull(
            agentId,
            r'RefreshTokenPairResponse',
            'agentId',
          ),
          refreshExpiresAt: BuiltValueNullFieldError.checkNotNull(
            refreshExpiresAt,
            r'RefreshTokenPairResponse',
            'refreshExpiresAt',
          ),
          refreshToken: BuiltValueNullFieldError.checkNotNull(
            refreshToken,
            r'RefreshTokenPairResponse',
            'refreshToken',
          ),
          rotationId: BuiltValueNullFieldError.checkNotNull(
            rotationId,
            r'RefreshTokenPairResponse',
            'rotationId',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
