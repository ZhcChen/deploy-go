// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'generate_secret_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$GenerateSecretRequest extends GenerateSecretRequest {
  @override
  final int? bytes;
  @override
  final int expectedVersion;
  @override
  final String key;

  factory _$GenerateSecretRequest([
    void Function(GenerateSecretRequestBuilder)? updates,
  ]) => (GenerateSecretRequestBuilder()..update(updates))._build();

  _$GenerateSecretRequest._({
    this.bytes,
    required this.expectedVersion,
    required this.key,
  }) : super._();
  @override
  GenerateSecretRequest rebuild(
    void Function(GenerateSecretRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  GenerateSecretRequestBuilder toBuilder() =>
      GenerateSecretRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is GenerateSecretRequest &&
        bytes == other.bytes &&
        expectedVersion == other.expectedVersion &&
        key == other.key;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, bytes.hashCode);
    _$hash = $jc(_$hash, expectedVersion.hashCode);
    _$hash = $jc(_$hash, key.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'GenerateSecretRequest')
          ..add('bytes', bytes)
          ..add('expectedVersion', expectedVersion)
          ..add('key', key))
        .toString();
  }
}

class GenerateSecretRequestBuilder
    implements Builder<GenerateSecretRequest, GenerateSecretRequestBuilder> {
  _$GenerateSecretRequest? _$v;

  int? _bytes;
  int? get bytes => _$this._bytes;
  set bytes(int? bytes) => _$this._bytes = bytes;

  int? _expectedVersion;
  int? get expectedVersion => _$this._expectedVersion;
  set expectedVersion(int? expectedVersion) =>
      _$this._expectedVersion = expectedVersion;

  String? _key;
  String? get key => _$this._key;
  set key(String? key) => _$this._key = key;

  GenerateSecretRequestBuilder() {
    GenerateSecretRequest._defaults(this);
  }

  GenerateSecretRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _bytes = $v.bytes;
      _expectedVersion = $v.expectedVersion;
      _key = $v.key;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(GenerateSecretRequest other) {
    _$v = other as _$GenerateSecretRequest;
  }

  @override
  void update(void Function(GenerateSecretRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  GenerateSecretRequest build() => _build();

  _$GenerateSecretRequest _build() {
    final _$result =
        _$v ??
        _$GenerateSecretRequest._(
          bytes: bytes,
          expectedVersion: BuiltValueNullFieldError.checkNotNull(
            expectedVersion,
            r'GenerateSecretRequest',
            'expectedVersion',
          ),
          key: BuiltValueNullFieldError.checkNotNull(
            key,
            r'GenerateSecretRequest',
            'key',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
