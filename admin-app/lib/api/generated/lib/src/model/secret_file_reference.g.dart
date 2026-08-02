// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'secret_file_reference.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SecretFileReference extends SecretFileReference {
  @override
  final String environmentKey;
  @override
  final String filePath;

  factory _$SecretFileReference([
    void Function(SecretFileReferenceBuilder)? updates,
  ]) => (SecretFileReferenceBuilder()..update(updates))._build();

  _$SecretFileReference._({
    required this.environmentKey,
    required this.filePath,
  }) : super._();
  @override
  SecretFileReference rebuild(
    void Function(SecretFileReferenceBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  SecretFileReferenceBuilder toBuilder() =>
      SecretFileReferenceBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SecretFileReference &&
        environmentKey == other.environmentKey &&
        filePath == other.filePath;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, environmentKey.hashCode);
    _$hash = $jc(_$hash, filePath.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SecretFileReference')
          ..add('environmentKey', environmentKey)
          ..add('filePath', filePath))
        .toString();
  }
}

class SecretFileReferenceBuilder
    implements Builder<SecretFileReference, SecretFileReferenceBuilder> {
  _$SecretFileReference? _$v;

  String? _environmentKey;
  String? get environmentKey => _$this._environmentKey;
  set environmentKey(String? environmentKey) =>
      _$this._environmentKey = environmentKey;

  String? _filePath;
  String? get filePath => _$this._filePath;
  set filePath(String? filePath) => _$this._filePath = filePath;

  SecretFileReferenceBuilder() {
    SecretFileReference._defaults(this);
  }

  SecretFileReferenceBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _environmentKey = $v.environmentKey;
      _filePath = $v.filePath;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SecretFileReference other) {
    _$v = other as _$SecretFileReference;
  }

  @override
  void update(void Function(SecretFileReferenceBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SecretFileReference build() => _build();

  _$SecretFileReference _build() {
    final _$result =
        _$v ??
        _$SecretFileReference._(
          environmentKey: BuiltValueNullFieldError.checkNotNull(
            environmentKey,
            r'SecretFileReference',
            'environmentKey',
          ),
          filePath: BuiltValueNullFieldError.checkNotNull(
            filePath,
            r'SecretFileReference',
            'filePath',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
