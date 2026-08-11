// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'register_admin_application_env_content.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RegisterAdminApplicationEnvContent
    extends RegisterAdminApplicationEnvContent {
  @override
  final String content;
  @override
  final String fileName;
  @override
  final String format;
  @override
  final String module;

  factory _$RegisterAdminApplicationEnvContent([
    void Function(RegisterAdminApplicationEnvContentBuilder)? updates,
  ]) => (RegisterAdminApplicationEnvContentBuilder()..update(updates))._build();

  _$RegisterAdminApplicationEnvContent._({
    required this.content,
    required this.fileName,
    required this.format,
    required this.module,
  }) : super._();
  @override
  RegisterAdminApplicationEnvContent rebuild(
    void Function(RegisterAdminApplicationEnvContentBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RegisterAdminApplicationEnvContentBuilder toBuilder() =>
      RegisterAdminApplicationEnvContentBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RegisterAdminApplicationEnvContent &&
        content == other.content &&
        fileName == other.fileName &&
        format == other.format &&
        module == other.module;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, content.hashCode);
    _$hash = $jc(_$hash, fileName.hashCode);
    _$hash = $jc(_$hash, format.hashCode);
    _$hash = $jc(_$hash, module.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RegisterAdminApplicationEnvContent')
          ..add('content', content)
          ..add('fileName', fileName)
          ..add('format', format)
          ..add('module', module))
        .toString();
  }
}

class RegisterAdminApplicationEnvContentBuilder
    implements
        Builder<
          RegisterAdminApplicationEnvContent,
          RegisterAdminApplicationEnvContentBuilder
        > {
  _$RegisterAdminApplicationEnvContent? _$v;

  String? _content;
  String? get content => _$this._content;
  set content(String? content) => _$this._content = content;

  String? _fileName;
  String? get fileName => _$this._fileName;
  set fileName(String? fileName) => _$this._fileName = fileName;

  String? _format;
  String? get format => _$this._format;
  set format(String? format) => _$this._format = format;

  String? _module;
  String? get module => _$this._module;
  set module(String? module) => _$this._module = module;

  RegisterAdminApplicationEnvContentBuilder() {
    RegisterAdminApplicationEnvContent._defaults(this);
  }

  RegisterAdminApplicationEnvContentBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _content = $v.content;
      _fileName = $v.fileName;
      _format = $v.format;
      _module = $v.module;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RegisterAdminApplicationEnvContent other) {
    _$v = other as _$RegisterAdminApplicationEnvContent;
  }

  @override
  void update(
    void Function(RegisterAdminApplicationEnvContentBuilder)? updates,
  ) {
    if (updates != null) updates(this);
  }

  @override
  RegisterAdminApplicationEnvContent build() => _build();

  _$RegisterAdminApplicationEnvContent _build() {
    final _$result =
        _$v ??
        _$RegisterAdminApplicationEnvContent._(
          content: BuiltValueNullFieldError.checkNotNull(
            content,
            r'RegisterAdminApplicationEnvContent',
            'content',
          ),
          fileName: BuiltValueNullFieldError.checkNotNull(
            fileName,
            r'RegisterAdminApplicationEnvContent',
            'fileName',
          ),
          format: BuiltValueNullFieldError.checkNotNull(
            format,
            r'RegisterAdminApplicationEnvContent',
            'format',
          ),
          module: BuiltValueNullFieldError.checkNotNull(
            module,
            r'RegisterAdminApplicationEnvContent',
            'module',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
