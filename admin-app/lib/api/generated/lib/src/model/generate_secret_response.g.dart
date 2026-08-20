// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'generate_secret_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$GenerateSecretResponse extends GenerateSecretResponse {
  @override
  final ApplicationConfigFileResponse file;
  @override
  final String key;
  @override
  final String secret;

  factory _$GenerateSecretResponse([
    void Function(GenerateSecretResponseBuilder)? updates,
  ]) => (GenerateSecretResponseBuilder()..update(updates))._build();

  _$GenerateSecretResponse._({
    required this.file,
    required this.key,
    required this.secret,
  }) : super._();
  @override
  GenerateSecretResponse rebuild(
    void Function(GenerateSecretResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  GenerateSecretResponseBuilder toBuilder() =>
      GenerateSecretResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is GenerateSecretResponse &&
        file == other.file &&
        key == other.key &&
        secret == other.secret;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, file.hashCode);
    _$hash = $jc(_$hash, key.hashCode);
    _$hash = $jc(_$hash, secret.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'GenerateSecretResponse')
          ..add('file', file)
          ..add('key', key)
          ..add('secret', secret))
        .toString();
  }
}

class GenerateSecretResponseBuilder
    implements Builder<GenerateSecretResponse, GenerateSecretResponseBuilder> {
  _$GenerateSecretResponse? _$v;

  ApplicationConfigFileResponseBuilder? _file;
  ApplicationConfigFileResponseBuilder get file =>
      _$this._file ??= ApplicationConfigFileResponseBuilder();
  set file(ApplicationConfigFileResponseBuilder? file) => _$this._file = file;

  String? _key;
  String? get key => _$this._key;
  set key(String? key) => _$this._key = key;

  String? _secret;
  String? get secret => _$this._secret;
  set secret(String? secret) => _$this._secret = secret;

  GenerateSecretResponseBuilder() {
    GenerateSecretResponse._defaults(this);
  }

  GenerateSecretResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _file = $v.file.toBuilder();
      _key = $v.key;
      _secret = $v.secret;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(GenerateSecretResponse other) {
    _$v = other as _$GenerateSecretResponse;
  }

  @override
  void update(void Function(GenerateSecretResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  GenerateSecretResponse build() => _build();

  _$GenerateSecretResponse _build() {
    _$GenerateSecretResponse _$result;
    try {
      _$result =
          _$v ??
          _$GenerateSecretResponse._(
            file: file.build(),
            key: BuiltValueNullFieldError.checkNotNull(
              key,
              r'GenerateSecretResponse',
              'key',
            ),
            secret: BuiltValueNullFieldError.checkNotNull(
              secret,
              r'GenerateSecretResponse',
              'secret',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'file';
        file.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'GenerateSecretResponse',
          _$failedField,
          e.toString(),
        );
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
