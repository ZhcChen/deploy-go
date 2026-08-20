// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'validate_application_config_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ValidateApplicationConfigRequest
    extends ValidateApplicationConfigRequest {
  @override
  final String? content;

  factory _$ValidateApplicationConfigRequest([
    void Function(ValidateApplicationConfigRequestBuilder)? updates,
  ]) => (ValidateApplicationConfigRequestBuilder()..update(updates))._build();

  _$ValidateApplicationConfigRequest._({this.content}) : super._();
  @override
  ValidateApplicationConfigRequest rebuild(
    void Function(ValidateApplicationConfigRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ValidateApplicationConfigRequestBuilder toBuilder() =>
      ValidateApplicationConfigRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ValidateApplicationConfigRequest &&
        content == other.content;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, content.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'ValidateApplicationConfigRequest',
    )..add('content', content)).toString();
  }
}

class ValidateApplicationConfigRequestBuilder
    implements
        Builder<
          ValidateApplicationConfigRequest,
          ValidateApplicationConfigRequestBuilder
        > {
  _$ValidateApplicationConfigRequest? _$v;

  String? _content;
  String? get content => _$this._content;
  set content(String? content) => _$this._content = content;

  ValidateApplicationConfigRequestBuilder() {
    ValidateApplicationConfigRequest._defaults(this);
  }

  ValidateApplicationConfigRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _content = $v.content;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ValidateApplicationConfigRequest other) {
    _$v = other as _$ValidateApplicationConfigRequest;
  }

  @override
  void update(void Function(ValidateApplicationConfigRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ValidateApplicationConfigRequest build() => _build();

  _$ValidateApplicationConfigRequest _build() {
    final _$result =
        _$v ?? _$ValidateApplicationConfigRequest._(content: content);
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
