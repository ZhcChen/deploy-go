// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'csrf_token_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CsrfTokenResponse extends CsrfTokenResponse {
  @override
  final String csrfToken;

  factory _$CsrfTokenResponse(
          [void Function(CsrfTokenResponseBuilder)? updates]) =>
      (CsrfTokenResponseBuilder()..update(updates))._build();

  _$CsrfTokenResponse._({required this.csrfToken}) : super._();
  @override
  CsrfTokenResponse rebuild(void Function(CsrfTokenResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CsrfTokenResponseBuilder toBuilder() =>
      CsrfTokenResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CsrfTokenResponse && csrfToken == other.csrfToken;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, csrfToken.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CsrfTokenResponse')
          ..add('csrfToken', csrfToken))
        .toString();
  }
}

class CsrfTokenResponseBuilder
    implements Builder<CsrfTokenResponse, CsrfTokenResponseBuilder> {
  _$CsrfTokenResponse? _$v;

  String? _csrfToken;
  String? get csrfToken => _$this._csrfToken;
  set csrfToken(String? csrfToken) => _$this._csrfToken = csrfToken;

  CsrfTokenResponseBuilder() {
    CsrfTokenResponse._defaults(this);
  }

  CsrfTokenResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _csrfToken = $v.csrfToken;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CsrfTokenResponse other) {
    _$v = other as _$CsrfTokenResponse;
  }

  @override
  void update(void Function(CsrfTokenResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CsrfTokenResponse build() => _build();

  _$CsrfTokenResponse _build() {
    final _$result = _$v ??
        _$CsrfTokenResponse._(
          csrfToken: BuiltValueNullFieldError.checkNotNull(
              csrfToken, r'CsrfTokenResponse', 'csrfToken'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
