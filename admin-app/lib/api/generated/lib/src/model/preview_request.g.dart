// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'preview_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$PreviewRequest extends PreviewRequest {
  @override
  final JsonObject? parameters;

  factory _$PreviewRequest([void Function(PreviewRequestBuilder)? updates]) =>
      (PreviewRequestBuilder()..update(updates))._build();

  _$PreviewRequest._({this.parameters}) : super._();
  @override
  PreviewRequest rebuild(void Function(PreviewRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  PreviewRequestBuilder toBuilder() => PreviewRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is PreviewRequest && parameters == other.parameters;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, parameters.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'PreviewRequest')
          ..add('parameters', parameters))
        .toString();
  }
}

class PreviewRequestBuilder
    implements Builder<PreviewRequest, PreviewRequestBuilder> {
  _$PreviewRequest? _$v;

  JsonObject? _parameters;
  JsonObject? get parameters => _$this._parameters;
  set parameters(JsonObject? parameters) => _$this._parameters = parameters;

  PreviewRequestBuilder() {
    PreviewRequest._defaults(this);
  }

  PreviewRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _parameters = $v.parameters;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(PreviewRequest other) {
    _$v = other as _$PreviewRequest;
  }

  @override
  void update(void Function(PreviewRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  PreviewRequest build() => _build();

  _$PreviewRequest _build() {
    final _$result = _$v ??
        _$PreviewRequest._(
          parameters: parameters,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
