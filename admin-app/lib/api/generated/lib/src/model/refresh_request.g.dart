// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'refresh_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RefreshRequest extends RefreshRequest {
  @override
  final String refreshToken;
  @override
  final String rotationId;

  factory _$RefreshRequest([void Function(RefreshRequestBuilder)? updates]) =>
      (RefreshRequestBuilder()..update(updates))._build();

  _$RefreshRequest._({required this.refreshToken, required this.rotationId})
    : super._();
  @override
  RefreshRequest rebuild(void Function(RefreshRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  RefreshRequestBuilder toBuilder() => RefreshRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RefreshRequest &&
        refreshToken == other.refreshToken &&
        rotationId == other.rotationId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, refreshToken.hashCode);
    _$hash = $jc(_$hash, rotationId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RefreshRequest')
          ..add('refreshToken', refreshToken)
          ..add('rotationId', rotationId))
        .toString();
  }
}

class RefreshRequestBuilder
    implements Builder<RefreshRequest, RefreshRequestBuilder> {
  _$RefreshRequest? _$v;

  String? _refreshToken;
  String? get refreshToken => _$this._refreshToken;
  set refreshToken(String? refreshToken) => _$this._refreshToken = refreshToken;

  String? _rotationId;
  String? get rotationId => _$this._rotationId;
  set rotationId(String? rotationId) => _$this._rotationId = rotationId;

  RefreshRequestBuilder() {
    RefreshRequest._defaults(this);
  }

  RefreshRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _refreshToken = $v.refreshToken;
      _rotationId = $v.rotationId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RefreshRequest other) {
    _$v = other as _$RefreshRequest;
  }

  @override
  void update(void Function(RefreshRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RefreshRequest build() => _build();

  _$RefreshRequest _build() {
    final _$result =
        _$v ??
        _$RefreshRequest._(
          refreshToken: BuiltValueNullFieldError.checkNotNull(
            refreshToken,
            r'RefreshRequest',
            'refreshToken',
          ),
          rotationId: BuiltValueNullFieldError.checkNotNull(
            rotationId,
            r'RefreshRequest',
            'rotationId',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
