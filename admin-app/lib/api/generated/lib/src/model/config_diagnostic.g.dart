// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'config_diagnostic.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ConfigDiagnostic extends ConfigDiagnostic {
  @override
  final String code;
  @override
  final int column;
  @override
  final int line;
  @override
  final String message;
  @override
  final String path;

  factory _$ConfigDiagnostic([
    void Function(ConfigDiagnosticBuilder)? updates,
  ]) => (ConfigDiagnosticBuilder()..update(updates))._build();

  _$ConfigDiagnostic._({
    required this.code,
    required this.column,
    required this.line,
    required this.message,
    required this.path,
  }) : super._();
  @override
  ConfigDiagnostic rebuild(void Function(ConfigDiagnosticBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ConfigDiagnosticBuilder toBuilder() =>
      ConfigDiagnosticBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ConfigDiagnostic &&
        code == other.code &&
        column == other.column &&
        line == other.line &&
        message == other.message &&
        path == other.path;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, code.hashCode);
    _$hash = $jc(_$hash, column.hashCode);
    _$hash = $jc(_$hash, line.hashCode);
    _$hash = $jc(_$hash, message.hashCode);
    _$hash = $jc(_$hash, path.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ConfigDiagnostic')
          ..add('code', code)
          ..add('column', column)
          ..add('line', line)
          ..add('message', message)
          ..add('path', path))
        .toString();
  }
}

class ConfigDiagnosticBuilder
    implements Builder<ConfigDiagnostic, ConfigDiagnosticBuilder> {
  _$ConfigDiagnostic? _$v;

  String? _code;
  String? get code => _$this._code;
  set code(String? code) => _$this._code = code;

  int? _column;
  int? get column => _$this._column;
  set column(int? column) => _$this._column = column;

  int? _line;
  int? get line => _$this._line;
  set line(int? line) => _$this._line = line;

  String? _message;
  String? get message => _$this._message;
  set message(String? message) => _$this._message = message;

  String? _path;
  String? get path => _$this._path;
  set path(String? path) => _$this._path = path;

  ConfigDiagnosticBuilder() {
    ConfigDiagnostic._defaults(this);
  }

  ConfigDiagnosticBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _code = $v.code;
      _column = $v.column;
      _line = $v.line;
      _message = $v.message;
      _path = $v.path;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ConfigDiagnostic other) {
    _$v = other as _$ConfigDiagnostic;
  }

  @override
  void update(void Function(ConfigDiagnosticBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ConfigDiagnostic build() => _build();

  _$ConfigDiagnostic _build() {
    final _$result =
        _$v ??
        _$ConfigDiagnostic._(
          code: BuiltValueNullFieldError.checkNotNull(
            code,
            r'ConfigDiagnostic',
            'code',
          ),
          column: BuiltValueNullFieldError.checkNotNull(
            column,
            r'ConfigDiagnostic',
            'column',
          ),
          line: BuiltValueNullFieldError.checkNotNull(
            line,
            r'ConfigDiagnostic',
            'line',
          ),
          message: BuiltValueNullFieldError.checkNotNull(
            message,
            r'ConfigDiagnostic',
            'message',
          ),
          path: BuiltValueNullFieldError.checkNotNull(
            path,
            r'ConfigDiagnostic',
            'path',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
