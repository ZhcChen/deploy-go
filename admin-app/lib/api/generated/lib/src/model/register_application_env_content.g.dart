// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'register_application_env_content.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RegisterApplicationEnvContent extends RegisterApplicationEnvContent {
  @override
  final String contentBase64;
  @override
  final String fileName;

  factory _$RegisterApplicationEnvContent([
    void Function(RegisterApplicationEnvContentBuilder)? updates,
  ]) => (RegisterApplicationEnvContentBuilder()..update(updates))._build();

  _$RegisterApplicationEnvContent._({
    required this.contentBase64,
    required this.fileName,
  }) : super._();
  @override
  RegisterApplicationEnvContent rebuild(
    void Function(RegisterApplicationEnvContentBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RegisterApplicationEnvContentBuilder toBuilder() =>
      RegisterApplicationEnvContentBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RegisterApplicationEnvContent &&
        contentBase64 == other.contentBase64 &&
        fileName == other.fileName;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, contentBase64.hashCode);
    _$hash = $jc(_$hash, fileName.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RegisterApplicationEnvContent')
          ..add('contentBase64', contentBase64)
          ..add('fileName', fileName))
        .toString();
  }
}

class RegisterApplicationEnvContentBuilder
    implements
        Builder<
          RegisterApplicationEnvContent,
          RegisterApplicationEnvContentBuilder
        > {
  _$RegisterApplicationEnvContent? _$v;

  String? _contentBase64;
  String? get contentBase64 => _$this._contentBase64;
  set contentBase64(String? contentBase64) =>
      _$this._contentBase64 = contentBase64;

  String? _fileName;
  String? get fileName => _$this._fileName;
  set fileName(String? fileName) => _$this._fileName = fileName;

  RegisterApplicationEnvContentBuilder() {
    RegisterApplicationEnvContent._defaults(this);
  }

  RegisterApplicationEnvContentBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _contentBase64 = $v.contentBase64;
      _fileName = $v.fileName;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RegisterApplicationEnvContent other) {
    _$v = other as _$RegisterApplicationEnvContent;
  }

  @override
  void update(void Function(RegisterApplicationEnvContentBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RegisterApplicationEnvContent build() => _build();

  _$RegisterApplicationEnvContent _build() {
    final _$result =
        _$v ??
        _$RegisterApplicationEnvContent._(
          contentBase64: BuiltValueNullFieldError.checkNotNull(
            contentBase64,
            r'RegisterApplicationEnvContent',
            'contentBase64',
          ),
          fileName: BuiltValueNullFieldError.checkNotNull(
            fileName,
            r'RegisterApplicationEnvContent',
            'fileName',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
