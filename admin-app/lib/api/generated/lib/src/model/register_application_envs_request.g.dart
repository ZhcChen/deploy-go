// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'register_application_envs_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RegisterApplicationEnvsRequest extends RegisterApplicationEnvsRequest {
  @override
  final BuiltList<RegisterApplicationEnvContent> files;
  @override
  final String manifestJson;

  factory _$RegisterApplicationEnvsRequest([
    void Function(RegisterApplicationEnvsRequestBuilder)? updates,
  ]) => (RegisterApplicationEnvsRequestBuilder()..update(updates))._build();

  _$RegisterApplicationEnvsRequest._({
    required this.files,
    required this.manifestJson,
  }) : super._();
  @override
  RegisterApplicationEnvsRequest rebuild(
    void Function(RegisterApplicationEnvsRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RegisterApplicationEnvsRequestBuilder toBuilder() =>
      RegisterApplicationEnvsRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RegisterApplicationEnvsRequest &&
        files == other.files &&
        manifestJson == other.manifestJson;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, files.hashCode);
    _$hash = $jc(_$hash, manifestJson.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RegisterApplicationEnvsRequest')
          ..add('files', files)
          ..add('manifestJson', manifestJson))
        .toString();
  }
}

class RegisterApplicationEnvsRequestBuilder
    implements
        Builder<
          RegisterApplicationEnvsRequest,
          RegisterApplicationEnvsRequestBuilder
        > {
  _$RegisterApplicationEnvsRequest? _$v;

  ListBuilder<RegisterApplicationEnvContent>? _files;
  ListBuilder<RegisterApplicationEnvContent> get files =>
      _$this._files ??= ListBuilder<RegisterApplicationEnvContent>();
  set files(ListBuilder<RegisterApplicationEnvContent>? files) =>
      _$this._files = files;

  String? _manifestJson;
  String? get manifestJson => _$this._manifestJson;
  set manifestJson(String? manifestJson) => _$this._manifestJson = manifestJson;

  RegisterApplicationEnvsRequestBuilder() {
    RegisterApplicationEnvsRequest._defaults(this);
  }

  RegisterApplicationEnvsRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _files = $v.files.toBuilder();
      _manifestJson = $v.manifestJson;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RegisterApplicationEnvsRequest other) {
    _$v = other as _$RegisterApplicationEnvsRequest;
  }

  @override
  void update(void Function(RegisterApplicationEnvsRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RegisterApplicationEnvsRequest build() => _build();

  _$RegisterApplicationEnvsRequest _build() {
    _$RegisterApplicationEnvsRequest _$result;
    try {
      _$result =
          _$v ??
          _$RegisterApplicationEnvsRequest._(
            files: files.build(),
            manifestJson: BuiltValueNullFieldError.checkNotNull(
              manifestJson,
              r'RegisterApplicationEnvsRequest',
              'manifestJson',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'files';
        files.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'RegisterApplicationEnvsRequest',
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
